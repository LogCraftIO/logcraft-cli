// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::{anyhow, bail, Ok, Result};
use clap::{Parser, Subcommand};
use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use logcraft_common::{
    configuration::{Environment, ProjectConfiguration},
    utils::ensure_kebab_case,
};

/// Manage environments
#[derive(Subcommand)]
pub enum EnvironmentsCommands {
    /// Add a new environment
    Add(AddEnvironment),

    /// List environments
    List(ListEnvironments),

    /// Remove environment
    Remove(RemoveEnvironment),

    /// Link service
    Link(LinkEnvironment),

    /// Unlink service
    Unlink(UnlinkEnvironment),
}

impl EnvironmentsCommands {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        match self {
            Self::Add(cmd) => cmd.run(config).await,
            Self::List(cmd) => cmd.run(config),
            Self::Remove(cmd) => cmd.run(config).await,
            Self::Link(cmd) => cmd.run(config).await,
            Self::Unlink(cmd) => cmd.run(config).await,
        }
    }
}

#[derive(Parser)]
pub struct AddEnvironment {
    /// ID of the environment to create
    pub id: Option<String>,
}

impl AddEnvironment {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        // Prompt theme
        let prompt_theme = ColorfulTheme::default();

        // Prompt id if not set
        let id = match self.id {
            Some(id) => id,
            None => Input::<String>::with_theme(&prompt_theme)
                .with_prompt("Environment id:")
                .interact_text()?,
        };

        // Naming contraints check
        let id = ensure_kebab_case(&id)?.to_string();

        // Add new environment if it does not exists
        let env = Environment {
            id,
            ..Default::default()
        };

        if config.environments.contains(&env) {
            bail!("environment `{}` already exists", &env.id)
        } else {
            config.environments.insert(env);
        }

        config.save_config(None)
    }
}

#[derive(Parser)]
pub struct ListEnvironments;

impl ListEnvironments {
    pub fn run(self, config: &ProjectConfiguration) -> Result<()> {
        // Retrieve configuration
        for plugin in &config.environments {
            println!(
                "`{}` {} service(s)",
                style(&plugin.id).bold(),
                plugin.services.len()
            );
        }

        Ok(())
    }
}

#[derive(Parser)]
pub struct RemoveEnvironment {
    /// ID of the environment to remove
    pub id: Option<String>,
}

impl RemoveEnvironment {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        if config.environments.is_empty() {
            bail!("no environments defined")
        }

        // Prompt theme
        let prompt_theme = ColorfulTheme::default();

        // Prompt id if not set
        let id = match self.id {
            Some(id) => id,
            None => {
                let environment = config.environment_ids()?;
                let selection = Select::with_theme(&prompt_theme)
                    .with_prompt("Select the environment to remove:")
                    .items(&environment)
                    .default(0)
                    .interact()?;
                environment[selection].to_string()
            }
        };

        // Because hash is computed from id,
        // best method discovered to prevent immutable borrow to get &Plugin for deletion
        let fake = Environment {
            id,
            ..Default::default()
        };

        if !config.environments.remove(&fake) {
            bail!("environment `{}` does not exists", &fake.id)
        };

        config.save_config(None)
    }
}

#[derive(Parser)]
pub struct LinkEnvironment {
    /// ID of the environment
    pub env_id: Option<String>,

    /// ID of the service to link to this environment
    #[clap(short, long)]
    pub service_id: Option<String>,
}

impl LinkEnvironment {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        if config.environments.is_empty() {
            bail!("no environments defined")
        }

        // Prompt theme
        let prompt_theme = ColorfulTheme::default();

        // Prompt id if not set
        let id = match self.env_id {
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

        // Retrieve environment
        let mut env = config
            .environments
            .get(&Environment {
                id: id.clone(),
                ..Default::default()
            })
            .ok_or_else(|| anyhow!("environment `{}` does not exist", &id))?
            .clone();

        // Retrieve selected services
        let services: Vec<&str> = config.service_ids()?;
        let service_id = match &self.service_id {
            Some(id) => {
                if !services.contains(&id.as_str()) {
                    bail!("service `{}` does not exist", &id)
                }
                id.to_string()
            }
            None => {
                let env_services: Vec<_> = services
                    .iter()
                    .filter(|&&svc_id| !env.services.contains(svc_id))
                    .cloned()
                    .collect();

                if env_services.is_empty() {
                    bail!("no available service to link to this environment")
                }

                let selection = Select::with_theme(&prompt_theme)
                    .with_prompt("Service to link:")
                    .items(&env_services)
                    .default(0)
                    .interact()?;
                env_services[selection].to_string()
            }
        };

        env.services.insert(service_id.to_string());
        config.environments.replace(env);
        tracing::info!("service `{}` linked to environement `{}`", service_id, id);
        config.save_config(None)
    }
}

#[derive(Parser)]
pub struct UnlinkEnvironment {
    /// ID of the environment
    pub env_id: Option<String>,

    /// ID of the service to unlink from this environment
    #[clap(short, long)]
    pub service_id: Option<String>,
}

impl UnlinkEnvironment {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        if config.environments.is_empty() {
            bail!("no environments defined")
        }

        // Prompt theme
        let prompt_theme = ColorfulTheme::default();

        // Prompt id if not set
        let id = match self.env_id {
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

        // Retrieve environment
        let mut env = config
            .environments
            .get(&Environment {
                id: id.clone(),
                ..Default::default()
            })
            .ok_or_else(|| anyhow!("environment `{}` does not exist", &id))?
            .clone();

        // Retrieve selected services
        let services: Vec<&str> = config.service_ids()?;
        let service_id = match self.service_id {
            Some(id) => {
                if !env.services.contains(&id) {
                    bail!(
                        "service `{}` is not linked to environment `{}`",
                        &id,
                        &env.id
                    )
                }
                id
            }
            None => {
                let env_services: Vec<_> = services
                    .iter()
                    .filter(|&&svc_id| env.services.contains(svc_id))
                    .cloned()
                    .collect();

                if env_services.is_empty() {
                    bail!("no available service to unlink from this environment")
                }

                let selection = Select::with_theme(&prompt_theme)
                    .with_prompt("Service to link:")
                    .items(&env_services)
                    .default(0)
                    .interact()?;
                env_services[selection].to_string()
            }
        };

        env.services.remove(&service_id);
        config.environments.replace(env);
        tracing::info!(
            "service `{}` unlinked from environement `{}`",
            service_id,
            id
        );
        config.save_config(None)
    }
}
