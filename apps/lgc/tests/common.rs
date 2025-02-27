// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use std::{env, fs, path, process};

use anyhow::Result;
use lgc_common::configuration::ProjectConfiguration;
use lgc_common::configuration::LGC_CONFIG_PATH;
use rexpect::session;

pub const DEFAULT_WORKSPACE: &str = "rules";
pub const PLUGIN_NAME: &str = "sample";
pub const DEFAULT_TIMEOUT: u64 = 600_000;

/// Provides helpers to run command tests.
pub struct TestingEnv {
    pub root_dir: path::PathBuf,
    pub bin_path: path::PathBuf,
    pub session: session::PtySession,
}

impl TestingEnv {
    pub fn init(
        cwd: bool,
        root: &path::Path,
        workspace: Option<&str>,
        create: bool,
    ) -> Result<Self> {
        // Retrieve the CLI binary path & build the command
        let bin_path = assert_cmd::cargo::cargo_bin(env!("CARGO_PKG_NAME"));
        let mut command = process::Command::new(&bin_path);
        if cwd {
            command.current_dir(root);
        }

        // Construct the init command
        command.args([
            "init",
            "--root",
            root.to_str()
                .expect("Failed to convert root path to string"),
        ]);
        if let Some(workspace) = workspace {
            command.arg("--workspace").arg(workspace);
        }
        if create {
            command.arg("--create");
        }

        // Return TestingEnv instance
        Ok(Self {
            bin_path,
            root_dir: root.to_path_buf(),
            session: session::spawn_command(command, Some(DEFAULT_TIMEOUT))?,
        })
    }

    pub fn setup_plugin(&self) -> Result<()> {
        // Ensure plugin dir exists
        let plugin_dir = self.root_dir.join(".logcraft/plugins");
        fs::create_dir_all(&plugin_dir)?;

        let cargo_root =
            path::PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("OUT_DIR not set"));

        let plugin_path = cargo_root.join(format!(
            "../../target/wasm32-wasip2/release/{PLUGIN_NAME}.wasm"
        ));

        if !plugin_path.exists() {
            // Build the dummy plugin
            let mut command = process::Command::new("cargo");
            command.args([
                "build",
                "-p",
                PLUGIN_NAME,
                "--release",
                "--target",
                "wasm32-wasip2",
            ]);
            command.current_dir(cargo_root);

            // Spawn the command
            let mut status = session::spawn_command(command, Some(DEFAULT_TIMEOUT))?;
            status.exp_eof().expect("Failed to build testing plugin");
        }

        // Copy the dummy plugin to the plugin directory
        fs::copy(
            plugin_path,
            plugin_dir.join(PLUGIN_NAME).with_extension("wasm"),
        )?;

        // Load the configuration
        let configuration_path = self.root_dir.join(LGC_CONFIG_PATH);
        let configuration_content = fs::read_to_string(&configuration_path)?;

        // Update base_dir for plugin retrieval
        let mut configuration: ProjectConfiguration = toml::from_str(&configuration_content)?;
        configuration.core.base_dir = Some(self.root_dir.join(".logcraft").display().to_string());
        configuration.save_config(Some(configuration_path.to_str().unwrap()))?;

        Ok(())
    }
}

pub fn assert_file_exists(path: &std::path::Path, expected: bool, message: &str) {
    assert_eq!(path.exists(), expected, "{}", message);
}
