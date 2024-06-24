// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::Result;
use clap::Parser;
use kclvm_api::{gpyrpc::ValidateCodeArgs, service::KclvmServiceImpl};
use tokio::task::JoinSet;

use logcraft_common::{
    configuration::ProjectConfiguration,
    detections::map_plugin_detections,
    plugins::manager::{PluginActions, PluginManager},
};
/// Validate configuration
#[derive(Parser, Debug, Default)]
#[clap(about = "Validate local detection rules", allow_hyphen_values = true)]
pub struct ValidateCommand;

impl ValidateCommand {
    pub async fn run(self, config: &ProjectConfiguration) -> Result<()> {
        // Load all detections
        let detections = map_plugin_detections()?;

        // Load plugins
        let plugin_manager = PluginManager::new()?;
        let mut set = JoinSet::new();

        for plugin_name in detections.keys() {
            let plugin_name = plugin_name.to_string();
            let plugin_manager = plugin_manager.clone();
            set.spawn(async move { plugin_manager.load_plugin(plugin_name).await });
        }

        // Start kclvm service
        let serv = KclvmServiceImpl::default();
        let mut args = ValidateCodeArgs {
            format: String::from("yaml"),
            ..Default::default()
        };

        let mut has_err: bool = false;
        // Call get schema and retrieve all detections
        while let Some(plugin) = set.join_next().await {
            let (instance, mut store) = plugin??;
            let meta = &instance.metadata;

            // Safe unwrap as we load plugins with detection HashMap.
            let (plugin, rules) = detections.get_key_value(&meta.name).unwrap();

            // Check services
            args.code = instance.settings(&mut store).await?;
            args.schema = String::from("Configuration");
            for svc in config.services.iter().filter(|svc| &svc.plugin == plugin) {
                args.data = serde_yaml_ng::to_string(&svc.settings)?;
                let check = serv.validate_code(&args)?;
                if !check.success {
                    has_err = true;
                    tracing::error!("`{}`", check.err_message);
                }
            }

            // Check rules
            args.code = instance.schema(&mut store).await?;
            args.schema = String::from("Rule");
            for detection in rules {
                args.data = serde_yaml_ng::to_string(&detection.content)?;
                let check = serv.validate_code(&args)?;
                if !check.success {
                    has_err = true;
                    tracing::error!("`{}`", check.err_message);
                }
            }
        }

        if !has_err {
            tracing::info!("all good, no problems identified");
        }

        Ok(())
    }
}
