// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use console::style;
use inquire::Select;
use logcraft_common::{
    configuration::{Environment, ProjectConfiguration, Service},
    detections::map_plugin_detections,
    plugins::manager::{PluginActions, PluginManager},
};
use tokio::task::JoinSet;

/// Prepare working directory for other lgcli commands
#[derive(Parser, Debug, Default)]
#[clap(
    about = "Show changes between local and remote detection rules",
    allow_hyphen_values = true
)]
pub struct DiffCommand {
    /// Show differences from this target environment
    #[clap(short, long)]
    pub env_name: Option<String>,

    /// Show differences from this target service
    #[clap(short, long, conflicts_with = "env_name")]
    pub service_name: Option<String>,
}

impl DiffCommand {
    pub async fn run(self, config: &ProjectConfiguration) -> Result<()> {
        // Load all detections
        let detections = map_plugin_detections()?;
        if detections.is_empty() {
            bail!("no detections found");
        }

        // Retrieve services depending on targeted environment or service
        let services = if let Some(service_name) = self.service_name {
            let service = config
                .services
                .get(&Service {
                    name: service_name.clone(),
                    ..Default::default()
                })
                .ok_or_else(|| anyhow!("service `{}` not found", &service_name))?;
            vec![service]
        } else {
            // Prompt if env_name and service_name not set
            let name = self.env_name.unwrap_or_else(|| {
                Select::new(
                    "Select the environment to use:",
                    config.environment_names().unwrap(),
                )
                .prompt()
                .unwrap()
                .to_owned()
            });

            let env = config
                .environments
                .get(&Environment {
                    name: name.clone(),
                    ..Default::default()
                })
                .ok_or_else(|| anyhow!("environement `{}` not found", &name))?;

            // Retrieve environment services
            config
                .services
                .iter()
                .filter(|svc| env.services.contains(&svc.name))
                .collect()
        };

        // Load plugins
        let plugin_manager = PluginManager::new()?;
        let mut set = JoinSet::new();

        for plugin_name in detections.keys() {
            let plugin_name = plugin_name.to_string();
            let plugin_manager = plugin_manager.clone();
            set.spawn(async move { plugin_manager.load_plugin(plugin_name).await });
        }

        // Call get schema and retrieve all detections
        while let Some(plugin) = set.join_next().await {
            let (instance, mut store) = plugin??;
            let meta = &instance.metadata;

            // Safe unwrap as we load plugins with detection HashMap.
            let (plugin, rules) = detections.get_key_value(&meta.name).unwrap();

            for svc in services.iter().filter(|svc| &svc.plugin == plugin) {
                let service_config = serde_json::to_string(&svc.settings)?;
                for (rule_name, rule) in rules {
                    let rule = serde_json::to_string(rule)?;
                    match instance
                        .read(&mut store, &service_config, rule_name, &rule)
                        .await?
                    {
                        Some(res) => {
                            println!("Res: {}", &res);
                            if res.is_empty() {
                                println!(
                                    "rule: `{}` will be unchanged on `{}`",
                                    style(&rule_name).dim(),
                                    svc.name
                                );
                            } else {
                                println!(
                                    "rule: `{}` will be updated on `{}`",
                                    style(&rule_name).yellow(),
                                    svc.name
                                );
                            }
                        }
                        None => println!(
                            "rule: `{}` will be created on `{}`",
                            style(&rule_name).green(),
                            svc.name
                        ),
                    }
                }
            }
        }

        Ok(())
    }
}
