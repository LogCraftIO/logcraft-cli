// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::bail;
use anyhow::Result;
use dialoguer::Confirm;
use dialoguer::Input;
use dialoguer::Password;
use dialoguer::Select;
use kclvm_api::{gpyrpc::ValidateCodeArgs, API};
use kclvm_query::get_schema_type;
use kclvm_query::GetSchemaOption;
use kclvm_sema::ty::TypeKind;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::{
    fs::File,
    hash::{Hash, Hasher},
    io::BufWriter,
    path::PathBuf,
    str::FromStr,
};

pub const LGC_CONFIG_PATH: &str = "lgc.yaml";
pub const LGC_RULES_DIR: &str = "rules";

use crate::plugins::Plugin;
use crate::state::backends::StateBackend;
use crate::utils::ensure_kebab_case;

/// ProjectConfiguration definition
/// BTreeSet has been chosen rather than BTreeMap in order to improve readability over name field in config file.
/// We know this is not optimal
/// Hash is calculated for the name field to provide unique objects.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ProjectConfiguration {
    #[serde(default)]
    pub state: StateBackend,
    pub plugins: BTreeMap<String, Plugin>,
    pub environments: BTreeSet<Environment>,
    pub services: BTreeSet<Service>,
}

impl ProjectConfiguration {
    pub fn save_config(&self, path: Option<&PathBuf>) -> Result<()> {
        let buffer = File::create(path.unwrap_or(&PathBuf::from_str(LGC_CONFIG_PATH)?))?;

        serde_yaml_ng::to_writer(BufWriter::new(buffer), &self)?;
        Ok(())
    }

    pub fn environment_ids(&self) -> Result<Vec<&str>> {
        self.environments
            .iter()
            .map(|env| ensure_kebab_case(&env.id))
            .collect()
    }

    pub fn service_ids(&self) -> Result<Vec<&str>> {
        self.services
            .iter()
            .map(|svc| ensure_kebab_case(&svc.id))
            .collect()
    }

    pub fn remove_service(&mut self, id: &String) {
        self.services.remove(&Service {
            id: id.to_owned(),
            ..Default::default()
        });
    }

    pub fn unlink_environments(&mut self, name: &String) {
        // Cannot mutate BTreeSet in place, replacing configuration envs with a new one.
        let mut modified_envs: BTreeSet<Environment> = BTreeSet::new();
        self.environments.clone().into_iter().for_each(|mut env| {
            // Seams lighter to remove without check than check name existence.
            env.services.remove(name);
            modified_envs.insert(env);
        });
        self.environments = modified_envs;
    }
}

#[derive(Eq, Serialize, Deserialize, Default, Clone)]
pub struct Environment {
    pub id: String,
    pub services: BTreeSet<String>,
}

impl PartialEq for Environment {
    fn eq(&self, other: &Environment) -> bool {
        self.id == other.id
    }
}

impl Hash for Environment {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialOrd for Environment {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Environment {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

#[derive(Eq, Serialize, Deserialize, Default, Clone)]
pub struct Service {
    pub id: String,
    pub plugin: String,
    pub settings: BTreeMap<String, Value>,
}

impl PartialEq for Service {
    fn eq(&self, other: &Service) -> bool {
        self.id == other.id
    }
}

impl Hash for Service {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialOrd for Service {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Service {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl Service {
    pub fn configure(&mut self, code: String, default: bool) -> Result<()> {
        let schema = get_schema_type(
            "",
            Some(&code),
            Some("Configuration"),
            GetSchemaOption::Definitions,
        )?;

        let (attributes, doc) = match schema.get("Configuration") {
            Some(schema) => (schema.attrs.clone(), schema.doc.to_string()),
            None => {
                tracing::info!(
                    "plugin does not provides configuration schema for dynamic configuration"
                );
                return Ok(());
            }
        };

        if !default {
            if doc.is_empty() {
                println!("Service configuration: ");
            } else {
                println!("{doc}:");
            }
        }

        for (attr_name, attr_type) in attributes.into_iter() {
            if default {
                let default = attr_type.ty.kind.defaut(&attr_name, attr_type.default)?;
                self.settings.insert(attr_name, default);
            } else {
                let name = if let Some(doc) = attr_type.doc {
                    trim_quotes(&doc)
                } else {
                    attr_name.to_string()
                };

                let default = if let Some(default) = self.settings.get(&attr_name) {
                    Some(trim_quotes(&serde_json::to_string(default)?))
                } else {
                    attr_type.default
                };

                let sensitive = attr_type
                    .decorators
                    .iter()
                    .any(|decorator| decorator.keywords.contains_key("sensitive"));

                let res = attr_type.ty.kind.prompt(&name, default, sensitive)?;
                if !res.is_null() {
                    self.settings.insert(attr_name.to_string(), res);
                }
            }
        }

        self.validate(code, serde_json::to_string(&self.settings)?)?;

        Ok(())
    }

    pub fn validate(&self, code: String, data: String) -> Result<()> {
        let kcl_api = API::default();

        let args = ValidateCodeArgs {
            code,
            data,
            schema: String::from("Configuration"),
            ..Default::default()
        };

        let check = kcl_api.validate_code(&args)?;
        if !check.success {
            bail!(check.err_message)
        };

        Ok(())
    }
}

fn trim_quotes(s: &str) -> String {
    s.trim_matches(|c| c == '"' || c == '\'').to_string()
}

fn match_bool(s: &str) -> bool {
    match s {
        "True" => true,
        "False" => false,
        "true" => true,
        "false" => false,
        _ => false,
    }
}

trait Prompt {
    fn prompt(&self, name: &str, default: Option<String>, sensitive: bool) -> Result<Value>;
    fn defaut(&self, name: &str, default: Option<String>) -> Result<Value>;
}

impl Prompt for TypeKind {
    fn defaut(&self, name: &str, default: Option<String>) -> Result<Value> {
        if default.is_none() {
            tracing::warn!("schema does not provides default value for {}", &name);
        }

        let default = match self {
            TypeKind::Str => Value::String(trim_quotes(&default.unwrap_or_default())),
            TypeKind::Bool => Value::Bool(match_bool(&default.unwrap_or("False".to_string()))),
            TypeKind::Float => json!(default
                .unwrap_or("0.0".to_string())
                .parse::<f64>()
                .unwrap_or(0.0)),
            TypeKind::Int => Value::Number(
                default
                    .unwrap_or("0".to_string())
                    .parse::<i64>()
                    .unwrap_or(0)
                    .into(),
            ),
            TypeKind::None | TypeKind::Void => Value::Null,
            ty => match serde_json::from_str(default.unwrap_or_default().as_str()) {
                Ok(res) => res,
                Err(_) => match ty {
                    TypeKind::List(_) => Value::Array(vec![]),
                    TypeKind::Dict(_) => Value::Object(Map::new()),
                    _ => Value::Null,
                },
            },
        };

        Ok(default)
    }

    fn prompt(&self, name: &str, default: Option<String>, sensitive: bool) -> Result<Value> {
        let prompt_theme = dialoguer::theme::ColorfulTheme::default();
        let value = match self {
            Self::Str => {
                let input = if sensitive {
                    Password::with_theme(&prompt_theme)
                        .with_prompt(format!("{} (hidden)", name))
                        .interact()?
                } else {
                    Input::<String>::with_theme(&prompt_theme)
                        .with_prompt(name)
                        .show_default(default.is_some())
                        .default(default.unwrap_or_default())
                        .interact_text()?
                };
                Value::String(trim_quotes(&input))
            }
            Self::Bool => Value::Bool(
                Confirm::with_theme(&prompt_theme)
                    .with_prompt(name)
                    .show_default(default.is_some())
                    .default(match_bool(&default.unwrap_or("False".to_string())))
                    .interact()?,
            ),
            Self::Int => Value::Number(
                Input::<i64>::with_theme(&prompt_theme)
                    .with_prompt(name)
                    .show_default(default.is_some())
                    .default(
                        default
                            .unwrap_or("0".to_string())
                            .parse::<i64>()
                            .unwrap_or(0),
                    )
                    .interact_text()?
                    .into(),
            ),
            Self::Float => json!(Input::<f64>::with_theme(&prompt_theme)
                .with_prompt(name)
                .show_default(default.is_some())
                .default(
                    default
                        .unwrap_or("0.0".to_string())
                        .parse::<f64>()
                        .unwrap_or(0.0)
                )
                .interact_text()?),
            Self::Union(types) => {
                let options = types
                    .iter()
                    .filter_map(|r#type| r#type.kind.prompt(name, default.clone(), sensitive).ok())
                    .collect::<Vec<_>>();

                let selection = Select::with_theme(&prompt_theme)
                    .with_prompt(name)
                    .items(&options)
                    .default(0)
                    .interact()?;

                options[selection].clone()
            }
            Self::StrLit(val) => json!(val),
            Self::BoolLit(val) => json!(val),
            Self::FloatLit(val) => json!(val),
            Self::IntLit(val) => json!(val),
            Self::None | Self::Void => Value::Null,
            ty => match serde_json::from_str(default.unwrap_or_default().as_str()) {
                Ok(res) => res,
                Err(_) => {
                    tracing::info!("Using default for `{}`", name);
                    match ty {
                        TypeKind::List(_) => Value::Array(vec![]),
                        TypeKind::Dict(_) => Value::Object(Map::new()),
                        _ => Value::Null,
                    }
                }
            },
        };

        Ok(value)
    }
}
