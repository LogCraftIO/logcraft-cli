// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::{bail, Result};
use clap::Parser;
use logcraft_common::configuration::{ProjectConfiguration, LGC_CONFIG_PATH, LGC_RULES_DIR};
use std::{
    env::current_dir,
    fmt::Debug,
    fs::{self, File},
    path::{Path, PathBuf},
    str::FromStr,
};

/// Prepare working directory for other lgcli commands
#[derive(Parser, Debug, Default)]
#[clap(
    about = "Initialize LogCraft CLI configuration",
    allow_hyphen_values = true
)]
pub struct InitCommand {
    /// Optional path for the project root
    #[clap(short, long, default_value = ".")]
    pub path: Option<String>,
}

impl InitCommand {
    /// Run the init command.
    pub fn run(self) -> Result<()> {
        let project_path = match self.path {
            Some(path) => PathBuf::from_str(&path)?,
            None => current_dir()?,
        };

        let rules_dir = &project_path.join(LGC_RULES_DIR);
        if Path::new(rules_dir).exists() {
            println!(
                "warn: rules folder already exists in `{}`",
                &project_path.canonicalize()?.display()
            )
        }

        // Create detections directory & configuration file
        if let Err(e) = fs::create_dir_all(rules_dir) {
            bail!("unable to create detection rules directory: `{}`", e)
        }

        let full_path = &project_path.join(LGC_CONFIG_PATH);
        if File::create_new(full_path).is_err() {
            bail!(
                "error: `{}` already exists in `{}`",
                LGC_CONFIG_PATH,
                &project_path.canonicalize()?.display()
            )
        }

        ProjectConfiguration::default().save_config(Some(full_path))?;

        println!(
            "LogCraft configuration initialized in `{}`",
            &project_path.canonicalize()?.display()
        );
        Ok(())
    }
}
