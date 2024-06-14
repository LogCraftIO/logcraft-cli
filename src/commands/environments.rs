// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::{anyhow, bail, Ok, Result};
use clap::{Parser, Subcommand};
use inquire::{Select, Text};
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
    /// Name of the environment to create
    pub name: Option<String>,
}

impl AddEnvironment {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        // Prompt name if not set
        let name = match self.name {
            Some(name) => name,
            None => Text::new("Environment name:").prompt()?,
        };

        // Naming contraints check
        ensure_kebab_case(&name)?;

        // Add new environment if it does not exists
        let env = Environment {
            name,
            ..Default::default()
        };

        if config.environments.contains(&env) {
            bail!("error: environment `{}` already exists", &env.name)
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
            println!("`{}` `{}` service(s)", &plugin.name, plugin.services.len());
        }

        Ok(())
    }
}

#[derive(Parser)]
pub struct RemoveEnvironment {
    /// Name of the environment to remove
    pub name: Option<String>,
}

impl RemoveEnvironment {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        if config.environments.is_empty() {
            bail!("no environments defined")
        }

        // Prompt name if not set
        let name = match self.name {
            Some(name) => name,
            None => Select::new(
                "Select the project to uninstall:",
                config.environment_names()?,
            )
            .prompt()?
            .to_owned(),
        };

        // Because hash is computed from name,
        // best method discovered to prevent immutable borrow to get &Plugin for deletion
        let fake = Environment {
            name,
            ..Default::default()
        };

        if !config.environments.remove(&fake) {
            bail!("environment `{}` does not exists", &fake.name)
        };

        config.save_config(None)
    }
}

#[derive(Parser)]
pub struct LinkEnvironment {
    /// Name of the environment
    #[clap(short, long)]
    pub env_name: Option<String>,

    /// Name of the service to link to this environment
    #[clap(short, long)]
    pub service_name: Option<String>,
}

impl LinkEnvironment {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        if config.environments.is_empty() {
            bail!("no environments defined")
        }

        // Prompt name if not set
        let name = self.env_name.unwrap_or_else(|| {
            Select::new(
                "Select the environment:",
                config.environment_names().unwrap(),
            )
            .prompt()
            .unwrap()
            .to_owned()
        });

        // Retrieve environment
        let mut env = config
            .environments
            .get(&Environment {
                name: name.clone(),
                ..Default::default()
            })
            .ok_or_else(|| anyhow!("environment `{}` does not exist", &name))?
            .clone();

        // Retrieve selected services
        let services: Vec<String> = config.service_names()?;
        let service_name = match self.service_name {
            Some(name) => {
                if !services.contains(&name) {
                    bail!("service `{}` does not exist", &name)
                }
                name
            }
            None => {
                let env_services: Vec<_> = services
                    .iter()
                    .filter(|&svc_name| !env.services.contains(svc_name))
                    .cloned()
                    .collect();

                if env_services.is_empty() {
                    bail!("no available service to link to this environment")
                }

                Select::new("Service to link:", env_services).prompt()?
            }
        };

        env.services.insert(service_name.to_string());
        config.environments.replace(env);
        println!(
            "service `{}` linked to environement `{}`",
            service_name, name
        );
        config.save_config(None)
    }
}

#[derive(Parser)]
pub struct UnlinkEnvironment {
    /// Name of the environment
    #[clap(short, long)]
    pub env_name: Option<String>,

    /// Name of the service to unlink from this environment
    #[clap(short, long)]
    pub service_name: Option<String>,
}

impl UnlinkEnvironment {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        if config.environments.is_empty() {
            bail!("no environments defined")
        }

        // Prompt name if not set
        let name = self.env_name.unwrap_or_else(|| {
            Select::new(
                "Select the environment:",
                config.environment_names().unwrap(),
            )
            .prompt()
            .unwrap()
            .to_owned()
        });

        // Retrieve environment
        let mut env = config
            .environments
            .get(&Environment {
                name: name.clone(),
                ..Default::default()
            })
            .ok_or_else(|| anyhow!("environment `{}` does not exist", &name))?
            .clone();

        // Retrieve selected services
        let services: Vec<String> = config.service_names()?;
        let service_name = match self.service_name {
            Some(name) => {
                if !env.services.contains(&name) {
                    bail!(
                        "service `{}` is not linked to environment `{}`",
                        &name,
                        &env.name
                    )
                }
                name
            }
            None => {
                let env_services: Vec<_> = services
                    .iter()
                    .filter(|&svc_name| env.services.contains(svc_name))
                    .cloned()
                    .collect();

                if env_services.is_empty() {
                    bail!("no available service to unlink from this environment")
                }

                Select::new("Service to unlink:", env_services).prompt()?
            }
        };

        env.services.remove(&service_name);
        config.environments.replace(env);
        println!(
            "service `{}` unlinked from environement `{}`",
            service_name, name
        );
        config.save_config(None)
    }
}
