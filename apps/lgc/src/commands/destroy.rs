// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use dialoguer::Confirm;
use lgc_common::{
    configuration::{self, DetectionContext, LGC_BASE_DIR},
    diff::{BOLD_STYLE, REMOVE_STYLE},
    plugins::manager::{PluginActions, PluginManager},
};
use std::{collections::HashMap, path};
use tokio::task::JoinSet;

#[derive(clap::Parser)]
#[clap(
    about = "Removes detections managed by lgc",
    allow_hyphen_values = true
)]
pub struct DestroyCommand {
    /// Service identifier
    pub identifier: Option<String>,

    /// Skip interactive approval of plan before destroying.
    #[clap(short, long)]
    pub auto_approve: bool,
}

impl DestroyCommand {
    pub async fn run(self, mut config: configuration::ProjectConfiguration) -> anyhow::Result<()> {
        // Retrieve current state.
        let state_backend = config.state.take().unwrap_or_default();
        let state_lock = state_backend.lock().await?;
        let (_, mut state) = state_backend.load().await?;

        // Build a map of detections per plugin.
        let mut detections: HashMap<String, DetectionContext> = HashMap::new();
        match self.identifier {
            Some(identifier) => {
                if let Some(service) = config.services.get(&identifier) {
                    let settings_bytes = serde_json::to_vec(&service.settings)?;
                    if let Some(service_detections) = state
                        .take_serialized_detections(&identifier)?
                        .filter(|d| !d.is_empty())
                    {
                        detections.insert(
                            service.plugin.clone(), // unavoidable clone since `get` returns a reference.
                            DetectionContext {
                                services: vec![(identifier, settings_bytes)],
                                detections: service_detections,
                            },
                        );
                    } else {
                        tracing::info!("no changes detected.");
                        state_backend.save(&mut state).await?;
                        state_backend.unlock(state_lock).await?;
                        return Ok(());
                    }
                } else {
                    // Process environment services associated with the identifier.
                    for (name, service) in config.environment_services(&identifier) {
                        let settings_bytes = serde_json::to_vec(&service.settings)?;
                        if let Some(service_detections) = state
                            .take_serialized_detections(&name)?
                            .filter(|d| !d.is_empty())
                        {
                            // When the service is borrowed, we still need to clone the plugin name.
                            detections
                                .entry(service.plugin.clone())
                                .and_modify(|ctx| {
                                    ctx.services.push((name.clone(), settings_bytes.clone()));
                                    ctx.detections.extend(service_detections.clone());
                                })
                                .or_insert(DetectionContext {
                                    services: vec![(name, settings_bytes)],
                                    detections: service_detections,
                                });
                        }
                    }
                }
            }
            None => {
                // Process all services.
                for (name, service) in config.services.into_iter() {
                    let settings_bytes = serde_json::to_vec(&service.settings)?;
                    if let Some(service_detections) = state
                        .take_serialized_detections(&name)?
                        .filter(|d| !d.is_empty())
                    {
                        detections
                            .entry(service.plugin)
                            .and_modify(|ctx| {
                                ctx.services.push((name.clone(), settings_bytes.clone()));
                                ctx.detections.extend(service_detections.clone());
                            })
                            .or_insert(DetectionContext {
                                services: vec![(name, settings_bytes)],
                                detections: service_detections,
                            });
                    }
                }
            }
        }

        // Retrieve plugin directory
        let plugins_dir =
            path::PathBuf::from(config.core.base_dir.as_deref().unwrap_or(LGC_BASE_DIR))
                .join("plugins");

        // Sync remote detection state.
        let plugin_manager = PluginManager::new()?;
        let mut join_set = JoinSet::new();
        for (plugin, plugin_context) in detections {
            // Check if the plugin exists.
            let plugin_path = plugins_dir.join(&plugin).with_extension("wasm");
            if !plugin_path.exists() {
                tracing::warn!(
                    "folder `{}/{}` has no plugin associated.",
                    config.core.workspace,
                    plugin
                );
                // Proceed to the next plugin.
                continue;
            }

            let plugin_manager = plugin_manager.clone();
            join_set.spawn(async move {
                let (instance, mut store) = plugin_manager.load_plugin(plugin_path).await?;
                let mut results = Vec::new();
                for (service_name, settings) in plugin_context.services {
                    let mut service_detections = Vec::new();
                    // We iterate over detections by reference since they're shared for all services.
                    for (path, content) in &plugin_context.detections {
                        match instance.read(&mut store, &settings, content).await {
                            Ok(Some(res)) => {
                                service_detections.push((path.clone(), settings.clone(), res));
                            }
                            Ok(None) => {}
                            Err(e) => {
                                anyhow::bail!(
                                    "retrieving detection '{}' for service `{}`: {}",
                                    path,
                                    service_name,
                                    e
                                )
                            }
                        }
                    }
                    if !service_detections.is_empty() {
                        results.push((service_name, service_detections));
                    }
                }
                Ok((plugin, results))
            });
        }

        // Merge the plugin detections into the state.
        let mut to_remove = HashMap::new();

        while let Some(res) = join_set.join_next().await {
            let (plugin, services) = res??;
            if !self.auto_approve {
                for (service_name, rules) in &services {
                    for (path, _, _) in rules {
                        println!(
                            "[-] `{}` will be removed from service `{}`",
                            REMOVE_STYLE.apply_to(path),
                            BOLD_STYLE.apply_to(service_name)
                        );
                    }
                }
            }
            to_remove.insert(plugin, services);
        }

        // Prompt the user for approval.
        if to_remove.is_empty() {
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
            state_backend.save(&mut state).await?;
            state_backend.unlock(state_lock).await?;
            anyhow::bail!("action aborted");
        }

        // Apply changes.
        let plugin_manager = PluginManager::new()?;
        for (plugin, services) in to_remove {
            let plugin_path = plugins_dir.join(&plugin).with_extension("wasm");
            let (instance, mut store) = plugin_manager.load_plugin(plugin_path).await?;
            for (service_name, rules) in services {
                for (path, settings, content) in rules {
                    match instance.delete(&mut store, &settings, &content).await {
                        Ok(_) => {
                            if let Some(rules) = state.services.get_mut(&service_name) {
                                rules.remove(&path);
                            }
                            println!(
                                "`{}` removed from service `{}`",
                                REMOVE_STYLE.apply_to(&path),
                                BOLD_STYLE.apply_to(&service_name)
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                "failed to delete rule `{}` on service `{}`: {}",
                                path,
                                service_name,
                                e
                            );
                        }
                    }
                }
            }
        }

        // Save updated state and release the lock.
        state_backend.save(&mut state).await?;
        state_backend.unlock(state_lock).await?;

        Ok(())
    }
}
