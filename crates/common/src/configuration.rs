// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::bail;
use anyhow::Result;
use kclvm_api::{
    gpyrpc::{GetSchemaTypeArgs, ValidateCodeArgs},
    service::KclvmServiceImpl,
};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Number;
use serde_yaml_ng::Value;
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
use crate::utils::ensure_kebab_case;

/// ProjectConfiguration definition
/// BTreeSet has been chosen rather than BTreeMap in order to improve readability over name field in config file.
/// We know this is not optimal
/// Hash is calculated for the name field to provide unique objects.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ProjectConfiguration {
    pub plugins: BTreeMap<String, Plugin>,
    pub environments: BTreeSet<Environment>,
    pub services: BTreeSet<Service>,
}

impl ProjectConfiguration {
    pub fn save_config(&self, path: Option<&PathBuf>) -> Result<()> {
        let buffer = File::create(path.unwrap_or(&PathBuf::from_str(LGC_CONFIG_PATH)?))?;

        serde_yaml_ng::to_writer(BufWriter::new(buffer), &self)?;
        println!("changes saved successfully");
        Ok(())
    }

    pub fn environment_names(&self) -> Result<Vec<String>> {
        self.environments
            .iter()
            .map(|env| ensure_kebab_case(&env.name))
            .collect()
    }

    pub fn service_names(&self) -> Result<Vec<String>> {
        self.services
            .iter()
            .map(|svc| ensure_kebab_case(&svc.name))
            .collect()
    }

    pub fn remove_service(&mut self, name: &String) {
        self.services.remove(&Service {
            name: name.to_owned(),
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
    pub name: String,
    pub services: BTreeSet<String>,
}

impl PartialEq for Environment {
    fn eq(&self, other: &Environment) -> bool {
        self.name == other.name
    }
}

impl Hash for Environment {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialOrd for Environment {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Environment {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

#[derive(Eq, Serialize, Deserialize, Default, Clone)]
pub struct Service {
    pub name: String,
    pub plugin: String,
    pub settings: BTreeMap<String, Value>,
}

impl PartialEq for Service {
    fn eq(&self, other: &Service) -> bool {
        self.name == other.name
    }
}

impl Hash for Service {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialOrd for Service {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Service {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl Service {
    pub fn configure(&mut self, code: String, default: bool) -> Result<()> {
        // Init kcl service
        let serv = KclvmServiceImpl::default();
        // Load package configuration schema
        let args = GetSchemaTypeArgs {
            code: code.clone(),
            schema_name: "Configuration".to_string(),
            ..Default::default()
        };

        // Retrieve schema type
        let schema = serv.get_schema_type(&args)?.schema_type_list;

        // Check if Configuration schema exists
        if schema.is_empty() {
            println!("Plugin does not provides schema for dynamic configuration");
            return Ok(());
        }

        // Ask for arguments
        for (p_name, p_type) in &schema[0].properties {
            if default {
                self.settings.insert(
                    p_name.to_string(),
                    Value::String(
                        p_type
                            .default
                            .trim_matches(|c| c == '"' || c == '\'')
                            .to_string(),
                    ),
                );
            } else {
                println!("Configure the service:");
                let msg = p_type.description.as_str();
                let res: Value = match p_type.r#type.as_str() {
                    "str" => Value::String(
                        dialoguer::Input::<String>::new()
                            .with_prompt(msg.trim_matches(|c| c == '"' || c == '\''))
                            .interact_text()?,
                    ),
                    "bool" => Value::Bool(inquire::prompt_confirmation(msg)?),
                    "float" => Value::Number(Number::from(inquire::prompt_f32(msg)?)),
                    "int" => Value::Number(Number::from(inquire::prompt_u32(msg)?)),
                    _ => {
                        println!(
                            "Using default for `{}`, type `{}` is not implemened yet",
                            p_name,
                            p_type.r#type.as_str()
                        );
                        serde_json::from_str(p_type.default.as_str())?
                    }
                };
                if !res.is_null() {
                    self.settings.insert(p_name.to_string(), res);
                }
            }
        }

        self.validate(code, serde_json::to_string(&self.settings)?)?;

        Ok(())
    }

    pub fn validate(&self, code: String, data: String) -> Result<()> {
        let serv = KclvmServiceImpl::default();

        let args = ValidateCodeArgs {
            code,
            data,
            schema: String::from("Configuration"),
            ..Default::default()
        };

        let check = serv.validate_code(&args)?;
        if !check.success {
            bail!(check.err_message)
        };

        Ok(())
    }
}
