// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use lgc_common::{
    configuration,
    plugins::manager::{PluginActions, PluginManager},
    utils::filter_missing_plugins,
};

use lgc_policies::policy::Severity;

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

        let mut has_warning = false;
        let mut has_error = false;
        for (plugin, detections) in &detections {
            // Load policies per plugin
            let policies = config.read_plugin_policies(plugin)?;
            if policies.is_empty() && !self.quiet {
                tracing::info!("0 policies loaded for plugin '{plugin}'.");
                continue;
            }

            for (policy_path, policy) in policies {
                let schema = policy
                    .to_schema()
                    .map_err(|e| anyhow::anyhow!("incorrect policy '{policy_path}': {e}"))?;

                let validator = jsonschema::Validator::new(&schema)?;
                let message = if let Some(message) = &policy.message {
                    message
                } else {
                    &policy.default_message()
                };

                // Validate detections against policies
                for (detection_path, content) in &detections.detections {
                    let val: serde_json::Value = serde_json::from_slice(content)?;
                    match validator.validate(&val) {
                        Ok(_) => (),
                        Err(_) => match policy.severity {
                            Severity::Error => {
                                tracing::error!("{message} (policy: {policy_path}, detection: {detection_path})");
                                has_error = true;
                            }
                            Severity::Warning => {
                                tracing::warn!("{message} (policy: {policy_path}, detection: {detection_path})");
                                has_warning = true;
                            }
                        },
                    }
                }
            }
        }

        // Prepare plugin manager and tasks JoinSet.
        let plugin_manager = PluginManager::new()?;
        let mut plugin_tasks = tokio::task::JoinSet::new();

        // Retrieve plugin directory and filter out plugins that do not exist.
        let plugins_dir = filter_missing_plugins(
            config.core.base_dir,
            &config.core.workspace,
            &mut detections,
        );

        // Collect the keys into a new vector.
        let plugin_keys: Vec<_> = detections.keys().cloned().collect();

        for plugin in plugin_keys {
            // Check if the plugin exists.
            let plugin_path = plugins_dir.join(&plugin).with_extension("wasm");
            if !plugin_path.exists() {
                tracing::warn!(
                    "ignoring '{}/{}' (no matching plugin).",
                    config.core.workspace,
                    plugin
                );
                continue;
            }

            let plugin_manager = plugin_manager.clone();
            // Now it's safe to remove the plugin's detections from `detections`.
            let plugin_detections = detections.remove(&plugin).ok_or_else(|| {
                anyhow::anyhow!(
                    "unexpected error. No detection data found for plugin '{}'.",
                    plugin
                )
            })?;

            // Spawn a task that does both instantiation and validation.
            plugin_tasks.spawn(async move {
                let (instance, mut store) = plugin_manager.load_plugin(plugin_path).await?;
                let mut errors = Vec::new();

                for (path, content) in &plugin_detections.detections {
                    if let Err(e) = instance.validate(&mut store, content).await {
                        errors.push((path.clone(), e));
                    }
                }
                Ok::<_, anyhow::Error>(errors)
            });
        }

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

        if !self.quiet && !has_error && !has_warning {
            tracing::info!("all good, no problem identified.");
        } else if has_error {
            std::process::exit(1);
        }

        Ok(())
    }
}
