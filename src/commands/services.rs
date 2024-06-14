// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::{anyhow, bail, Result};
use clap::{Parser, Subcommand};
use inquire::{Confirm, Select, Text};
use logcraft_common::{
    configuration::{ProjectConfiguration, Service},
    plugins::manager::{PluginActions, PluginManager},
    utils,
};
use std::collections::HashMap;
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
    /// Name of the service to create
    pub name: Option<String>,

    /// Name of the plugin used by this service
    #[clap(short, long)]
    pub plugin_name: Option<String>,

    /// Enable prompt for plugin settings
    #[clap(long)]
    pub configure: bool,

    /// Enable prompt for plugin settings
    #[clap(long)]
    pub insecure: bool,
}

impl AddService {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        // Choose plugin if not set
        let plugins: Vec<&str> = config.plugins.keys().map(|k| k.as_str()).collect();
        let plugin_name = match &self.plugin_name {
            Some(name) => name,
            None => {
                if plugins.is_empty() {
                    bail!("the configuration does not have any plugin")
                }
                Select::new("Select the plugin to use:", plugins.clone()).prompt()?
            }
        };

        if !plugins.contains(&plugin_name) {
            bail!("plugin `{}` does not exists", &plugin_name)
        }

        // Prompt name if not set
        let name = match self.name {
            Some(name) => name,
            None => Text::new("Service name:").prompt()?,
        };

        // Naming contraints check
        let name = utils::ensure_kebab_case(&name)?;

        let mut service = Service {
            name: name.clone(),
            plugin: plugin_name.to_string(),
            ..Default::default()
        };

        if config.services.contains(&service)
            && !Confirm::new("This service already exists, overwrite ?")
                .with_default(false)
                .prompt()?
        {
            println!("action aborted");
            return Ok(());
        }

        // Load plugin
        let (instance, mut store) = PluginManager::new()?.load_plugin(plugin_name).await?;

        // Start plugin configuration
        service.configure(instance.settings(&mut store).await?, !self.configure)?;

        config.services.insert(service);
        println!("service `{}` created", &name);
        config.save_config(None)
    }
}

#[derive(Parser)]
pub struct ListServices {
    /// Name of the environment.
    #[clap(short, long)]
    pub env_name: Option<String>,
}

impl ListServices {
    pub fn run(self, config: &ProjectConfiguration) -> Result<()> {
        if config.services.is_empty() {
            bail!("no services defined")
        }

        for svc in &config.services {
            println!("`{}` (`{}`)", &svc.name, svc.plugin);
        }
        Ok(())
    }
}

#[derive(Parser)]
pub struct RemoveService {
    /// Name of the service to remove.
    pub name: Option<String>,

    /// Force service removal and its references
    #[clap(short, long, default_value = "false")]
    pub force: bool,
}

impl RemoveService {
    pub fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        if config.services.is_empty() {
            bail!("no services defined")
        }

        // Choose service if not set
        let name = match self.name {
            Some(name) => name,
            None => Select::new("Select the service:", config.service_names()?)
                .prompt()?
                .to_owned(),
        };

        // Check if service exists
        if !config.services.iter().any(|svc| svc.name == name) {
            bail!("service `{}` does not exists", &name)
        }

        // If removal is not forced, check if service is used in any environment
        if !self.force
      // check if service is used in any environment
      && config.environments.iter().any(|env| env.services.contains(&name))
      && !Confirm::new("This service is used in some environment(s), remove it along with its references ?").with_default(false).prompt()?
    {
      bail!("action aborted")
    }

        // remove all occurences of service in environments
        config.unlink_environments(&name);

        config.remove_service(&name);
        config.save_config(None)
    }
}

#[derive(Parser)]
pub struct ConfigureService {
    /// name of the service to configure
    pub name: Option<String>,
}

impl ConfigureService {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        if config.services.is_empty() {
            bail!("no services defined")
        }

        // Choose service if not set
        let name = self.name.unwrap_or_else(|| {
            Select::new("Select the service:", config.service_names().unwrap())
                .prompt()
                .unwrap()
                .to_owned()
        });

        let mut service = config
            .services
            .take(&Service {
                name: name.clone(),
                ..Default::default()
            })
            .ok_or_else(|| anyhow!("service `{}` does not exist", &name))?;

        // Load plugin
        let (instance, mut store) = PluginManager::new()?.load_plugin(&service.plugin).await?;

        // Start plugin configuration
        service.configure(instance.settings(&mut store).await?, false)?;

        config.services.insert(service);
        println!("service `{}` configured", &name);
        config.save_config(None)
    }
}

#[derive(Parser)]
pub struct PingService;

impl PingService {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
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
                let config = &serde_json::to_string(&svc.settings)?;

                if let Err(e) = instance.ping(&mut store, config).await {
                    println!("{}... {}", &svc.name, e);
                } else {
                    println!("{}... OK", &svc.name);
                }
            }
        }

        Ok(())
    }
}
