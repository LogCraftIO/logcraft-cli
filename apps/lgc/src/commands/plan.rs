// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use lgc_common::{
    configuration,
    detections::PluginsDetections,
    diff::{DiffConfig, ADD_STYLE, BOLD_STYLE, MODIFY_STYLE, REMOVE_STYLE},
    plugins::manager::{PluginActions, PluginManager},
    utils::filter_missing_plugins,
};
use serde_json::Value;
use std::{
    collections::{self, HashSet},
    io::Write,
};
use tokio::task::JoinSet;

/// Plan configuration
#[derive(clap::Parser)]
#[clap(
    about = "Preview the changes that lgc will make",
    allow_hyphen_values = true
)]
pub struct PlanCommand {
    /// Service identifier
    pub identifier: Option<String>,

    /// Uses only the state to plan the changes
    #[clap(short, long)]
    pub state_only: bool,

    /// Verbose mode
    #[clap(short, long)]
    pub verbose: bool,
}

impl PlanCommand {
    pub async fn run(self, config: configuration::ProjectConfiguration) -> anyhow::Result<()> {
        // Load detections from workspace.
        let mut context = config.load_detections(self.identifier)?;

        // Exit early if no detections are found.
        if context.is_empty() {
            anyhow::bail!("nothing to apply, no detection found.");
        }

        // Retrieve plugin directory and filter out plugins that do not exist.
        let plugins_dir =
            filter_missing_plugins(config.core.base_dir, &config.core.workspace, &mut context);

        // Retrieve current state.
        let state_backend = config.state.unwrap_or_default();
        let (exists, mut state) = state_backend.load().await?;
        if !exists && self.state_only {
            anyhow::bail!("state missing, cannot determine changes.");
        }

        if !self.state_only {
            // Prepare plugin engine and spawned futures set.
            let plugin_manager = PluginManager::new()?;
            let mut join_set = JoinSet::new();

            // Spawn a task per plugin that will retrieve the detections for all related services.
            for (plugin, context) in context.iter() {
                let plugin_path = plugins_dir.join(plugin).with_extension("wasm");
                let plugin_manager = plugin_manager.clone();

                // Cheap clone of context
                let context = context.clone();
                join_set.spawn(async move {
                    let (instance, mut store) = plugin_manager.load_plugin(plugin_path).await?;
                    let mut results: PluginsDetections = collections::HashMap::new();
                    for (service_name, settings) in &context.services {
                        let mut service_detections = collections::HashMap::new();
                        for (path, content) in &context.detections {
                            match instance.read(&mut store, settings, content).await {
                                Ok(Some(detection)) => {
                                    let raw_json: Value =
                                        serde_json::from_slice(&detection).map_err(|e| {
                                            anyhow::anyhow!(
                                                "plugin returned invalid JSON for detection '{}': {}",
                                                path,
                                                e
                                            )
                                        })?;
                                    service_detections.insert(path.clone(), raw_json);
                                }
                                Ok(None) => {
                                    // Insert with Null value to remove the rule in state merge_sync method later.
                                    service_detections.insert(path.clone(), Value::Null);
                                }
                                Err(e) => {
                                    anyhow::bail!(
                                        "retrieving {} for service {}: {}",
                                        path,
                                        service_name,
                                        e
                                    )
                                }
                            }
                        }
                        if !service_detections.is_empty() {
                            results.insert(service_name.clone(), service_detections);
                        }
                    }
                    Ok::<PluginsDetections, anyhow::Error>(results)
                });
            }

            // Merge the plugin detections into the state.
            while let Some(res) = join_set.join_next().await {
                state.merge_synced(res??);
            }
        }

        // Prepare the diff configuration.
        let diff_config = DiffConfig::default();
        // Prepare the output writer.
        let stdout = std::io::stdout();
        let mut writer = std::io::BufWriter::new(stdout.lock());
        let mut has_diff = false;

        // Compute the diff between the state and the definitions.
        for name in context.keys() {
            let detection_ctx = match context.get(name) {
                Some(ctx) => ctx.clone(),
                None => continue,
            };

            // For each service in the context
            for (svc_name, _) in &detection_ctx.services {
                // Retrieve state service rules
                if let Some(svc_rules) = state.services.get(svc_name) {
                    // Retrieve detections definitions paths
                    let mut detection_keys: HashSet<&String> =
                        detection_ctx.detections.keys().collect();

                    // Loop over the state rules and compare them with the context.
                    for (path, plugin_val) in svc_rules {
                        match detection_ctx.detections.get(path) {
                            // Rule is in both context and state
                            Some(desired) => {
                                let desired: Value = serde_json::from_slice(desired)?;
                                if &desired != plugin_val {
                                    println!(
                                        "[~] {} will be updated on {}",
                                        MODIFY_STYLE.apply_to(path),
                                        BOLD_STYLE.apply_to(svc_name),
                                    );

                                    if self.verbose {
                                        diff_config.diff_json(&desired, plugin_val, &mut writer)?;

                                        writer.flush()?;
                                    }
                                    has_diff = true;
                                }
                                detection_keys.remove(path);
                            }
                            // Rule is not in the context but is in the state
                            None => {
                                println!(
                                    "[-] {} will be removed from {}",
                                    REMOVE_STYLE.apply_to(path),
                                    BOLD_STYLE.apply_to(svc_name),
                                );
                                detection_keys.remove(path);
                                has_diff = true;
                            }
                        }
                    }

                    // Check what remains in the detection context that is not in the state
                    for rule in detection_keys {
                        println!(
                            "[+] {} will be created on {}",
                            ADD_STYLE.apply_to(rule),
                            BOLD_STYLE.apply_to(svc_name),
                        );
                        has_diff = true;
                    }
                }
            }
        }

        if !has_diff {
            tracing::info!("no changes detected.");
        }

        Ok(())
    }
}
