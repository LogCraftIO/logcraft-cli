// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use lgc_common::{
    configuration::{self, LGC_BASE_DIR},
    plugins::manager::{PluginActions, PluginManager},
    utils::{self, ensure_kebab_case},
};
use std::path;

/// Manage backend services
#[derive(clap::Subcommand)]
#[clap(about = "Manage remote services")]
pub enum ServicesCommands {
    /// Create a new service
    Create(CreateService),

    /// List services
    List(ListServices),

    /// Remove a service
    Remove(RemoveService),

    /// Configure a service
    Configure(ConfigureService),
}

impl ServicesCommands {
    pub async fn run(self, config: configuration::ProjectConfiguration) -> anyhow::Result<()> {
        match self {
            Self::Create(cmd) => cmd.run(config).await,
            Self::List(cmd) => cmd.run(config),
            Self::Remove(cmd) => cmd.run(config),
            Self::Configure(cmd) => cmd.run(config).await,
        }
    }
}

#[derive(clap::Parser)]
pub struct CreateService {
    /// Set the new service identifier
    #[clap(short, long)]
    pub identifier: Option<String>,

    /// Name of the plugin used by this service
    #[clap(short, long)]
    pub plugin: Option<String>,

    /// Environment name this service belongs to
    #[clap(short, long)]
    pub env: Option<String>,

    /// Interactive service configuration [default: false]
    #[clap(short, long)]
    pub configure: bool,
}

impl CreateService {
    pub async fn run(self, mut config: configuration::ProjectConfiguration) -> anyhow::Result<()> {
        // Get plugins directory
        let plugins_dir =
            path::PathBuf::from(config.core.base_dir.as_deref().unwrap_or(LGC_BASE_DIR))
                .join("plugins");

        // Prompt theme for interactive mode
        let prompt_theme = dialoguer::theme::ColorfulTheme::default();
        
        // Prompt for service identifier if not provided
        let identifier: String = ensure_kebab_case(match self.identifier {
            Some(id) => id,
            None => dialoguer::Input::<String>::with_theme(&prompt_theme)
                .with_prompt("Service identifier:")
                .interact_text()?,
        })?;

        // Check if service already exists
        if config.services.contains_key(&identifier) {
            anyhow::bail!("identifier '{identifier}' is already defined");
        }

        // Start plugin manager and retrieve plugin names
        let plugin_manager = PluginManager::new()?;
        let plugin_names = plugin_manager.plugin_names(&plugins_dir)?;

        // Determine plugin_name as an owned String
        let plugin_name: String = match self.plugin {
            Some(plugin) => {
                // Check that the plugin actually exists
                if !plugin_names.contains(&plugin) {
                    anyhow::bail!("plugin '{}' does not exist", plugin);
                }
                plugin
            }
            None => {
                let selection = dialoguer::Select::with_theme(&prompt_theme)
                    .with_prompt("Select the plugin to use:")
                    .items(&plugin_names)
                    .default(0)
                    .interact()?;

                if let Some(plugin) = plugin_names.get(selection) {
                    plugin.to_string()
                } else {
                    anyhow::bail!("plugin not found");
                }
            }
        };

        // Prompt for environment name if not provided
        let environment = match self.env {
            Some(env) => env,
            None => dialoguer::Input::<String>::with_theme(&prompt_theme)
                .with_prompt("Environment name:")
                .allow_empty(true)
                .interact_text()?,
        };

        // Enforce kebab case
        let environment = match environment.is_empty() {
            true => None,
            false => Some(utils::ensure_kebab_case(environment)?),
        };

        // Create new service and configure
        let mut service = configuration::Service {
            plugin: plugin_name.clone(),
            environment,
            ..Default::default()
        };

        // Prompt for service configuration
        let use_default = !dialoguer::Confirm::with_theme(&prompt_theme)
            .with_prompt("Do you want to configure the service now?")
            .interact()?;

        // Load plugin & configure plugin
        let (instance, mut store) = plugin_manager
            .load_plugin(plugins_dir.join(plugin_name).with_extension("wasm"))
            .await?;
        service.configure(&instance.settings(&mut store).await?, use_default)?;

        tracing::info!("service '{identifier}' successfully created");

        // Insert the service into the config
        config.services.insert(identifier, service);

        // Save changes
        config.save_config(None)
    }
}

#[derive(clap::Parser)]
pub struct ListServices {}

impl ListServices {
    pub fn run(self, config: configuration::ProjectConfiguration) -> anyhow::Result<()> {
        config.services.iter().for_each(|(name, settings)| {
            println!(
                "---\n{:<11}: {}\n{:<11}: {}\n{:<11}: {}",
                "service",
                console::style(name).bold().green(),
                "environment",
                console::style(
                    &settings
                        .environment
                        .clone()
                        .unwrap_or(console::style("undefined").italic().dim().to_string())
                )
                .bold(),
                "plugin",
                console::style(&settings.plugin).bold(),
            );
        });
        Ok(())
    }
}

#[derive(clap::Parser)]
pub struct RemoveService {
    /// Service identifier to remove
    pub identifier: Option<String>,
}

impl RemoveService {
    pub fn run(self, mut config: configuration::ProjectConfiguration) -> anyhow::Result<()> {
        if config.services.is_empty() {
            anyhow::bail!("no services defined")
        }

        // Prompt theme for interactive mode
        let prompt_theme = dialoguer::theme::ColorfulTheme::default();

        // Determine identifier as an owned String
        let identifier: String = match self.identifier {
            Some(identifier) => identifier,
            None => {
                let services_names = config.services.keys().collect::<Vec<_>>();
                let selection = dialoguer::Select::with_theme(&prompt_theme)
                    .with_prompt("Select the service to remove:")
                    .items(&services_names)
                    .default(0)
                    .interact()?;
                services_names[selection].to_string()
            }
        };

        // Remove service from configuation
        if config.services.remove_entry(&identifier).is_none() {
            anyhow::bail!("service '{}' not found", &identifier)
        }

        tracing::info!("service '{identifier}' successfully removed");

        // Save changes
        config.save_config(None)
    }
}

#[derive(clap::Parser)]
pub struct ConfigureService {
    /// Service identifier to remove
    pub identifier: Option<String>,
}

impl ConfigureService {
    pub async fn run(self, mut config: configuration::ProjectConfiguration) -> anyhow::Result<()> {
        if config.services.is_empty() {
            anyhow::bail!("no services defined")
        }

        if config.services.is_empty() {
            anyhow::bail!("no services defined")
        }

        // Prompt theme for interactive mode
        let prompt_theme = dialoguer::theme::ColorfulTheme::default();

        // Determine identifier as an owned String
        let identifier: String = match self.identifier {
            Some(identifier) => identifier,
            None => {
                let services_names = config.services.keys().collect::<Vec<_>>();
                let selection = dialoguer::Select::with_theme(&prompt_theme)
                    .with_prompt("Select the service to configure:")
                    .items(&services_names)
                    .default(0)
                    .interact()?;
                services_names[selection].to_string()
            }
        };

        // Get mutable reference to service
        let service = config
            .services
            .get_mut(&identifier)
            .ok_or_else(|| anyhow::anyhow!("service '{}' not found", &identifier))?;

        // Get plugins directory
        let plugins_dir =
            path::PathBuf::from(config.core.base_dir.as_deref().unwrap_or(LGC_BASE_DIR))
                .join("plugins");

        // Load plugin
        let (instance, mut store) = PluginManager::new()?
            .load_plugin(plugins_dir.join(&service.plugin).with_extension("wasm"))
            .await?;

        // Start plugin's service configuration
        service.configure(&instance.settings(&mut store).await?, false)?;

        tracing::info!("service '{identifier}' configured");

        config.save_config(None)
    }
}
