// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

#![forbid(unsafe_code)]

use anyhow::Result;
use clap::builder::styling;
use clap::{crate_version, Subcommand};
use clap::{CommandFactory, FromArgMatches, Parser};
use console::{set_colors_enabled, set_colors_enabled_stderr};
use figment::providers::{Env, Format, Yaml};
use figment::Figment;
use lgc::commands::{
    deploy::DeployCommand, destroy::DestroyCommand, diff::DiffCommand,
    environments::EnvironmentsCommands, init::InitCommand, plugins::PluginsCommands,
    services::ServicesCommands, validate::ValidateCommand,
};
use lgc_common::configuration::{ProjectConfiguration, LGC_CONFIG_PATH};
use lgc_common::utils::env_forbidden_chars;
use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fs};
use tracing::Level;
use tracing_subscriber::EnvFilter;

#[forbid(unsafe_code)]
#[tokio::main]
async fn main() {
    if let Err(err) = LogCraftCli::init().await {
        tracing::error!("{err}");
        std::process::exit(1);
    }
}

const HELP_TEMPLATE: &str = r#"
{before-help}{about} {version}

{usage-heading} {usage}

{all-args}{after-help}
"#;

/// LogCraft CLI
#[derive(Parser)]
#[clap(name="LogCraft", help_template=HELP_TEMPLATE, version=crate_version!())]
struct LogCraftCli {
    #[clap(subcommand)]
    commands: LogCraftCommands,

    #[clap(skip)]
    config: ProjectConfiguration,
}

/// LogCraft CLI
#[derive(Subcommand)]
// #[clap(name="LogCraft", help_template=HELP_TEMPLATE, version=crate_version!())]
enum LogCraftCommands {
    Deploy(DeployCommand),
    Destroy(DestroyCommand),
    Diff(DiffCommand),
    #[clap(subcommand, name = "envs")]
    Environments(EnvironmentsCommands),
    Init(InitCommand),
    #[clap(subcommand)]
    Plugins(PluginsCommands),
    #[clap(subcommand)]
    Services(ServicesCommands),
    Validate(ValidateCommand),
}

impl LogCraftCli {
    /// Initialize and load the configuration.
    async fn init() -> Result<()> {
        // Prepare style
        let styles = styling::Styles::styled()
            .header(styling::AnsiColor::Green.on_default().bold().underline())
            .usage(styling::AnsiColor::Green.on_default().bold().underline())
            .literal(styling::AnsiColor::Blue.on_default().bold());

        // Forces tty colors
        if env::var("LGC_FORCE_COLORS").is_ok_and(|t| &t == "true") {
            set_colors_enabled(true);
            set_colors_enabled_stderr(true);
        }

        let matches = LogCraftCli::command().styles(styles).get_matches();
        let mut cli = LogCraftCli::from_arg_matches(&matches)?;

        tracing_subscriber::fmt()
            .with_writer(std::io::stdout)
            .with_target(false)
            .without_time()
            .with_env_filter(EnvFilter::from_env("LGC_LOG"))
            .with_max_level(Level::INFO)
            .init();

        // Load configuration
        match cli.commands {
            LogCraftCommands::Init(cmd) => return cmd.run(),
            _ => {
                let configuration_path = PathBuf::from(LGC_CONFIG_PATH);

                if configuration_path.is_file() {
                    let mut configuration_file = fs::read_to_string(configuration_path)?;

                    // Environment variables substitution
                    if envsubst::is_templated(&configuration_file) {
                        configuration_file = envsubst::substitute(
                            configuration_file,
                            &env::vars()
                                .filter_map(|(key, value)| {
                                    if !env_forbidden_chars(&key) && !env_forbidden_chars(&value) {
                                        Some((key, value))
                                    } else {
                                        None
                                    }
                                })
                                .collect::<HashMap<String, String>>(),
                        )?;
                    }

                    cli.config = match Figment::new()
                        .merge(Yaml::string(&configuration_file))
                        .merge(Env::prefixed("LGC_").split("_"))
                        .extract()
                    {
                        Ok(config) => config,
                        Err(e) => {
                            tracing::error!("unable to load configuration: {}", e);
                            std::process::exit(1)
                        }
                    };
                } else {
                    tracing::error!("unable to find configuration file, run `lgc init` to initialize a new project");
                    std::process::exit(1)
                }
            }
        };

        cli.run().await
    }

    /// LogCraft CLI entrypoint.
    pub async fn run(mut self) -> Result<()> {
        match self.commands {
            // General commands
            LogCraftCommands::Init(cmd) => cmd.run(),
            LogCraftCommands::Diff(cmd) => cmd.run(&self.config).await,
            LogCraftCommands::Deploy(cmd) => cmd.run(&self.config).await,
            LogCraftCommands::Destroy(cmd) => cmd.run(&self.config).await,
            LogCraftCommands::Validate(cmd) => cmd.run(&self.config).await,
            // Plugins commands
            LogCraftCommands::Plugins(cmd) => cmd.run(&mut self.config).await,
            // Environments commands
            LogCraftCommands::Environments(cmd) => cmd.run(&mut self.config).await,
            // Services commands
            LogCraftCommands::Services(cmd) => cmd.run(&mut self.config).await,
        }
    }
}
