// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use std::str::FromStr;

/// Prepare working directory for other lgcli commands
#[derive(clap::Parser)]
#[clap(
    about = "Initialize LogCraft CLI configuration",
    allow_hyphen_values = true
)]
pub struct InitCommand {
    /// Optional path for the project root
    #[clap(short, long, default_value = ".")]
    pub root: String,

    /// Optional base directory in which detections will be stored
    #[clap(short, long, default_value = lgc_common::configuration::LGC_RULES_DIR)]
    pub workspace: String,

    /// Creates the workspace directory in the root path [default: false]
    #[clap(short, long)]
    pub create: bool,
}

impl InitCommand {
    /// Run the init command.
    pub fn run(self) -> anyhow::Result<()> {
        use anyhow::bail;
        use lgc_common::configuration;

        let project_path = std::path::PathBuf::from_str(&self.root)?;
        if !project_path.exists() {
            bail!("directory '{}' does not exist", self.root)
        } else if !project_path.is_dir() {
            bail!("'{}' is not a directory", self.root)
        }

        if self.create {
            let rules_dir = &project_path.join(&self.workspace);
            if std::path::Path::new(rules_dir).exists() {
                bail!("workspace directory '{}' already exists", self.workspace)
            }

            // Create detections directory & configuration file
            if let Err(e) = std::fs::create_dir(rules_dir) {
                bail!("unable to create detection rules directory: {}", e)
            }

            tracing::info!("workspace directory '{}' created", self.workspace);
        }

        let config_path = &project_path.join(configuration::LGC_CONFIG_PATH);
        if std::fs::File::create_new(config_path).is_err() {
            bail!("{} already exists", configuration::LGC_CONFIG_PATH)
        }

        // Save the configuration
        configuration::ProjectConfiguration {
            core: configuration::CoreConfiguration {
                workspace: self.workspace,
                ..Default::default()
            },
            ..Default::default()
        }
        .save_config(config_path.to_str())?;

        tracing::info!("{} saved", configuration::LGC_CONFIG_PATH);
        Ok(())
    }
}
