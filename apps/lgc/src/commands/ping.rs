// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use std::{collections::HashMap, path, time::Duration};

use lgc_common::{
    configuration::{self, LGC_BASE_DIR},
    diff::BOLD_STYLE,
    plugins::manager::{PluginActions, PluginManager},
};
use tokio::task::JoinSet;

/// Validate services network connectivity
#[derive(clap::Parser)]
#[clap(
    about = "Validate services network connectivity",
    allow_hyphen_values = true
)]
pub struct PingCommand {
    /// Service/Environment identifier (optional)
    pub identifier: Option<String>,
}

impl PingCommand {
    /// Run the ping command.
    pub async fn run(self, config: configuration::ProjectConfiguration) -> anyhow::Result<()> {
        if config.services.is_empty() {
            anyhow::bail!("no services defined");
        }

        // Prepare services to ping.
        let mut services: HashMap<String, Vec<(String, Vec<u8>)>> = HashMap::new();

        // Retrieve services from the configuration.
        match self.identifier {
            Some(identifier) => {
                if let Some((name, configuration)) = config.services.get_key_value(&identifier) {
                    let settings = serde_json::to_vec(&configuration.settings)?;
                    services
                        .entry(configuration.plugin.clone())
                        .or_default()
                        .push((name.clone(), settings));
                } else {
                    let environment_services = config.environment_services(&identifier);
                    if environment_services.is_empty() {
                        anyhow::bail!("invalid identifier '{}'.", identifier);
                    } else {
                        for (name, configuration) in environment_services {
                            let settings = serde_json::to_vec(&configuration.settings)?;
                            services
                                .entry(configuration.plugin.clone())
                                .or_default()
                                .push((name.clone(), settings));
                        }
                    }
                }
            }
            None => {
                for (name, configuration) in config.services.iter() {
                    let settings = serde_json::to_vec(&configuration.settings)
                        .expect("serialization should succeed");
                    services
                        .entry(configuration.plugin.clone())
                        .or_default()
                        .push((name.clone(), settings));
                }
            }
        };

        // Prepare the plugin engine and a JoinSet to concurrently ping services.
        let plugin_manager = PluginManager::new()?;
        let mut join_set = JoinSet::new();

        // Retrieve plugin directory and prepare the root plugin path.
        let plugins_dir =
            path::PathBuf::from(config.core.base_dir.as_deref().unwrap_or(LGC_BASE_DIR))
                .join("plugins");

        // For each service, spawn a separate task.
        for (plugin, service_list) in services {
            let plugin_path = plugins_dir.join(&plugin).with_extension("wasm");
            if !plugin_path.exists() {
                tracing::warn!(
                    "ignoring '{}/{}' (no matching plugin).",
                    config.core.workspace,
                    plugin
                );
                continue;
            }

            for (service_name, settings) in service_list {
                let plugin_manager = plugin_manager.clone();
                let plugin_path = plugin_path.clone();
                join_set.spawn(async move {
                    tracing::info!("checking {}", BOLD_STYLE.apply_to(&service_name));

                    // Create a new instance of the plugin and ping the service.
                    let (instance, mut store) = plugin_manager.load_plugin(plugin_path).await?;
                    let ping_future = instance.ping(&mut store, &settings);
                    tokio::pin!(ping_future);

                    // Set the interval duration.
                    let mut interval = tokio::time::interval(Duration::from_secs(10));
                    let start = tokio::time::Instant::now();

                    // Loop until the ping completes.
                    let ping_result = loop {
                        tokio::select! {
                            result = &mut ping_future => {
                                break result;
                            }
                            _ = interval.tick() => {
                                if start.elapsed().as_secs() > 0 {
                                    tracing::info!(
                                        "waiting for {} [{}s elapsed]",
                                        BOLD_STYLE.apply_to(&service_name),
                                        start.elapsed().as_secs()
                                    );
                                }
                            }
                        }
                    };

                    // Handle the result.
                    match ping_result {
                        Ok(_) => {
                            tracing::info!(
                                "connection with {} successful",
                                BOLD_STYLE.apply_to(&service_name)
                            );
                            Ok(())
                        }
                        Err(e) => Err(anyhow::anyhow!(
                            "unable to contact {}: {}",
                            BOLD_STYLE.apply_to(&service_name),
                            e
                        )),
                    }
                });
            }
        }

        // Wait for all tasks to finish.
        while let Some(result) = join_set.join_next().await {
            // Propagate errors from any plugin ping instantiation.
            result??;
        }

        Ok(())
    }
}
