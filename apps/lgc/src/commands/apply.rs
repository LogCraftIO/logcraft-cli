// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use dialoguer::Confirm;
use lgc_common::{
    configuration::{self, LGC_BASE_DIR},
    detections::PluginsDetections,
    diff::{DiffConfig, ADD_STYLE, BOLD_STYLE, MODIFY_STYLE, REMOVE_STYLE},
    plugins::manager::{PluginActions, PluginManager},
};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    io::{self, Write},
    path, sync,
};
use tokio::task::JoinSet;

#[derive(clap::Parser)]
#[clap(about = "Apply changes to remote services", allow_hyphen_values = true)]
pub struct ApplyCommand {
    /// Service identifier (optional)
    pub identifier: Option<String>,

    /// Skip interactive approval of plan before applying.
    #[clap(short, long)]
    pub auto_approve: bool,
}

impl ApplyCommand {
    pub async fn run(self, config: configuration::ProjectConfiguration) -> anyhow::Result<()> {
        // Load detections from workspace.
        let mut context = sync::Arc::new(config.load_detections(self.identifier)?);

        // Exit early if no detections are found.
        if context.is_empty() {
            anyhow::bail!("nothing to apply, no detection found.");
        }

        // Retrieve plugin directory and filter out plugins that do not exist.
        let plugins_dir =
            path::PathBuf::from(config.core.base_dir.as_deref().unwrap_or(LGC_BASE_DIR))
                .join("plugins");

        sync::Arc::make_mut(&mut context).retain(|name, _| {
            let exists = plugins_dir.join(name).with_extension("wasm").exists();
            if !exists {
                tracing::warn!(
                    "ignoring '{}/{}' (no matching plugin).",
                    config.core.workspace,
                    name
                );
            }
            exists
        });

        // Retrieve current state.
        let state_backend = config.state.unwrap_or_default();

        // Lock the state for the duration of the apply operation.
        let state_lock = state_backend.lock().await?;
        let (_, mut state) = state_backend.load().await?;

        // Sync remote detection state from plugins (using read) and merge into our state.
        let plugin_manager = PluginManager::new()?;
        let mut join_set = JoinSet::new();
        for (plugin, context) in context.iter() {
            let plugin_path = plugins_dir.join(plugin).with_extension("wasm");
            let plugin_manager = plugin_manager.clone();

            // Cheap clone of context
            let context = context.clone();

            join_set.spawn(async move {
                let (instance, mut store) = plugin_manager.load_plugin(plugin_path).await?;
                let mut results: PluginsDetections = HashMap::new();
                for (service_name, settings) in &context.services {
                    let mut service_detections = HashMap::new();

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
                                    "retrieving detection '{}' for service '{}': {}",
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

        // Prepare the diff configuration.
        let diff_config = DiffConfig::default();
        let stdout = io::stdout();
        let mut writer = io::BufWriter::new(stdout.lock());

        // Show diff and retrieve the changes to apply.
        let mut to_create: HashMap<String, Vec<(String, Vec<u8>)>> = HashMap::new();
        let mut to_update: HashMap<String, Vec<(String, Vec<u8>)>> = HashMap::new();
        let mut to_remove: HashMap<String, Vec<(String, Vec<u8>)>> = HashMap::new();

        for (_, detection_ctx) in context.iter() {
            for (svc_name, _) in detection_ctx.services.iter() {
                if let Some(svc_rules) = state.services.get(svc_name) {
                    let mut detection_keys: HashSet<&String> =
                        detection_ctx.detections.keys().collect();

                    for (rule, current_val) in svc_rules {
                        match detection_ctx.detections.get(rule) {
                            // Rule is in both context and state
                            Some(desired_bytes) => {
                                let desired: Value = serde_json::from_slice(desired_bytes)?;
                                if &desired != current_val {
                                    if !self.auto_approve {
                                        println!(
                                            "[~] {} will be updated on {}",
                                            MODIFY_STYLE.apply_to(rule),
                                            BOLD_STYLE.apply_to(svc_name)
                                        );
                                        diff_config.diff_json(
                                            &desired,
                                            current_val,
                                            &mut writer,
                                        )?;
                                        writer.flush()?;
                                    }
                                    to_update
                                        .entry(svc_name.clone())
                                        .or_default()
                                        .push((rule.clone(), desired_bytes.clone()));
                                }

                                detection_keys.remove(rule);
                            }
                            // Rule is not in the context but is in the state
                            None => {
                                if !self.auto_approve {
                                    println!(
                                        "[-] {} will be removed from {}",
                                        REMOVE_STYLE.apply_to(rule),
                                        BOLD_STYLE.apply_to(svc_name)
                                    );
                                }
                                // Add to the list of rules to remove.
                                to_remove
                                    .entry(svc_name.clone())
                                    .or_default()
                                    .push((rule.clone(), serde_json::to_vec(&current_val)?));
                                detection_keys.remove(rule);
                            }
                        }
                    }
                }
                // Check what remains in the detection context that is not in the state
                for (path, content) in &detection_ctx.detections {
                    if !state
                        .services
                        .get(svc_name.as_str())
                        .map(|rules| rules.contains_key(path))
                        .unwrap_or(false)
                    {
                        if !self.auto_approve {
                            println!(
                                "[+] {} will be created on {}",
                                ADD_STYLE.apply_to(&path),
                                BOLD_STYLE.apply_to(svc_name)
                            );
                        }
                        let desired = (path.clone(), content.clone());
                        to_create.entry(svc_name.clone()).or_default().push(desired);
                    }
                }
            }
        }

        // Prompt the user for approval
        if to_create.is_empty() & to_update.is_empty() & to_remove.is_empty() {
            tracing::info!("no changes detected.");
            state_backend.save(&mut state).await?;
            state_backend.unlock(state_lock).await?;
            return Ok(());
        } else if !self.auto_approve
            && !Confirm::new()
                .with_prompt("Apply these changes?")
                .default(false)
                .interact()?
        {
            state_backend.unlock(state_lock).await?;
            anyhow::bail!("action aborted");
        }

        // Apply changes
        let plugin_manager = PluginManager::new()?;
        for (plugin, context) in context.iter() {
            let plugin_path = plugins_dir.join(plugin).with_extension("wasm");
            let (instance, mut store) = plugin_manager.load_plugin(plugin_path).await?;

            // Cheap clone of context
            let context = context.clone();

            for (svc_name, settings) in context.services.iter() {
                // Apply detections creation.
                if let Some(to_create) = to_create.get(svc_name) {
                    for (path, desired) in to_create {
                        if let Err(e) = instance.create(&mut store, settings, desired).await {
                            tracing::warn!("failed to create {} on {}: {}", path, svc_name, e);
                        } else {
                            println!(
                                "{} created on {}",
                                ADD_STYLE.apply_to(path),
                                BOLD_STYLE.apply_to(&svc_name)
                            );
                        }

                        // Add the new rule to the state.
                        if let Some(rules) = state.services.get_mut(svc_name) {
                            rules.insert(path.clone(), serde_json::from_slice(desired)?);
                        }
                    }
                }

                // Apply detections updates.
                if let Some(to_update) = to_update.get(svc_name) {
                    for (path, desired) in to_update {
                        if let Err(e) = instance.update(&mut store, settings, desired).await {
                            tracing::warn!("failed to update {} on {}: {}", path, svc_name, e);
                        } else {
                            println!(
                                "{} updated on {}",
                                MODIFY_STYLE.apply_to(path),
                                BOLD_STYLE.apply_to(&svc_name)
                            );
                        }

                        // Update the rule in the state.
                        if let Some(rules) = state.services.get_mut(svc_name) {
                            rules.insert(path.clone(), serde_json::from_slice(desired)?);
                        }
                    }
                }

                // Apply detections removals.
                if let Some(to_remove) = to_remove.get(svc_name) {
                    for (path, content) in to_remove {
                        match instance.delete(&mut store, settings, content).await {
                            Ok(_) => {
                                if let Some(rules) = state.services.get_mut(svc_name) {
                                    rules.remove(path);
                                }
                                println!(
                                    "{} removed from {}",
                                    REMOVE_STYLE.apply_to(path),
                                    BOLD_STYLE.apply_to(&svc_name)
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "failed to remove {} from {}: {}",
                                    path,
                                    svc_name,
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }

        // Save the updated state then release the lock.
        state_backend.save(&mut state).await?;
        state_backend.unlock(state_lock).await?;

        Ok(())
    }
}
