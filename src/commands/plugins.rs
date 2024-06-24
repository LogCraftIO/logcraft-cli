// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::{anyhow, bail, Result};
use clap::{Parser, Subcommand};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use logcraft_common::{
    configuration::ProjectConfiguration,
    plugins::{
        cleanup_plugin, determine_plugin_location,
        manager::{PluginActions, PluginManager},
        Plugin, PluginLocation, LGC_PLUGINS_PATH,
    },
};
use std::path::PathBuf;

/// Manage plugins
#[derive(Subcommand)]
pub enum PluginsCommands {
    /// Install plugin
    #[clap(alias = "i")]
    Install(InstallPlugin),

    /// List installed plugins
    List(ListPlugin),

    /// Remove plugin
    Uninstall(UninstallPlugin),

    /// Update plugin
    Update(UpdatePlugin),

    /// Get plugin configuration schema
    Schema(PluginSchema),
}

impl PluginsCommands {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        match self {
            Self::Install(cmd) => cmd.run(config).await,
            Self::Schema(cmd) => cmd.run(config).await,
            Self::List(cmd) => cmd.run(config),
            Self::Uninstall(cmd) => cmd.run(config).await,
            Self::Update(cmd) => cmd.run(config).await,
        }
    }
}

#[derive(Parser)]
pub struct InstallPlugin {
    /// Location of the plugin
    pub source: Option<String>,
    // /// Version of plugin to fetch
    // #[clap(default_value = "latest")]
    // pub version: String,
}

impl InstallPlugin {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        // Prompt theme
        let prompt_theme = ColorfulTheme::default();

        // Prompt source if not set
        let source = match self.source {
            Some(source) => source,
            None => Input::<String>::with_theme(&prompt_theme)
                .with_prompt("Plugin source:")
                .interact_text()?,
        };

        // Determine the plugin location
        let location = determine_plugin_location(&source)?;

        // Retrieve plugin informations
        let meta = PluginManager::new()?.install_plugin(&location).await?;

        let source = match location {
            PluginLocation::Local(_) => {
                PluginLocation::Local(PathBuf::from(LGC_PLUGINS_PATH).join(&meta.name))
            } // PluginLocation::Remote(url) => url,
              // PluginLocation::Oci(image) => image,
        };

        config.plugins.insert(
            meta.name,
            Plugin {
                source,
                version: meta.version,
                description: meta.description,
                author: meta.author,
            },
        );

        config.save_config(None)
    }
}

#[derive(Parser)]
pub struct ListPlugin;

impl ListPlugin {
    pub fn run(self, config: &ProjectConfiguration) -> Result<()> {
        // Check if there are any plugins
        if config.plugins.is_empty() {
            bail!("no plugin installed");
        }

        // Iterate and print plugin information
        config.plugins.iter().for_each(|(name, plugin)| {
            println!(
                "- `{}` (`{}`)",
                style(name).bold(),
                style(&plugin.version).bold()
            );
        });

        Ok(())
    }
}

#[derive(Parser)]
pub struct UninstallPlugin {
    /// Name of the plugin.
    pub name: Option<String>,

    /// Force plugin removal, including all associated services and environments
    #[clap(short, long, default_value = "false")]
    pub force: bool,
}

impl UninstallPlugin {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        if config.plugins.is_empty() {
            bail!("no plugin installed")
        }

        // Prompt theme
        let prompt_theme = ColorfulTheme::default();

        let name = match self.name {
            Some(name) => name,
            None => {
                let plugins = config.plugins.keys().cloned().collect::<Vec<_>>();
                let selection = Select::with_theme(&prompt_theme)
                    .with_prompt("Select the plugin to uninstall:")
                    .items(&plugins)
                    .default(0)
                    .interact()?;
                plugins[selection].clone()
            }
        };

        if config.plugins.remove(&name).is_none() {
            bail!("plugin `{}` does not exists", &name)
        };

        let services = config
            .services
            .iter()
            .filter(|svc| svc.plugin == name)
            .map(|svc| svc.id.clone())
            .collect::<Vec<_>>();

        // If removal is not forced, check if plugin is used in any service
        if !self.force
            && !services.is_empty()
            && !Confirm::with_theme(&prompt_theme)
                .with_prompt(&format!(
                    "This plugin is used in `{}` services(s), force removal ?",
                    style(services.join(", ")).red()
                ))
                .interact()?
        {
            bail!("action aborted")
        }

        for svc_id in services {
            config.remove_service(&svc_id);
            config.unlink_environments(&svc_id)
        }

        cleanup_plugin(&name)?;
        config.save_config(None)
    }
}

#[derive(Parser)]
pub struct PluginSchema {
    /// Name of the plugin.
    pub name: Option<String>,
}

impl PluginSchema {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        if config.plugins.is_empty() {
            bail!("no plugin installed")
        }

        // Prompt theme
        let prompt_theme = ColorfulTheme::default();

        // Prompt name if not set
        let name = match self.name {
            Some(name) => name,
            None => {
                let plugins = config.plugins.keys().cloned().collect::<Vec<_>>();
                let selection = Select::with_theme(&prompt_theme)
                    .with_prompt("Select the plugin:")
                    .items(&plugins)
                    .default(0)
                    .interact()?;
                plugins[selection].clone()
            }
        };

        // Load plugin
        let (instance, mut store) = PluginManager::new()?.load_plugin(&name).await?;

        // Retrieve schema
        let schema = instance.schema(&mut store).await?;

        println!("{schema}");
        Ok(())
    }
}

#[derive(Parser)]
pub struct UpdatePlugin {
    /// Name of the plugin.
    pub name: Option<String>,
}

impl UpdatePlugin {
    pub async fn run(self, config: &mut ProjectConfiguration) -> Result<()> {
        if config.plugins.is_empty() {
            bail!("no plugin installed")
        }

        // Prompt theme
        let prompt_theme = ColorfulTheme::default();

        // Prompt name if not set
        let name = match self.name {
            Some(name) => name,
            None => {
                let plugins = config.plugins.keys().cloned().collect::<Vec<_>>();
                let selection = Select::with_theme(&prompt_theme)
                    .with_prompt("Select the plugin:")
                    .items(&plugins)
                    .default(0)
                    .interact()?;
                plugins[selection].clone()
            }
        };

        let plugin = config
            .plugins
            .get(&name)
            .ok_or_else(|| anyhow!("plugin `{}` does not exists", &name))?;
        match plugin.source {
            PluginLocation::Local(_) => {
                bail!("command `plugin update` is not available for file source, please use `plugin install` instead")
            } // _ => ()
        }

        // ! Not needed for now - Update isn't available for Local source.
        // // Load plugin
        // let meta = PluginManager::new()?.install_plugin(&plugin.source).await?;
        // tracing::info!(
        //     "`{}` plugin loaded with version: `{}`",
        //     &meta.id,
        //     &meta.version
        // );

        // config.plugins.insert(meta.id, Plugin {
        //     source: plugin.source,
        //     version: meta.version,
        //     description: meta.description,
        //     author: meta.author,
        // });

        // Ok(())
    }
}
