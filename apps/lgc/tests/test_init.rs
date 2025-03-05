// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use anyhow::Result;
use std::path::Path;

use lgc_common::configuration::LGC_CONFIG_PATH;

pub mod common;

/// Test that initializing a project with the default configuration succeeds.
#[test]
fn init_default_command_without_create() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    common::TestingEnv::init(false, temp_dir.path(), None, false)?.init_success()?;

    let config_file = temp_dir.join(LGC_CONFIG_PATH);
    common::assert_file_exists(&config_file, true, "Expected the config file to be created");

    let workspace_dir = temp_dir.join(common::DEFAULT_WORKSPACE);
    common::assert_file_exists(
        &workspace_dir,
        false,
        "Workspace should not exist if '--create' was not given",
    );

    Ok(())
}

/// Test that initializing a project with the default configuration and creating the workspace succeeds.
#[test]
fn init_default_command_with_create() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    let mut env = common::TestingEnv::init(false, &temp_dir, None, true)?;

    env.session.exp_string(&format!(
        "workspace directory '{}' created",
        common::DEFAULT_WORKSPACE
    ))?;
    env.init_success()?;

    let workspace_dir = temp_dir.join(common::DEFAULT_WORKSPACE);
    common::assert_file_exists(&workspace_dir, true, "Expected workspace to be created");

    let config_file = temp_dir.join(LGC_CONFIG_PATH);
    common::assert_file_exists(&config_file, true, "Expected the config file to be created");

    Ok(())
}

/// Test that initializing a project with a custom root path and workspace succeeds.
#[test]
fn init_custom_root_and_workspace() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    // Create a subdirectory for the workspace
    let root_dir = temp_dir.path().join("custom_root");
    std::fs::create_dir(&root_dir).expect("Failed to create custom root directory");

    let mut env = common::TestingEnv::init(false, &root_dir, Some("custom_workspace"), true)?;

    env.session
        .exp_string("workspace directory 'custom_workspace' created")?;
    env.init_success()?;

    Ok(())
}

/// Test that initializing a project with an already existing workspace fails.
#[test]
fn init_workspace_conflict() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;

    // Create a subdirectory for the workspace
    let workspace_dir = temp_dir.join(common::DEFAULT_WORKSPACE);
    std::fs::create_dir(&workspace_dir).expect("Failed to pre-create workspace dir");

    let mut env = common::TestingEnv::init(false, &temp_dir, None, true)?;

    assert!(workspace_dir.exists(), "Expected workspace to exist");

    env.session.exp_string(&format!(
        "workspace directory '{}' already exists",
        common::DEFAULT_WORKSPACE
    ))?;

    let config_file = temp_dir.join(LGC_CONFIG_PATH);
    common::assert_file_exists(
        &config_file,
        false,
        "Expected the config file to be missing",
    );

    Ok(())
}

/// Test that initializing a project with an existing configuration file fails.
#[test]
fn init_config_conflict() -> Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;
    let mut env = common::TestingEnv::init(false, &temp_dir, None, false)?;

    let config_file = temp_dir.join(LGC_CONFIG_PATH);
    std::fs::File::create(&config_file).expect("Failed to pre-create config file");

    assert!(config_file.exists(), "Expected the config file to exist");

    env.session
        .exp_string(&format!("{} already exists", LGC_CONFIG_PATH))?;

    let workspace_dir = temp_dir.join(common::DEFAULT_WORKSPACE);
    common::assert_file_exists(&workspace_dir, false, "Expected workspace to be missing");

    Ok(())
}

/// Test that initializing a project with an invalid root path fails.
#[test]
fn init_invalid_root() -> Result<()> {
    // Create a temporary file
    let temp_dir = assert_fs::TempDir::new()?;
    let invalid_root = temp_dir.path().join("invalid_root");
    std::fs::File::create(&invalid_root).expect("Failed to create invalid root file");

    let mut env = common::TestingEnv::init(false, &invalid_root, None, false)?;
    env.session.exp_string(&format!(
        "'{}' is not a directory",
        invalid_root
            .to_str()
            .expect("Failed to convert invalid root path to string")
    ))?;

    Ok(())
}

/// Test that initializing a project with a missing root path fails.
#[test]
fn init_missing_root() -> Result<()> {
    // Path to a non-existent directory
    let missing_root = Path::new("/tmp/missing_root");
    let mut env = common::TestingEnv::init(false, missing_root, None, false)?;

    env.session
        .exp_regex(&format!("'{}' does not exist", missing_root.display()))?;

    Ok(())
}
