// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

#![forbid(unsafe_code)]

use anyhow::Result;
use std::env;

use lgc::commands;
use lgc_common::{configuration, utils};

#[tokio::main]
async fn main() {
    // If an error occurs, log it and exit
    if let Err(err) = LogCraftCli::init().await {
        tracing::error!("{err}");
        std::process::exit(1);
    }
}

const HELP_TEMPLATE: &str = r#"
{before-help}{name} {version}

{usage-heading} {usage}

{all-args}{after-help}
"#;

/// LogCraft CLI
#[derive(clap::Parser)]
#[clap(
    name="LogCraft CLI",
    help_template=HELP_TEMPLATE,
    version=concat!("v", env!("CARGO_PKG_VERSION")
))]
struct LogCraftCli {
    #[clap(subcommand)]
    commands: LogCraftCommands,

    #[clap(skip)]
    config: configuration::ProjectConfiguration,
}

/// LogCraft CLI
#[derive(clap::Subcommand)]
enum LogCraftCommands {
    Init(commands::init::InitCommand),
    Ping(commands::ping::PingCommand),
    Validate(commands::validate::ValidateCommand),
    Plan(commands::plan::PlanCommand),
    Apply(commands::apply::ApplyCommand),
    Destroy(commands::destroy::DestroyCommand),
    #[clap(subcommand)]
    Services(commands::services::ServicesCommands),
}

impl LogCraftCli {
    /// Initialize and load the configuration.
    async fn init() -> Result<()> {
        use clap::{builder::styling, CommandFactory};
        use console::{set_colors_enabled, set_colors_enabled_stderr};
        use figment::providers::{Env, Format, Toml};

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
        let mut cli = <LogCraftCli as clap::FromArgMatches>::from_arg_matches(&matches)?;

        tracing_subscriber::fmt()
            .with_writer(std::io::stdout)
            .with_target(false)
            .without_time()
            .with_env_filter(tracing_subscriber::EnvFilter::from_env("LGC_LOG"))
            .with_max_level(tracing::Level::INFO)
            .init();

        // Load configuration
        match cli.commands {
            LogCraftCommands::Init(cmd) => return cmd.run(),
            _ => {
                let configuration_path = std::path::PathBuf::from(configuration::LGC_CONFIG_PATH);

                if configuration_path.is_file() {
                    let mut configuration_file = std::fs::read_to_string(configuration_path)?;

                    // Environment variables substitution
                    if envsubst::is_templated(&configuration_file) {
                        configuration_file = envsubst::substitute(
                            configuration_file,
                            &env::vars()
                                .filter_map(|(key, value)| {
                                    if !utils::env_forbidden_chars(&key)
                                        && !utils::env_forbidden_chars(&value)
                                    {
                                        Some((key, value))
                                    } else {
                                        None
                                    }
                                })
                                .collect::<std::collections::HashMap<String, String>>(),
                        )?;
                    }

                    cli.config = match figment::Figment::new()
                        .merge(Toml::string(&configuration_file))
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
                    tracing::error!(
                        "no configuration file, run 'lgc init' to initialize a new project"
                    );
                    std::process::exit(1)
                }
            }
        };

        cli.run().await
    }

    /// LogCraft CLI entrypoint.
    pub async fn run(self) -> Result<()> {
        match self.commands {
            // General commands
            LogCraftCommands::Init(cmd) => cmd.run(),
            LogCraftCommands::Ping(cmd) => cmd.run(self.config).await,
            LogCraftCommands::Validate(cmd) => cmd.run(self.config).await,
            LogCraftCommands::Plan(cmd) => cmd.run(self.config).await,
            LogCraftCommands::Apply(cmd) => cmd.run(self.config).await,
            LogCraftCommands::Destroy(cmd) => cmd.run(self.config).await,
            // Services commands
            LogCraftCommands::Services(cmd) => cmd.run(self.config).await,
        }
    }
}
