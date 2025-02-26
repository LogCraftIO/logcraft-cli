// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use std::{env, fs, path, process};

use anyhow::Result;
use rexpect::session;

pub const DEFAULT_WORKSPACE: &str = "rules";
pub const PLUGIN_NAME: &str = "sample";

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

        // Spawn the command & return the TestingEnv instance
        let instance = Self {
            bin_path,
            root_dir: root.to_path_buf(),
            session: session::spawn_command(command, Some(10_000))?,
        };

        Ok(instance)
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
            let mut status = session::spawn_command(command, Some(10_000))?;
            status.exp_eof().expect("Failed to build testing plugin");
        }

        // Copy the dummy plugin to the plugin directory
        fs::copy(plugin_path, plugin_dir.join(PLUGIN_NAME))?;

        Ok(())
    }
}

pub fn assert_file_exists(path: &std::path::Path, expected: bool, message: &str) {
    assert_eq!(path.exists(), expected, "{}", message);
}
