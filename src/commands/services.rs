// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::{anyhow, bail, Result};
use clap::{Parser, Subcommand};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use logcraft_common::{
    configuration::{ProjectConfiguration, Service},
    plugins::manager::{PluginActions, PluginManager},
    utils,
};
use std::{collections::HashMap, time::Duration};
use tokio::task::JoinSet;

/// Manage backend services
#[derive(Subcommand)]
pub enum ServicesCommands {
    /// Create a new service
    Add(AddService),

    /// List services
    List(ListServices),

    /// Remove a service
    Remove(RemoveService),

    /// Configure a service
    Configure(ConfigureService),

    /// Validate network connectivity to services
    Ping(PingService),
}

impl ServicesCommands {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        match self {
            Self::Add(cmd) => cmd.run(config).await,
            Self::List(cmd) => cmd.run(config),
            Self::Remove(cmd) => cmd.run(config),
            Self::Configure(cmd) => cmd.run(config).await,
            Self::Ping(cmd) => cmd.run(config).await,
        }
    }
}

#[derive(Parser)]
pub struct AddService {
    /// ID of the service to create
    pub id: Option<String>,

    /// Name of the plugin used by this service
    #[clap(short, long)]
    pub plugin_name: Option<String>,

    /// Interactive service configuration
    #[clap(long)]
    pub configure: bool,
}

impl AddService {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        // Prompt theme
        let prompt_theme = ColorfulTheme::default();

        // Choose plugin if not set
        let plugins: Vec<&str> = config.plugins.keys().map(|k| k.as_str()).collect();
        let plugin_name = match &self.plugin_name {
            Some(id) => id,
            None => {
                if plugins.is_empty() {
                    bail!("no plugin installed")
                }
                let selection = Select::with_theme(&prompt_theme)
                    .with_prompt("Select the plugin to use:")
                    .items(&plugins)
                    .default(0)
                    .interact()?;
                plugins[selection]
            }
        };

        if !plugins.contains(&plugin_name) {
            bail!("plugin `{}` does not exists", &plugin_name)
        }

        // Prompt id if not set
        let id = match self.id {
            Some(id) => id,
            // None => Text::new("Service id:").prompt()?,
            None => Input::<String>::with_theme(&prompt_theme)
                .with_prompt("Service id:")
                .interact_text()?,
        };

        // Naming contraints check
        let id = utils::ensure_kebab_case(&id)?;

        let mut service = Service {
            id: id.to_string(),
            plugin: plugin_name.to_string(),
            ..Default::default()
        };

        if config.services.contains(&service)
            && !Confirm::with_theme(&prompt_theme)
                .with_prompt("This service already exists, overwrite ?")
                .interact()?
        {
            bail!("action aborted")
        }

        // Load plugin
        let (instance, mut store) = PluginManager::new()?.load_plugin(plugin_name).await?;

        // Start plugin configuration
        service.configure(instance.settings(&mut store).await?, !self.configure)?;

        config.services.insert(service);
        tracing::info!("service `{}` created", &id);
        config.save_config(None)
    }
}

#[derive(Parser)]
pub struct ListServices {
    /// ID of the environment.
    #[clap(short, long)]
    pub env_id: Option<String>,
}

impl ListServices {
    pub fn run(self, config: &ProjectConfiguration) -> Result<()> {
        if config.services.is_empty() {
            bail!("no services defined")
        }

        for svc in &config.services {
            println!(
                "- `{}` (`{}`)",
                style(&svc.id).bold(),
                style(&svc.plugin).bold()
            );
        }
        Ok(())
    }
}

#[derive(Parser)]
pub struct RemoveService {
    /// ID of the service to remove.
    pub id: Option<String>,

    /// Force service removal and its references
    #[clap(short, long, default_value = "false")]
    pub force: bool,
}

impl RemoveService {
    pub fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        if config.services.is_empty() {
            bail!("no services defined")
        }

        // Prompt theme
        let prompt_theme = ColorfulTheme::default();

        // Choose service if not set
        let id = match self.id {
            Some(id) => id,
            None => {
                let services = config.service_ids()?;
                let selection = Select::with_theme(&prompt_theme)
                    .with_prompt("Select the service:")
                    .items(&services)
                    .default(0)
                    .interact()?;
                services[selection].to_string()
            }
        };

        // Check if service exists
        if !config.services.iter().any(|svc| svc.id == id) {
            bail!("service `{}` does not exists", &id)
        }

        // If removal is not forced, check if service is used in any environment
        if !self.force
      // check if service is used in any environment
      && config.environments.iter().any(|env| env.services.contains(&id))
      && !Confirm::with_theme(&prompt_theme).with_prompt("This service is used in some environment(s), force removal ?").interact()?
        {
            bail!("action aborted")
        }

        // remove all occurences of service in environments
        config.unlink_environments(&id);

        config.remove_service(&id);
        config.save_config(None)
    }
}

#[derive(Parser)]
pub struct ConfigureService {
    /// id of the service to configure
    pub id: Option<String>,
}

impl ConfigureService {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        if config.services.is_empty() {
            bail!("no services defined")
        }

        // Prompt theme
        let prompt_theme = ColorfulTheme::default();

        // Choose service if not set
        let id = match self.id {
            Some(id) => id,
            None => {
                let services = config.service_ids()?;
                let selection = Select::with_theme(&prompt_theme)
                    .with_prompt("Select the service:")
                    .items(&services)
                    .default(0)
                    .interact()?;
                services[selection].to_string()
            }
        };

        let mut service = config
            .services
            .take(&Service {
                id: id.clone(),
                ..Default::default()
            })
            .ok_or_else(|| anyhow!("service `{}` does not exist", &id))?;

        // Load plugin
        let (instance, mut store) = PluginManager::new()?.load_plugin(&service.plugin).await?;

        // Start plugin configuration
        service.configure(instance.settings(&mut store).await?, false)?;

        config.services.insert(service);
        tracing::info!("service `{}` configured", &id);
        config.save_config(None)
    }
}

pub const SPINNER: &[&str; 4] = &["-", "\\", "|", "/"];

#[derive(Parser)]
pub struct PingService;

impl PingService {
    pub async fn run(self, config: &ProjectConfiguration) -> Result<()> {
        if config.services.is_empty() {
            bail!("no services defined")
        }

        let mut plugins: HashMap<&str, Vec<&Service>> =
            HashMap::with_capacity(config.services.len());

        // Map service to plugin
        for svc in &config.services {
            plugins.entry(&svc.plugin).or_default().push(svc);
        }

        // Load plugins
        let plugin_manager = PluginManager::new()?;
        let mut set = JoinSet::new();

        for plugin_name in plugins.keys() {
            let plugin_name = plugin_name.to_string();
            let plugin_manager = plugin_manager.clone();
            set.spawn(async move { plugin_manager.load_plugin(plugin_name).await });
        }

        // Call ping function for each plugin's service
        while let Some(plugin) = set.join_next().await {
            let (instance, mut store) = plugin??;
            let meta = &instance.metadata;

            for svc in plugins
                .get(meta.name.as_str())
                .ok_or_else(|| anyhow!("plugin `{}` instance not found", &meta.name))?
                .iter()
            {
                let spinner = ProgressBar::new_spinner();
                spinner.enable_steady_tick(Duration::from_millis(130));
                spinner.set_style(
                    ProgressStyle::with_template("{spinner:.bold.dim} {msg}")
                        .unwrap()
                        .tick_strings(SPINNER),
                );
                spinner.set_message(svc.id.clone());

                let config = &serde_json::to_string(&svc.settings)?;
                if let Err(e) = instance.ping(&mut store, config).await {
                    spinner.finish_with_message(format!(
                        "{} ... {}",
                        style(&svc.id).bold().red(),
                        e
                    ));
                } else {
                    spinner
                        .finish_with_message(format!("{} ... OK", style(&svc.id).bold().green()));
                }
            }
        }

        Ok(())
    }
}
