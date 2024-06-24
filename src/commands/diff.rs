// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::{anyhow, Result};
use clap::Parser;
use console::style;
use dialoguer::{theme::ColorfulTheme, Select};
use logcraft_common::{
    configuration::{Environment, ProjectConfiguration, Service},
    detections::{
        compare_detections, map_plugin_detections, DetectionState, PluginDetections,
        ServiceDetections,
    },
    plugins::manager::{PluginActions, PluginManager},
    state::State,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use tokio::task::JoinSet;

/// Prepare working directory for other lgcli commands
#[derive(Parser, Debug, Default)]
#[clap(
    about = "Show changes between local and remote detection rules",
    allow_hyphen_values = true
)]
pub struct DiffCommand {
    /// Show differences from this target environment
    pub env_id: Option<String>,

    /// Show differences from this target service
    #[clap(short, long)]
    pub service_id: Option<String>,
}

impl DiffCommand {
    pub async fn run(self, config: &ProjectConfiguration) -> Result<()> {
        // Load all detections
        let detections: PluginDetections = map_plugin_detections()?;

        // Prompt theme
        let prompt_theme = ColorfulTheme::default();

        // Retrieve services
        let mut services: HashMap<String, Vec<&Service>> = HashMap::new();
        if let Some(svc_id) = self.service_id {
            let svc = config
                .services
                .get(&Service {
                    id: svc_id.clone(),
                    ..Default::default()
                })
                .ok_or_else(|| anyhow!("service `{}` not found", &svc_id))?;

            services.insert(svc.plugin.clone(), vec![svc]);
        } else {
            let env_id = match self.env_id {
                Some(id) => id,
                // None => Select::new("Select the environment to use:", config.service_ids()?).prompt()?
                None => {
                    let environment = config.environment_ids()?;
                    let selection = Select::with_theme(&prompt_theme)
                        .with_prompt("Select the environment:")
                        .items(&environment)
                        .default(0)
                        .interact()?;
                    environment[selection].to_string()
                }
            };

            let env = config
                .environments
                .get(&Environment {
                    id: env_id.clone(),
                    ..Default::default()
                })
                .ok_or_else(|| anyhow!("environment `{}` not found", &env_id))?;

            config
                .services
                .iter()
                .filter(|svc| env.services.contains(&svc.id))
                .for_each(|svc| {
                    services.entry(svc.plugin.clone()).or_default().push(svc);
                })
        };

        // Load plugins
        let plugin_manager = PluginManager::new()?;
        let mut set = JoinSet::new();

        for plugin_id in detections.keys() {
            let plugin_id = plugin_id.to_string();
            let plugin_manager = plugin_manager.clone();
            set.spawn(async move { plugin_manager.load_plugin(plugin_id).await });
        }

        let mut returned_rules: ServiceDetections = HashMap::new();
        let mut has_diff = false;

        // Call get schema and retrieve all detections
        while let Some(plugin) = set.join_next().await {
            let (instance, mut store) = plugin??;
            let meta = &instance.metadata;

            // Safe unwrap as we load plugins with detection HashMap.
            let (plugin, rules) = detections.get_key_value(&meta.name).unwrap();

            if let Some(services) = services.get(plugin) {
                for svc in services {
                    let service_config = serde_json::to_string(&svc.settings)?;
                    for rule_state in rules {
                        let requested_rule = serde_json::to_string(&rule_state.content)?;
                        if let Some(rule) = instance
                            .read(
                                &mut store,
                                &service_config,
                                &rule_state.name,
                                &requested_rule,
                            )
                            .await?
                        {
                            let content: Value = serde_json::from_str(&rule)?;
                            returned_rules
                                .entry(svc.id.clone())
                                .and_modify(|rules| {
                                    rules.insert(DetectionState {
                                        name: rule_state.name.clone(),
                                        content: content.clone(),
                                    });
                                })
                                .or_insert(HashSet::from([DetectionState {
                                    name: rule_state.name.clone(),
                                    content,
                                }]));
                        } else {
                            has_diff = true;
                            println!(
                                "[+] rule: `{}` will be created on `{}`",
                                style(&rule_state.name).green(),
                                &svc.id
                            )
                        }
                    }
                }
            }
        }

        let changes = compare_detections(&detections, &returned_rules, &services, true).is_empty();

        if State::read()?.missing_rules(&returned_rules).is_empty() && changes && !has_diff {
            tracing::info!("no differences found");
        }

        Ok(())
    }
}
