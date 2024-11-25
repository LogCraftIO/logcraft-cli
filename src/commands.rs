// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

// Commands
mod deploy;
mod destroy;
mod diff;
mod init;
mod validate;
// Subcommands
mod environments;
pub mod plugins;
pub mod services;

// Re-exporting the commands
pub use {
    // Commands
    deploy::DeployCommand,
    destroy::DestroyCommand,
    diff::DiffCommand,
    init::InitCommand,
    validate::ValidateCommand,
    // Subcommands
    environments::EnvironmentsCommands,
    plugins::PluginsCommands,
    services::ServicesCommands,
};