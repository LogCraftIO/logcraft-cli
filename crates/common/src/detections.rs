// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::{bail, Result};
use console::{style, Style};
use dashmap::DashMap;
use kclvm_api::gpyrpc::ValidateCodeArgs;
use kclvm_api::service::KclvmServiceImpl;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};

use crate::{
    configuration::{Service, LGC_RULES_DIR},
    plugins::LGC_PLUGINS_PATH,
};

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

// Helper types to store detections per plugin or per service
pub type PluginDetections = HashMap<String, HashSet<DetectionState>>;
pub type ServiceDetections = HashMap<String, HashSet<DetectionState>>;

#[derive(Serialize, Deserialize, Clone)]
pub struct Detection {
    pub name: String,
    pub rules: HashMap<String, Value>,
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
            tracing::error!(
                "failed to verify detection file `{}`: {}",
                path,
                check.err_message
            );
            std::process::exit(1);
        };

        serde_yaml_ng::from_str(&fs::read_to_string(&path)?)
            .map_err(|e| anyhow::Error::msg(format!("{e}")))
    }
}

pub fn map_plugin_detections(
    detection_id: Option<String>,
) -> Result<HashMap<String, HashSet<DetectionState>>> {
    let entries: Vec<PathBuf> = if let Some(detection_id) = detection_id {
        let detection_path = PathBuf::from(format!("{}/{}.yaml", LGC_RULES_DIR, detection_id));
        if detection_path.is_file() {
            vec![detection_path]
        } else {
            bail!("detection `{}` does not exist", detection_id)
        }
    } else {
        fs::read_dir(LGC_RULES_DIR)?
            .filter_map(|file| file.ok().map(|f| f.path()))
            .collect()
    };

    let plugins: DashMap<String, HashSet<DetectionState>> = DashMap::new();

    // Check plugin existence
    if !PathBuf::from(LGC_PLUGINS_PATH).exists() {
        bail!("plugin directory `{LGC_PLUGINS_PATH}` does not exist. Have you installed plugins?")
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
                    Ok(detection) => Some((path, detection)),
                    Err(e) => {
                        tracing::error!("{e}");
                        None
                    }
                }
            }
            _ => None,
        })
        .for_each(|(path, detection)| {
            detection.rules.into_iter().for_each(|(plugin, content)| {
                if plugins_name.contains(&plugin) {
                    if !plugins.entry(plugin).or_default().insert(DetectionState {
                        name: detection.name.clone(),
                        content,
                    }) {
                        tracing::error!(
                            "detection duplication - {} appears again in: {}",
                            &detection.name,
                            path.display()
                        );
                        std::process::exit(1);
                    };
                } else {
                    tracing::error!(
                        "referenced plugin `{}` in `{}` does not exist",
                        &plugin,
                        path.display()
                    )
                }
            });
        });

    Ok(plugins.into_iter().collect())
}

#[derive(Eq, Debug, Clone, Serialize, Deserialize)]
pub struct DetectionState {
    pub name: String,
    pub content: Value,
}

impl PartialEq for DetectionState {
    fn eq(&self, other: &DetectionState) -> bool {
        self.name == other.name
    }
}

impl Hash for DetectionState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

// Return true if there is a change in detections
pub fn compare_detections(
    detections: &PluginDetections,
    retrieved_detections: &ServiceDetections,
    services: &HashMap<String, Vec<&Service>>,
    debug: bool,
) -> ServiceDetections {
    let changed: DashMap<String, HashSet<DetectionState>> = DashMap::new();

    detections.par_iter().for_each(|(plugin_name, rules)| {
        if let Some(services) = services.get(plugin_name) {
            for service in services {
                if let Some(retrieved) = retrieved_detections.get(&service.id) {
                    for rule in rules {
                        if let Some(retrieved_rule) = retrieved.get(rule) {
                            let retrieved =
                                serde_json::to_string_pretty(&retrieved_rule.content).unwrap();
                            let requested = serde_json::to_string_pretty(&rule.content).unwrap();
                            if retrieved != requested {
                                changed
                                    .entry(service.id.clone())
                                    .and_modify(|s| {
                                        s.insert(rule.clone());
                                    })
                                    .or_insert(HashSet::from([rule.clone()]));
                                if debug {
                                    println!(
                                        "[~] rule: `{}` will be updated on `{}`:",
                                        style(&rule.name).yellow(),
                                        &service.id
                                    );
                                    show_diff(&retrieved, &requested);
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    changed.into_iter().collect()
}

pub fn show_diff(old: &str, new: &str) {
    let diff = TextDiff::from_lines(old, new);
    for op in diff.ops() {
        for change in diff.iter_changes(op) {
            let (sign, style) = match change.tag() {
                ChangeTag::Delete => ("| - ", Style::new().red()),
                ChangeTag::Insert => ("| + ", Style::new().green()),
                ChangeTag::Equal => ("|   ", Style::new().dim()),
            };
            print!("{}{}", style.apply_to(sign).bold(), style.apply_to(change));
        }
    }
}
