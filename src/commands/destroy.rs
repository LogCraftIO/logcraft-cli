// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use logcraft_common::{
    configuration::{Environment, ProjectConfiguration, Service},
    plugins::manager::{PluginActions, PluginManager},
};
use std::collections::HashMap;
use tokio::task::JoinSet;

#[derive(Parser, Debug, Default)]
#[clap(
    about = "Remove deployed detection rules from remote systems",
    allow_hyphen_values = true
)]
pub struct DestroyCommand {
    /// Destroy from this environment
    pub env_id: Option<String>,

    /// Destroy from this service
    #[clap(short, long)]
    pub service_id: Option<String>,

    /// Skip interactive approval of rules destruction
    #[clap(long)]
    pub auto_approve: bool,
}

impl DestroyCommand {
    pub async fn run(self, config: &ProjectConfiguration) -> Result<()> {
        // Load all detections
        let mut state = config.state.load().await?;

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

        for plugin_id in services.keys() {
            let plugin_id = plugin_id.to_string();
            let plugin_manager = plugin_manager.clone();
            set.spawn(async move { plugin_manager.load_plugin(plugin_id).await });
        }

        // Call get schema and retrieve all detections
        while let Some(plugin) = set.join_next().await {
            let (instance, mut store) = plugin??;
            let meta = &instance.metadata;

            // Safe unwrap as we load plugins with detection HashMap.
            let services = services.get(&meta.name).unwrap();
            let mut has_diff = false;

            for svc in services {
                let service_config = serde_json::to_string(&svc.settings)?;
                if let Some(rules) = state.services.get(&svc.id) {
                    for rule_state in rules {
                        let requested_rule = serde_json::to_string(&rule_state.content)?;
                        if instance
                            .read(
                                &mut store,
                                &service_config,
                                &rule_state.name,
                                &requested_rule,
                            )
                            .await?
                            .is_some()
                        {
                            has_diff = true;
                            if !self.auto_approve {
                                println!(
                                    "[-] rule: `{}` will be deleted from `{}`",
                                    style(&rule_state.name).red(),
                                    &svc.id
                                )
                            }
                        }
                    }
                }
            }

            // Destroy rules
            if has_diff {
                if self.auto_approve
                    || Confirm::with_theme(&prompt_theme)
                        .with_prompt("Do you want to deploy these changes?")
                        .interact()?
                {
                    for svc in services {
                        let service_config = serde_json::to_string(&svc.settings)?;
                        if let Some(service) = state.services.get_mut(&svc.id) {
                            // Collect rules to avoid borrowing issues during iteration
                            let rules: Vec<_> = service.iter().cloned().collect();

                            for rule_state in rules {
                                let rule_content = serde_json::to_string(&rule_state.content)?;
                                match instance
                                    .delete(
                                        &mut store,
                                        &service_config,
                                        &rule_state.name,
                                        &rule_content,
                                    )
                                    .await
                                {
                                    Ok(Some(_)) => {
                                        println!(
                                            "[-] rule: `{}` deleted from `{}`",
                                            style(&rule_state.name).red(),
                                            svc.id
                                        );
                                        service.remove(&rule_state);
                                    }
                                    Ok(None) => {
                                        println!(
                                            "[!] rule: `{}` not found on `{}` - ignoring",
                                            style(&rule_state.name).dim(),
                                            svc.id
                                        );
                                        service.remove(&rule_state);
                                    }
                                    Err(e) => {
                                        state.save(&config.state).await?;
                                        bail!(
                                            "on deletion for `{}` in `{}`: {}",
                                            style(&rule_state.name).red(),
                                            svc.id,
                                            e
                                        );
                                    }
                                }
                            }
                            state.services.remove(&svc.id);
                        }
                    }
                } else {
                    bail!("action aborted")
                }
            } else {
                tracing::info!("no differences found");
                return Ok(());
            }
        }

        state.save(&config.state).await
    }
}
