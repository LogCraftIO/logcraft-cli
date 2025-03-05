// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use std::path;

use lgc_common::{
    configuration::{self, LGC_BASE_DIR},
    plugins::manager::{PluginActions, PluginManager},
};

use lgc_policies::Severity;

/// Validate detection rules
#[derive(clap::Parser)]
#[clap(about = "Validate local detection rules", allow_hyphen_values = true)]
pub struct ValidateCommand {
    /// Quiet mode
    #[clap(short, long)]
    pub quiet: bool,
}

impl ValidateCommand {
    pub async fn run(self, config: configuration::ProjectConfiguration) -> anyhow::Result<()> {
        // Load detections
        let mut detections = config.load_detections(None)?;
        if detections.is_empty() {
            anyhow::bail!("nothing to validate, no detection found.");
        }

        // Load policies
        for (plugin, detections) in &detections {
            for (policy_path, policy) in config.read_plugin_policies(plugin)? {
                let validator = jsonschema::Validator::new(&policy.to_schema())?;

                for (detection_path, content) in &detections.detections {
                    let val: serde_json::Value = serde_json::from_slice(content)?;
                    match validator.validate(&val) {
                        Ok(_) => (),
                        Err(_) => match policy.severity {
                            Severity::Error => {
                                tracing::error!(
                                    "{} (policy: {}, detection: {})",
                                    policy.default_message(),
                                    policy_path,
                                    detection_path
                                );
                            }
                            Severity::Warning => {
                                tracing::warn!(
                                    "{} (policy: {}, detection: {})",
                                    policy.default_message(),
                                    policy_path,
                                    detection_path
                                );
                            }
                        },
                    }
                }
            }
        }

        // Validate detections against policies

        // Prepare plugin manager and tasks JoinSet.
        let plugin_manager = PluginManager::new()?;
        let mut plugin_tasks = tokio::task::JoinSet::new();

        let plugins_dir =
            path::PathBuf::from(config.core.base_dir.as_deref().unwrap_or(LGC_BASE_DIR))
                .join("plugins");

        let plugin_names: Vec<String> = detections.keys().cloned().collect();

        // Instantiate plugins & validate detections
        for plugin in plugin_names {
            // Check if the plugin exists.
            let plugin_path = plugins_dir.join(&plugin).with_extension("wasm");
            if !plugin_path.exists() {
                tracing::warn!(
                    "ignoring '{}/{}' (no matching plugin).",
                    config.core.workspace,
                    plugin
                );
                // Proceed to the next plugin.
                continue;
            }

            // Data to be used in the task.
            let plugin_manager = plugin_manager.clone();
            let detections = detections.remove(&plugin).ok_or_else(|| {
                anyhow::anyhow!(
                    "unexpected error. No detection data found for plugin '{}'.",
                    plugin
                )
            })?;

            // Spawn a task that does both instantiation and validation.
            plugin_tasks.spawn(async move {
                // Instantiate the plugin.
                let (instance, mut store) = plugin_manager.load_plugin(plugin_path).await?;
                let mut errors = Vec::new();

                // Validate plugin detection.
                for (path, content) in &detections.detections {
                    if let Err(e) = instance.validate(&mut store, content).await {
                        errors.push((path.clone(), e));
                    }
                }
                // Return collected errors.
                Ok::<_, anyhow::Error>(errors)
            });
        }

        let mut has_error = false;
        // Process the results of each plugin task.
        while let Some(join_result) = plugin_tasks.join_next().await {
            match join_result {
                Ok(result) => {
                    let errors = result?;
                    for (path, err) in errors {
                        tracing::error!("validation failed on '{path}': {err}");
                        has_error = true;
                    }
                }
                Err(e) => {
                    // A panic in one of the spawned tasks.
                    tracing::error!("plugin panicked: {:?}", e);
                    has_error = true;
                }
            }
        }

        if !self.quiet && !has_error {
            tracing::info!("all good, no problem identified.");
        }

        Ok(())
    }
}
