// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use anyhow::Result;
use rexpect::session::spawn_command;
use std::process;

use lgc_common::configuration::LGC_CONFIG_PATH;

pub mod common;

const SERVICE_NAME: &str = "my-service";
const ENVIRONMENT_NAME: &str = "testing";

/// Test that running service command without an initialized project fails.
#[test]
fn service_command_no_project() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    let mut env = common::TestingEnv::init(false, temp_dir.path(), None, false)?;
    env.session
        .exp_string(&format!("{} saved", LGC_CONFIG_PATH))?;

    // Remove `lgc.toml` to simulate a project that has not been initialized
    let config_file = temp_dir.path().join(LGC_CONFIG_PATH);
    std::fs::remove_file(&config_file)?;

    // Run the service command
    let mut command = process::Command::new(&env.bin_path);
    command.args(["services", "list"]);
    command.current_dir(&temp_dir);

    let mut session = spawn_command(command, None)?;
    session.exp_string("no configuration file, run 'lgc init' to initialize a new project")?;
    session.exp_eof()?;

    Ok(())
}

/// Test that initializing a project with the default configuration succeeds.
#[test]
fn service_create_no_configuration() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    let mut env = common::TestingEnv::init(false, temp_dir.path(), None, false)?;
    env.session
        .exp_string(&format!("{} saved", LGC_CONFIG_PATH))?;

    // Add the sample plugin to the project
    env.setup_plugin()?;

    common::assert_file_exists(
        &temp_dir.join(".logcraft/plugins/sample.wasm"),
        true,
        "Plugin 'sample' not found in testing project",
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
    session.exp_string(&format!("service '{}' successfully created", SERVICE_NAME))?;
    session.exp_eof()?;

    Ok(())
}

/// Test creating a service with configuration.
#[test]
fn service_create_with_configuration() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    let mut env = common::TestingEnv::init(false, temp_dir.path(), None, false)?;
    env.session
        .exp_string(&format!("{} saved", LGC_CONFIG_PATH))?;

    // Add the sample plugin to the project
    env.setup_plugin()?;
    common::assert_file_exists(
        &temp_dir.join(".logcraft/plugins/sample.wasm"),
        true,
        "Plugin 'sample' not found in testing project",
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
    session.send_line("y")?;
    // Send `return` to skip the configuration
    // Sample plugin has 6 parameters
    for _ in 0..6 {
        session.send_line("")?;
    }
    session.exp_string(&format!("service '{}' successfully created", SERVICE_NAME))?;
    session.exp_eof()?;

    Ok(())
}

/// Test listing services when none exist.
#[test]
fn service_list_empty() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    let mut env = common::TestingEnv::init(false, temp_dir.path(), None, false)?;
    env.session
        .exp_string(&format!("{} saved", LGC_CONFIG_PATH))?;

    // List services when none exist
    let mut command = process::Command::new(&env.bin_path);
    command.args(["services", "list"]);
    command.current_dir(&temp_dir);

    let mut session = spawn_command(command, None)?;
    session.exp_eof()?;

    Ok(())
}

/// Test creating a service with invalid identifier, environment, or plugin.
#[test]
fn service_create_invalid_values() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    let mut env = common::TestingEnv::init(false, temp_dir.path(), None, false)?;
    env.session
        .exp_string(&format!("{} saved", LGC_CONFIG_PATH))?;

    // Add the sample plugin to the project
    env.setup_plugin()?;

    // Test invalid identifier
    let mut command = process::Command::new(&env.bin_path);
    command.args([
        "services",
        "create",
        "-i",
        "invalid identifier!",
        "-e",
        ENVIRONMENT_NAME,
        "-p",
        common::PLUGIN_NAME,
    ]);
    command.current_dir(&temp_dir);

    let mut session = spawn_command(command, Some(5_000))?;
    session.exp_string("invalid format `invalid identifier!`, must be kebab-case")?;
    session.exp_eof()?;

    // Test invalid environment
    let mut command = process::Command::new(&env.bin_path);
    command.args([
        "services",
        "create",
        "-i",
        SERVICE_NAME,
        "-e",
        "invalid environment!",
        "-p",
        common::PLUGIN_NAME,
    ]);
    command.current_dir(&temp_dir);

    let mut session = spawn_command(command, Some(5_000))?;
    session.exp_string("invalid format `invalid environment!`, must be kebab-case")?;
    session.exp_eof()?;

    // Test invalid plugin
    let mut command = process::Command::new(&env.bin_path);
    command.args([
        "services",
        "create",
        "-i",
        SERVICE_NAME,
        "-e",
        ENVIRONMENT_NAME,
        "-p",
        "non-existent-plugin",
    ]);
    command.current_dir(&temp_dir);

    let mut session = spawn_command(command, None)?;
    session.exp_string("plugin 'non-existent-plugin' does not exist")?;
    session.exp_eof()?;

    Ok(())
}

/// Test listing services in the configuration.
#[test]
fn service_list() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    let mut env = common::TestingEnv::init(false, temp_dir.path(), None, false)?;
    env.session
        .exp_string(&format!("{} saved", LGC_CONFIG_PATH))?;

    // Add the sample plugin to the project
    env.setup_plugin()?;
    common::assert_file_exists(
        &temp_dir.join(".logcraft/plugins/sample.wasm"),
        true,
        "Plugin 'sample' not found in testing project",
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
    session.exp_string(&format!("service '{}' successfully created", SERVICE_NAME))?;
    session.exp_eof()?;

    // List services
    let mut command = process::Command::new(&env.bin_path);
    command.args(["services", "list"]);
    command.current_dir(&temp_dir);

    let mut session = spawn_command(command, Some(5_000))?;
    session.exp_string("---")?;
    session.exp_regex("service\\s+:.*\nenvironment:.*\nplugin\\s+:.*")?;
    session.exp_eof()?;

    Ok(())
}

/// Test removing a service from the configuration.
#[test]
fn service_remove() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    let mut env = common::TestingEnv::init(false, temp_dir.path(), None, false)?;
    env.session
        .exp_string(&format!("{} saved", LGC_CONFIG_PATH))?;

    // Add the sample plugin to the project
    env.setup_plugin()?;
    common::assert_file_exists(
        &temp_dir.join(".logcraft/plugins/sample.wasm"),
        true,
        "Plugin 'sample' not found in testing project",
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
    session.exp_string(&format!("service '{}' successfully created", SERVICE_NAME))?;
    session.exp_eof()?;

    // Remove the service
    let mut command = process::Command::new(&env.bin_path);
    command.args(["services", "remove", SERVICE_NAME]);
    command.current_dir(&temp_dir);

    let mut session = spawn_command(command, None)?;
    session.exp_string(&format!("service '{}' successfully removed", SERVICE_NAME))?;
    session.exp_eof()?;

    Ok(())
}

/// Test removing a service that is not defined in the configuration.
#[test]
fn service_remove_non_existent() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    let mut env = common::TestingEnv::init(false, temp_dir.path(), None, false)?;
    env.session
        .exp_string(&format!("{} saved", LGC_CONFIG_PATH))?;

    // Add the sample plugin to the project
    env.setup_plugin()?;
    common::assert_file_exists(
        &temp_dir.join(".logcraft/plugins/sample.wasm"),
        true,
        "Plugin 'sample' not found in testing project",
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
    session.exp_string(&format!("service '{}' successfully created", SERVICE_NAME))?;
    session.exp_eof()?;

    // Remove a service that does not exist
    let mut command = process::Command::new(&env.bin_path);
    command.args(["services", "remove", "non-existent-service"]);
    command.current_dir(&temp_dir);

    let mut session = spawn_command(command, None)?;
    session.exp_string("service 'non-existent-service' not found")?;
    session.exp_eof()?;

    Ok(())
}

/// Test removing a service that is not defined in the configuration.
#[test]
fn service_configure_non_existent() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    let mut env = common::TestingEnv::init(false, temp_dir.path(), None, false)?;
    env.session
        .exp_string(&format!("{} saved", LGC_CONFIG_PATH))?;

    // Add the sample plugin to the project
    env.setup_plugin()?;
    common::assert_file_exists(
        &temp_dir.join(".logcraft/plugins/sample.wasm"),
        true,
        "Plugin 'sample' not found in testing project",
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
    session.exp_string(&format!("service '{}' successfully created", SERVICE_NAME))?;
    session.exp_eof()?;

    // Remove a service that does not exist
    let mut command = process::Command::new(&env.bin_path);
    command.args(["services", "configure", "non-existent-service"]);
    command.current_dir(&temp_dir);

    let mut session = spawn_command(command, None)?;
    session.exp_string("service 'non-existent-service' not found")?;
    session.exp_eof()?;

    Ok(())
}

/// Test configuring a service interactively.
#[test]
fn service_configure() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    let mut env = common::TestingEnv::init(false, temp_dir.path(), None, false)?;
    env.session
        .exp_string(&format!("{} saved", LGC_CONFIG_PATH))?;

    // Add the sample plugin to the project
    env.setup_plugin()?;
    common::assert_file_exists(
        &temp_dir.join(".logcraft/plugins/sample.wasm"),
        true,
        "Plugin 'sample' not found in testing project",
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
    session.exp_string(&format!("service '{}' successfully created", SERVICE_NAME))?;
    session.exp_eof()?;

    // Configure the service
    let mut command = process::Command::new(&env.bin_path);
    command.args(["services", "configure", SERVICE_NAME]);
    command.current_dir(&temp_dir);

    let mut session = spawn_command(command, None)?;
    for _ in 0..6 {
        session.send_line("")?;
    }
    session.exp_string(&format!("service '{}' configured", SERVICE_NAME))?;
    session.exp_eof()?;

    Ok(())
}

/// Test remove and configure commands with no service defined.
#[test]
fn service_commands_empty_service_list() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    let mut env = common::TestingEnv::init(false, temp_dir.path(), None, false)?;
    env.session
        .exp_string(&format!("{} saved", LGC_CONFIG_PATH))?;

    // Remove a service that does not exist
    let mut command = process::Command::new(&env.bin_path);
    command.args(["services", "remove", SERVICE_NAME]);
    command.current_dir(&temp_dir);

    let mut session = spawn_command(command, None)?;
    session.exp_string("no services defined")?;
    session.exp_eof()?;

    // Configure a service that does not exist
    let mut command = process::Command::new(&env.bin_path);
    command.args(["services", "configure", SERVICE_NAME]);
    command.current_dir(&temp_dir);

    let mut session = spawn_command(command, None)?;
    session.exp_string("no services defined")?;
    session.exp_eof()?;

    Ok(())
}
