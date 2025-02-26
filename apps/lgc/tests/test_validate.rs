// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use anyhow::Result;
use common::{DEFAULT_WORKSPACE, PLUGIN_NAME};
use rexpect::session::spawn_command;
use std::process;

use lgc_common::configuration::LGC_CONFIG_PATH;

pub mod common;

const SERVICE_NAME: &str = "my-service";
const ENVIRONMENT_NAME: &str = "testing";

const SAMPLE_RULE: &str = r#"
title: High Entropy Domain Names - Sample Rule

search: |-
  stats count

parameters:
  # Unknown parameters
  unknown_parameters: "null"
  # Known parameters
  cron_schedule: 0 * * * *
  action.notable: 1
  action.email: john=doe@foo.bar
  action.notable.param.nes_fields: user,dest
  disabled: 1
  is_visible: true
"#;

const INVALID_SAMPLE_RULE: &str = r#"
    title: High Entropy Domain Names - Sample Rule
    
    search: |-
      stats count
    
    parameters:
      # Unknown parameters
      disabled: foo
"#;

/// Test that running validate command with a valid rule passes with expected output.
#[test]
fn test_validate() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    let mut env = common::TestingEnv::init(false, temp_dir.path(), None, true)?;
    env.session
        .exp_string(&format!("`{}` saved", LGC_CONFIG_PATH))?;

    // Add the sample plugin to the project
    env.setup_plugin()?;
    common::assert_file_exists(
        &temp_dir.join(".logcraft/plugins/sample"),
        true,
        "Plugin `sample` not found in testing project",
    );

    // Create a new command to create a service
    let mut command = process::Command::new(&env.bin_path);
    command.args([
        "services",
        "create",
        "-i",
        SERVICE_NAME,
        "-e",
        ENVIRONMENT_NAME,
        "-p",
        common::PLUGIN_NAME,
    ]);
    command.current_dir(&temp_dir);

    let mut session = spawn_command(command, None)?;
    session.exp_string("Do you want to configure the service now?")?;
    session.send_line("n")?;
    session.exp_string(&format!("service `{}` successfully created", SERVICE_NAME))?;
    session.exp_eof()?;

    // Start detection validation
    let mut command = process::Command::new(&env.bin_path);
    command.args(["validate"]);
    command.current_dir(&temp_dir);

    // Create sample plugin rule directory
    let rule_dir = temp_dir.join(DEFAULT_WORKSPACE).join(PLUGIN_NAME);
    std::fs::create_dir_all(&rule_dir)?;

    // Create sample detection file in the project workspace
    let detection_file = temp_dir
        .join(DEFAULT_WORKSPACE)
        .join(PLUGIN_NAME)
        .join("sample.rule");
    std::fs::write(&detection_file, SAMPLE_RULE)?;

    let mut session = spawn_command(command, None)?;
    session.exp_string("all good, no problem identified.")?;
    session.exp_eof()?;

    Ok(())
}

/// Test that running validate command with no rule passes with expected output.
#[test]
fn test_validate_empty_rules() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    let mut env = common::TestingEnv::init(false, temp_dir.path(), None, true)?;
    env.session
        .exp_string(&format!("`{}` saved", LGC_CONFIG_PATH))?;

    // Add the sample plugin to the project
    env.setup_plugin()?;
    common::assert_file_exists(
        &temp_dir.join(".logcraft/plugins/sample"),
        true,
        "Plugin `sample` not found in testing project",
    );

    // Create a new command to create a service
    let mut command = process::Command::new(&env.bin_path);
    command.args([
        "services",
        "create",
        "-i",
        SERVICE_NAME,
        "-e",
        ENVIRONMENT_NAME,
        "-p",
        common::PLUGIN_NAME,
    ]);
    command.current_dir(&temp_dir);

    let mut session = spawn_command(command, None)?;
    session.exp_string("Do you want to configure the service now?")?;
    session.send_line("n")?;
    session.exp_string(&format!("service `{}` successfully created", SERVICE_NAME))?;
    session.exp_eof()?;

    // Remove a service that does not exist
    let mut command = process::Command::new(&env.bin_path);
    command.args(["validate"]);
    command.current_dir(&temp_dir);

    let mut session = spawn_command(command, None)?;
    session.exp_string("nothing to validate, no detection found.")?;
    session.exp_eof()?;

    Ok(())
}

/// Test that running validate command with a rule that has no plugin passes with expected output.
#[test]
fn test_validate_plugin_missing() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    let mut env = common::TestingEnv::init(false, temp_dir.path(), None, true)?;
    env.session
        .exp_string(&format!("`{}` saved", LGC_CONFIG_PATH))?;

    // Add the sample plugin to the project
    env.setup_plugin()?;
    common::assert_file_exists(
        &temp_dir.join(".logcraft/plugins/sample"),
        true,
        "Plugin `sample` not found in testing project",
    );

    // Create a new command to create a service
    let mut command = process::Command::new(&env.bin_path);
    command.args([
        "services",
        "create",
        "-i",
        SERVICE_NAME,
        "-e",
        ENVIRONMENT_NAME,
        "-p",
        common::PLUGIN_NAME,
    ]);
    command.current_dir(&temp_dir);

    let mut session = spawn_command(command, None)?;
    session.exp_string("Do you want to configure the service now?")?;
    session.send_line("n")?;
    session.exp_string(&format!("service `{}` successfully created", SERVICE_NAME))?;
    session.exp_eof()?;

    // Start detection validation
    let mut command = process::Command::new(&env.bin_path);
    command.args(["validate"]);
    command.current_dir(&temp_dir);

    // Create sample plugin rule directory
    let rule_dir = temp_dir.join(DEFAULT_WORKSPACE).join("noplugin");
    std::fs::create_dir_all(&rule_dir)?;

    // Create sample detection file in the project workspace
    let detection_file = temp_dir
        .join(DEFAULT_WORKSPACE)
        .join("noplugin")
        .join("sample.rule");
    std::fs::write(&detection_file, SAMPLE_RULE)?;

    let mut session = spawn_command(command, None)?;
    session.exp_string(&format!(
        "folder `{}/noplugin` has no plugin associated",
        DEFAULT_WORKSPACE
    ))?;
    session.exp_string("all good, no problem identified.")?;
    session.exp_eof()?;

    Ok(())
}

/// Test that running validate command with a rule that has no plugin passes with expected output.
#[test]
fn test_validate_incorrect_values() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    let mut env = common::TestingEnv::init(false, temp_dir.path(), None, true)?;
    env.session
        .exp_string(&format!("`{}` saved", LGC_CONFIG_PATH))?;

    // Add the sample plugin to the project
    env.setup_plugin()?;
    common::assert_file_exists(
        &temp_dir.join(".logcraft/plugins/sample"),
        true,
        "Plugin `sample` not found in testing project",
    );

    // Create a new command to create a service
    let mut command = process::Command::new(&env.bin_path);
    command.args([
        "services",
        "create",
        "-i",
        SERVICE_NAME,
        "-e",
        ENVIRONMENT_NAME,
        "-p",
        common::PLUGIN_NAME,
    ]);
    command.current_dir(&temp_dir);

    let mut session = spawn_command(command, None)?;
    session.exp_string("Do you want to configure the service now?")?;
    session.send_line("n")?;
    session.exp_string(&format!("service `{}` successfully created", SERVICE_NAME))?;
    session.exp_eof()?;

    // Start detection validation
    let mut command = process::Command::new(&env.bin_path);
    command.args(["validate"]);
    command.current_dir(&temp_dir);

    // Create sample plugin rule directory
    let rule_dir = temp_dir.join(DEFAULT_WORKSPACE).join(PLUGIN_NAME);
    std::fs::create_dir_all(&rule_dir)?;

    // Create sample detection file in the project workspace
    let detection_file = temp_dir
        .join(DEFAULT_WORKSPACE)
        .join(PLUGIN_NAME)
        .join("sample.yaml");
    std::fs::write(&detection_file, INVALID_SAMPLE_RULE)?;

    let mut session = spawn_command(command, None)?;
    session.exp_string("validation failed on")?;
    session.exp_eof()?;

    Ok(())
}
