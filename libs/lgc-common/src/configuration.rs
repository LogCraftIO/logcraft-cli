// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::{self, HashMap},
    fs, path,
    sync::Arc,
};

use anyhow::{bail, Context};
use lgc_policies::Policy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::state::backends::StateBackend;

pub const LGC_CONFIG_PATH: &str = "lgc.toml";
pub const LGC_RULES_DIR: &str = "rules";
pub const LGC_POLICIES_DIR: &str = "policies";
pub const LGC_BASE_DIR: &str = "/opt/logcraft-cli";

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ProjectConfiguration {
    pub core: CoreConfiguration,
    #[serde(default)]
    pub state: Option<StateBackend>,
    pub services: collections::BTreeMap<String, Service>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CoreConfiguration {
    pub base_dir: Option<String>,
    pub workspace: String,
}

impl Default for CoreConfiguration {
    fn default() -> Self {
        Self {
            base_dir: Some(String::from(LGC_BASE_DIR)),
            workspace: String::from("rules"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DetectionContext {
    // Tuple of (service_name, serialized configuration)
    pub services: Vec<(String, Vec<u8>)>,
    // List of related detections
    pub detections: HashMap<String, Vec<u8>>,
}

impl ProjectConfiguration {
    pub fn save_config(&self, path: Option<&str>) -> anyhow::Result<()> {
        // Serialize the config to a TOML string
        let toml_string = toml::to_string(&self)
            .with_context(|| format!("failed to serialize config to TOML for {:?}.", path))?;

        // Write the TOML string directly to the file
        std::fs::write(path.unwrap_or(LGC_CONFIG_PATH), toml_string)
            .with_context(|| format!("failed to write TOML to file at {:?}.", path))?;

        Ok(())
    }

    pub fn service_ids(&self) -> Vec<&str> {
        self.services.keys().map(|k| k.as_str()).collect()
    }

    pub fn remove_service(&mut self, name: &String) {
        if self.services.remove_entry(name).is_none() {
            tracing::warn!("service `{}` not found,", name);
        }
    }

    /// Retrieve all detections based on an identifier
    pub fn load_detections(
        &self,
        identifier: Option<String>,
    ) -> anyhow::Result<HashMap<String, Arc<DetectionContext>>> {
        let mut detections: HashMap<String, Arc<DetectionContext>> = HashMap::new();

        match identifier {
            Some(identifier) => {
                // Check if the identifier is a service.
                if self.services.contains_key(&identifier) {
                    let service = &self.services[&identifier];
                    detections.insert(
                        service.plugin.clone(),
                        Arc::new(DetectionContext {
                            services: vec![(
                                identifier.clone(),
                                serde_json::to_vec(&service.settings)?,
                            )],
                            detections: self.read_plugin_files(&service.plugin)?,
                        }),
                    );
                } else {
                    // Otherwise, check if the identifier is an environment.
                    let services_config = self.environment_services(&identifier);
                    if services_config.is_empty() {
                        bail!("invalid identifier: `{identifier}`.");
                    } else {
                        // Use the plugin name from the first service.
                        let plugin_name = &services_config[0].1.plugin;
                        // Map each service in the environment to a tuple (service_name, configuration)
                        let services_vec: Result<Vec<(String, Vec<u8>)>, anyhow::Error> =
                            services_config
                                .iter()
                                .map(|(name, service)| {
                                    Ok((name.clone(), serde_json::to_vec(&service.settings)?))
                                })
                                .collect();
                        detections.insert(
                            plugin_name.clone(),
                            Arc::new(DetectionContext {
                                services: services_vec?,
                                detections: self.read_plugin_files(plugin_name)?,
                            }),
                        );
                    }
                }
            }
            None => {
                // Iterate over each subdirectory in the workspace and collect detection files.
                let workspace_dir = path::Path::new(&self.core.workspace);
                for entry in fs::read_dir(workspace_dir).with_context(|| {
                    format!(
                        "failed to read workspace directory: {}.",
                        workspace_dir.display()
                    )
                })? {
                    let entry = entry?;
                    if entry.path().is_dir() {
                        // Convert the directory name to a &str.
                        if let Some(plugin_name) = entry.file_name().to_str() {
                            // For each plugin, filter services that match its name,
                            // then map each matching service to a Result containing the tuple.
                            let services_vec: Result<Vec<(String, Vec<u8>)>, anyhow::Error> = self
                                .services
                                .iter()
                                .filter(|(_, service)| service.plugin == plugin_name)
                                .map(|(name, service)| {
                                    Ok((name.clone(), serde_json::to_vec(&service.settings)?))
                                })
                                .collect();
                            detections.insert(
                                plugin_name.to_owned(),
                                Arc::new(DetectionContext {
                                    services: services_vec?,
                                    detections: self.read_plugin_files(plugin_name)?,
                                }),
                            );
                        }
                    }
                }
            }
        }
        Ok(detections)
    }
    /// Retrieve all service for an environment
    pub fn environment_services(&self, environment: &str) -> Vec<(String, &Service)> {
        self.services
            .iter()
            .filter_map(|(name, config)| {
                if let Some(env) = &config.environment {
                    if env == environment {
                        return Some((name.clone(), config));
                    }
                }
                None
            })
            .collect()
    }

    /// Reads all files under `<workspace>/<plugin_name>` and returns their contents.
    fn read_plugin_files(&self, plugin_name: &str) -> anyhow::Result<HashMap<String, Vec<u8>>> {
        let plugin_path = path::Path::new(&self.core.workspace).join(plugin_name);

        // Check if the directory exists and is indeed a directory
        if !plugin_path.is_dir() {
            bail!("plugin directory not found: {}", plugin_path.display());
        }

        // Collect detection files from the plugin directory
        let mut file_contents: HashMap<String, Vec<u8>> = HashMap::new();
        for entry_result in fs::read_dir(&plugin_path)? {
            let entry = entry_result?;
            let path = entry.path();

            if path.is_file() {
                let content = fs::read(&path)?;
                file_contents.insert(
                    path.to_str().expect("path is not valid UTF-8").to_owned(),
                    serde_json::to_vec(
                        // Deserialize the YAML file into a serde_json::Value
                        &serde_yaml_ng::from_slice::<Value>(&content)?,
                    )?,
                );
            }
        }

        Ok(file_contents)
    }

    /// Reads all files under `<policies>/<plugin_name>` and returns a concatenated policy.
    pub fn read_plugin_policies(&self, plugin_name: &str) -> anyhow::Result<Vec<(String, Policy)>> {
        let policies_path = path::Path::new(LGC_POLICIES_DIR).join(plugin_name);

        // Create an empty JSON object to store the policies
        let mut policies = vec![];

        // Check if the directory exists and is indeed a directory
        if !policies_path.is_dir() {
            tracing::warn!("no policies for plugin: {}", policies_path.display());
            return Ok(policies);
        }

        // Collect policy files
        for entry in fs::read_dir(&policies_path)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(ext) = path.extension() {
                if ext != "yml" || ext != "yaml" {
                    policies.push((
                        path.display().to_string(),
                        serde_yaml_ng::from_slice::<Policy>(&fs::read(&path)?)
                            .with_context(|| format!("failed to read policy file: {:?}", path))?,
                    ));
                }
            }
        }

        Ok(policies)
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Service {
    pub environment: Option<String>,
    pub plugin: String,
    #[serde(skip_serializing_if = "collections::HashMap::is_empty", default)]
    pub settings: collections::HashMap<String, Value>,
}

impl Service {
    pub fn configure(&mut self, schema: &[u8], use_default: bool) -> anyhow::Result<()> {
        // Prepare the jsonschema validator
        let schema_json: Value = serde_json::from_slice(schema)?;
        let validator = jsonschema::validator_for(&schema_json)?;

        // Parse properties
        let properties: collections::BTreeMap<String, JsonProperty> =
            serde_json::from_value(schema_json["properties"].clone())?;

        let mut settings = collections::HashMap::new();
        for (key, property) in properties {
            // Retrieve any referenced definition and merge it with the property
            let property = property.resolve_definition_if_needed(&schema_json)?;

            // Start parameter prompt
            let value = loop {
                match if use_default {
                    if let Some(default_value) = &property.default {
                        default_value.clone()
                    } else {
                        tracing::warn!("no default value found for `{key}`, using type default.");
                        property.type_default()
                    }
                } else {
                    property.clone().prompt(key.clone())?
                } {
                    Value::Null => break None,
                    value => {
                        // Validate user input with the parameter's jsonschema
                        match validator.validate(&json!({ &key: value })) {
                            Ok(_) => break Some(value),
                            Err(e) => {
                                tracing::error!("{e}");
                                // re-prompt
                            }
                        }
                    }
                }
            };

            // Only insert if the resulting value is not null.
            if let Some(value) = value {
                settings.insert(key, value);
            }
        }

        // Apply the settings to the service
        self.settings = settings;
        Ok(())
    }
}

#[derive(Deserialize, Clone)]
struct JsonProperty {
    /// The `type` of the property
    pub r#type: Option<String>,

    /// The `default` value for this property
    #[serde(default)]
    pub default: Option<Value>,

    /// Possibly store a `description` (to show in prompts)
    pub description: Option<String>,

    /// Optional format for the property
    pub format: Option<String>,

    /// Optional definition for a property
    #[serde(rename(deserialize = "allOf"))]
    pub all_of: Option<Vec<serde_json::Value>>,

    /// Optional enum variants for the property
    #[serde(skip)]
    pub variants: Option<Vec<serde_json::Value>>,
}

impl JsonProperty {
    /// Returns the default value for the property type.
    fn type_default(&self) -> Value {
        match self.r#type.as_deref() {
            Some("string") => Value::String(String::new()),
            Some("boolean") => Value::Bool(false),
            Some("integer") => Value::Number(0.into()),
            Some("array") => Value::Array(vec![]),
            Some("object") => Value::Object(serde_json::Map::new()),
            _ => Value::Null,
        }
    }

    /// Retrieve definition information from the schema and merges them into the property.
    pub fn resolve_definition_if_needed(mut self, root_schema: &Value) -> anyhow::Result<Self> {
        if let Some(items) = &self.all_of {
            for item in items {
                if let Some(ref_str) = item.get("$ref").and_then(Value::as_str) {
                    // Remove the "#/definitions/" prefix to get the definition name.
                    let definition_name = ref_str
                        .strip_prefix("#/definitions/")
                        .ok_or_else(|| anyhow::anyhow!("unexpected $ref format: {}", ref_str))?;

                    // Look up the definition object by its key in the schema.
                    let definition = root_schema
                        .get("definitions")
                        .and_then(Value::as_object)
                        .and_then(|defs| defs.get(definition_name))
                        .ok_or_else(|| {
                            anyhow::anyhow!("could not find definition for {}", definition_name)
                        })?;

                    // Merge fields from the definition.
                    if let Some(t) = definition.get("type").and_then(Value::as_str) {
                        self.r#type = Some(t.to_owned());
                    }
                    if let Some(variants) = definition.get("enum").and_then(Value::as_array) {
                        self.variants = Some(variants.clone());
                    }
                    if let Some(d) = definition.get("default") {
                        self.default = Some(d.clone());
                    }
                    if let Some(fmt) = definition.get("format").and_then(Value::as_str) {
                        self.format = Some(fmt.to_owned());
                    }
                }
            }
        }
        Ok(self)
    }

    fn prompt(self, key: String) -> anyhow::Result<Value> {
        // Initialize the prompt theme
        let prompt_theme = dialoguer::theme::ColorfulTheme::default();

        // Promp for enum if variants are present
        if let Some(variants) = self.variants {
            let items: Vec<String> = variants.iter().map(|v| v.to_string()).collect();

            // Retrieve the default index or set it to first variant
            let default_index = self
                .default
                .as_ref()
                .and_then(|v| items.iter().position(|e| e == &v.to_string()))
                .unwrap_or(0);

            // Prompt the user to select an item from the list of variants
            let selection = dialoguer::Select::with_theme(&prompt_theme)
                .with_prompt(self.description.unwrap_or(key))
                .items(&items)
                .default(default_index)
                .interact()?;

            let value = if let Some(r#type) = self.r#type {
                match r#type.as_str() {
                    "string" => Value::String(items[selection].replace("\"", "")),
                    "integer" => Value::Number(items[selection].parse().unwrap()),
                    "boolean" => Value::Bool(items[selection].parse().unwrap()),
                    _ => bail!("unsupported type: {}. Plugin may be misconfigured.", r#type),
                }
            } else {
                Value::Null
            };

            return Ok(value);
        }

        // Match the type of the property
        let input = if let Some(r#type) = self.r#type {
            match r#type.as_str() {
                "string" => {
                    let input = dialoguer::Input::<String>::with_theme(&prompt_theme)
                        .with_prompt(self.description.unwrap_or(key))
                        .show_default(self.default.is_some())
                        // Safe unwrap as we hide the default if Option is None
                        .default(self.default.unwrap_or_default().to_string())
                        .interact_text()?
                        .replace("\"", "");

                    if input == "null" {
                        Value::Null
                    } else {
                        Value::String(input)
                    }
                }
                "boolean" => Value::Bool(
                    dialoguer::Confirm::with_theme(&prompt_theme)
                        .with_prompt(self.description.unwrap_or(key))
                        .show_default(self.default.is_some())
                        .default(
                            self.default
                                .unwrap_or_default()
                                .as_bool()
                                .unwrap_or_default(),
                        )
                        .interact()?,
                ),
                "integer" => {
                    if self.format.unwrap_or_default().eq("double") {
                        dialoguer::Input::<f64>::with_theme(&prompt_theme)
                            .with_prompt(self.description.unwrap_or(key))
                            .show_default(self.default.is_some())
                            .default(
                                self.default
                                    .unwrap_or_default()
                                    .as_f64()
                                    .unwrap_or_default(),
                            )
                            .interact_text()?
                            .into()
                    } else {
                        json!(dialoguer::Input::<i64>::with_theme(&prompt_theme)
                            .with_prompt(self.description.unwrap_or(key))
                            .show_default(self.default.is_some())
                            .default(
                                self.default
                                    .unwrap_or_default()
                                    .as_i64()
                                    .unwrap_or_default()
                            )
                            .interact_text()?)
                    }
                }
                "array" => {
                    tracing::warn!("using default value for `{key}` (array).");
                    Value::Array(
                        self.default
                            .unwrap_or_default()
                            .as_array()
                            .cloned()
                            .unwrap_or_default(),
                    )
                }
                "object" => {
                    tracing::warn!("using default value for `{key}` (object).");
                    Value::Object(
                        self.default
                            .unwrap_or_default()
                            .as_object()
                            .cloned()
                            .unwrap_or_default(),
                    )
                }
                "null" => Value::Null,
                _ => {
                    bail!("unsupported type: {}. Plugin may be misconfigured.", r#type);
                }
            }
        } else {
            Value::Null
        };

        Ok(input)
    }
}
