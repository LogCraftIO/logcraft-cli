// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::{bail, Result};
use dashmap::DashMap;
use kclvm_api::gpyrpc::ValidateCodeArgs;
use kclvm_api::service::KclvmServiceImpl;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value;
use std::fs;
use std::path::PathBuf;
use std::{collections::HashMap, sync::Mutex};

use crate::{configuration::LGC_RULES_DIR, plugins::LGC_PLUGINS_PATH};

pub const GENERIC_DETECTION: &str = r#"
schema Detection:
    """
    Attributes
    ----------
    name : str, required,
        Name of the detection
    rules: [any], required,
        <plugin>:
            Plugin specific implementation
        <plugin>:
            Plugin specific implementation
    """
    name: str
    rules: {str:any}
"#;

#[derive(Serialize, Deserialize, Clone)]
pub struct Detection {
    pub name: String,
    pub rules: std::collections::HashMap<String, serde_yaml_ng::Value>,
}

impl Detection {
    pub fn pre_validate(path: String) -> Result<Self> {
        // KCL validation
        // ! Validation does not provide specific check for now
        // ! It is used for better configuration messages
        let serv = KclvmServiceImpl::default();
        let args = ValidateCodeArgs {
            datafile: path.clone(),
            code: GENERIC_DETECTION.to_string(),
            schema: String::from("Detection"),
            format: String::from("yaml"),
            ..Default::default()
        };

        let check = serv.validate_code(&args)?;
        if !check.success {
            eprintln!(
                "Failed to verify detection file `{}`: {}",
                path, check.err_message
            );
            std::process::exit(1);
        };

        let data = fs::read_to_string(&path)?;
        let detection: Self =
            serde_yaml_ng::from_str(&data).map_err(|e| anyhow::Error::msg(format!("{e}")))?;

        Ok(detection)
    }
}

pub fn map_plugin_detections() -> Result<HashMap<String, Vec<(String, Value)>>> {
    let entries: Vec<PathBuf> = fs::read_dir(LGC_RULES_DIR)?
        .filter_map(|file| file.ok().map(|f| f.path()))
        .collect();

    let plugins: DashMap<String, Vec<(String, Value)>> = DashMap::new();
    let detection_names: Mutex<Vec<String>> = Mutex::new(Vec::new());

    // Check plugin existence
    if !PathBuf::from(LGC_PLUGINS_PATH).exists() {
        bail!("Plugin directory `{LGC_PLUGINS_PATH}` does not exist. Have you installed plugins?")
    }

    let plugins_name: Vec<String> = fs::read_dir(LGC_PLUGINS_PATH)?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_file() {
                path.file_name()?.to_str().map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();

    // Map detections for each plugin
    entries
        .into_par_iter()
        .filter_map(|path| match path.extension().and_then(|ext| ext.to_str()) {
            Some("yml") | Some("yaml") => {
                match Detection::pre_validate(path.display().to_string()) {
                    Ok(detection) => {
                        let mut detection_names_lock = detection_names.lock().unwrap();
                        if detection_names_lock.contains(&detection.name) {
                            eprintln!(
                                "error: detection duplication - {} appears again in: {}",
                                &detection.name,
                                path.display()
                            );
                            std::process::exit(1);
                        } else {
                            detection_names_lock.push(detection.name.clone());
                            Some((path, detection))
                        }
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        None
                    }
                }
            }
            _ => None,
        })
        .for_each(|(path, detection)| {
            detection.rules.into_iter().for_each(|(plugin, params)| {
                if plugins_name.contains(&plugin) {
                    plugins
                        .entry(plugin)
                        .or_default()
                        .push((detection.name.clone(), params))
                } else {
                    eprintln!(
                        "referenced plugin `{}` in `{}` does not exist",
                        &plugin,
                        path.display()
                    )
                }
            });
        });

    Ok(plugins.into_iter().collect())
}
