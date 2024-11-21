// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use lgc_common::{
    configuration::{Environment, ProjectConfiguration, Service},
    detections::{compare_detections, map_plugin_detections, DetectionState, ServiceDetections},
    plugins::manager::{PluginActions, PluginManager},
};
use serde_json::Value;
use tokio::task::JoinSet;

/// Prepare working directory for other lgcli commands
#[derive(Parser, Debug, Default)]
#[clap(
    about = "Deploy rules changes to remote systems",
    allow_hyphen_values = true
)]
pub struct DeployCommand {
    /// Deploy to this target environment
    pub env_id: Option<String>,

    /// Deploy to this target service
    #[clap(short, long)]
    pub service_id: Option<String>,

    /// Show differences for this detection path
    #[clap(short, long)]
    pub detection_id: Option<String>,

    /// Skip interactive approval of changes deployment
    #[clap(long)]
    pub auto_approve: bool,
}

impl DeployCommand {
    pub async fn run(self, config: &ProjectConfiguration) -> Result<()> {
        // Load all detections
        let detections = map_plugin_detections(self.detection_id.clone())?;

        // Prompt theme
        let prompt_theme = ColorfulTheme::default();

        // Retrieve services depending on targeted environment or service
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

        // Call get schema and retrieve all detections
        while let Some(plugin) = set.join_next().await {
            let (instance, mut store) = plugin??;
            let meta = &instance.metadata;

            // Safe unwrap as we load plugins with detection HashMap.
            let (plugin, rules) = detections.get_key_value(&meta.name).unwrap();

            let mut has_diff = false;
            if let Some(plugin_services) = services.get(plugin) {
                let mut returned_rules: ServiceDetections = HashMap::new();
                let mut missing_rules: HashMap<String, HashSet<&DetectionState>> = HashMap::new();

                for svc in plugin_services {
                    let service_config = serde_json::to_string(&svc.settings)?;
                    for rule in rules {
                        let requested_rule = serde_json::to_string(&rule.content)?;
                        if let Some(resp) = instance
                            .read(&mut store, &service_config, &rule.name, &requested_rule)
                            .await?
                        {
                            let content: Value = serde_json::from_str(&resp)?;
                            returned_rules
                                .entry(svc.id.clone())
                                .and_modify(|rules| {
                                    rules.insert(DetectionState {
                                        name: rule.name.clone(),
                                        content: content.clone(),
                                    });
                                })
                                .or_insert(HashSet::from([DetectionState {
                                    name: rule.name.clone(),
                                    content,
                                }]));
                        } else {
                            has_diff = true;
                            missing_rules
                                .entry(svc.id.clone())
                                .and_modify(|rules| {
                                    rules.insert(rule);
                                })
                                .or_insert(HashSet::from([rule]));
                            if !self.auto_approve {
                                println!(
                                    "[+] rule: `{}` will be created on `{}`",
                                    style(&rule.name).green(),
                                    &svc.id
                                )
                            }
                        }
                    }
                }

                let mut state = config.state.load().await?;
                let to_remove = state.missing_rules(
                    &returned_rules,
                    self.auto_approve,
                    self.detection_id.clone(),
                );
                let changed =
                    compare_detections(&detections, &returned_rules, &services, !self.auto_approve);

                if !changed.is_empty() || has_diff || !to_remove.is_empty() {
                    if self.auto_approve
                        || Confirm::with_theme(&prompt_theme)
                            .with_prompt("Do you want to deploy these changes?")
                            .interact()?
                    {
                        for svc in plugin_services {
                            let service_config = serde_json::to_string(&svc.settings)?;
                            let state_service = state.services.entry(svc.id.clone()).or_default();

                            // Create
                            if let Some(missing_rules) = missing_rules.get(&svc.id) {
                                for &rule in missing_rules {
                                    let rule_content = serde_json::to_string(&rule.content)?;
                                    match instance
                                        .create(
                                            &mut store,
                                            &service_config,
                                            &rule.name,
                                            &rule_content,
                                        )
                                        .await
                                    {
                                        Ok(_) => {
                                            state_service.insert(rule.clone());
                                            println!(
                                                "[+] rule: `{}` created on `{}`",
                                                style(&rule.name).green(),
                                                svc.id
                                            )
                                        }
                                        Err(e) => {
                                            state.save(&config.state).await?;
                                            bail!(
                                                "on update for `{}` in `{}`: {}",
                                                style(&rule.name).red(),
                                                svc.id,
                                                e
                                            );
                                        }
                                    }
                                }
                            }

                            // Update
                            if let Some(changed_rules) = changed.get(&svc.id) {
                                for rule in rules.intersection(changed_rules) {
                                    let rule_content = serde_json::to_string(&rule.content)?;
                                    match instance
                                        .update(
                                            &mut store,
                                            &service_config,
                                            &rule.name,
                                            &rule_content,
                                        )
                                        .await
                                    {
                                        Ok(_) => {
                                            state_service.replace(rule.clone());
                                            println!(
                                                "[~] rule: `{}` updated on `{}`",
                                                style(&rule.name).yellow(),
                                                svc.id
                                            )
                                        }
                                        Err(e) => {
                                            state.save(&config.state).await?;
                                            bail!(
                                                "on update for `{}` in `{}`: {}",
                                                style(&rule.name).red(),
                                                svc.id,
                                                e
                                            );
                                        }
                                    }
                                }
                            }

                            // Delete
                            if let Some(rules) = to_remove.get(&svc.id) {
                                for rule in rules {
                                    let rule_content = serde_json::to_string(&rule.content)?;
                                    match instance
                                        .delete(
                                            &mut store,
                                            &service_config,
                                            &rule.name,
                                            &rule_content,
                                        )
                                        .await
                                    {
                                        Ok(_) => {
                                            state_service.remove(rule);
                                            println!(
                                                "[-] rule: `{}` deleted from `{}`",
                                                style(&rule.name).red(),
                                                svc.id
                                            );
                                        }
                                        Err(e) => {
                                            state.save(&config.state).await?;
                                            bail!(
                                                "on deletion for `{}` in `{}`: {}",
                                                style(&rule.name).red(),
                                                svc.id,
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        state.save(&config.state).await?;
                    } else {
                        bail!("action aborted")
                    }
                } else {
                    // Update state to include any missing rules detected
                    if returned_rules
                        .iter()
                        .any(|(k, v)| state.services.get(k) != Some(v))
                    {
                        tracing::info!("including unchanged remote detection rules that are not currently referenced in state");
                        state.services.extend(returned_rules);
                        state.save(&config.state).await?;
                    }

                    tracing::info!("no differences found");
                }
            }
        }
        Ok(())
    }
}
