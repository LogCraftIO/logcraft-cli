// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, str::FromStr};

pub mod manager;
pub use manager::PluginLocation;
use url::Url;

pub const LGC_PLUGINS_PATH: &str = ".logcraft";

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Plugin {
    pub source: PluginLocation,
    pub author: String,
    pub description: String,
    pub version: String,
}

pub fn cleanup_plugin(name: &str) -> Result<()> {
    let plugin_path = PathBuf::from(LGC_PLUGINS_PATH).join(name);
    if plugin_path.exists() {
        fs::remove_file(plugin_path)?;
    }

    if fs::read_dir(LGC_PLUGINS_PATH)?.count() == 0 {
        fs::remove_dir(LGC_PLUGINS_PATH)?;
    }

    Ok(())
}

pub fn determine_plugin_location(source: &str) -> Result<PluginLocation> {
    match Url::parse(source) {
        Ok(uri) => match uri.scheme() {
            "http" | "https" => bail!("not implemented yet"),
            "oci" => bail!("not implemented yet"),
            _ => bail!("invalid scheme: {}", uri.scheme()),
        },
        Err(_) => {
            let path = PathBuf::from_str(source)?;
            if path.is_file() {
                Ok(PluginLocation::Local(path))
            } else {
                bail!("provided path does not target a file")
            }
        }
    }
}
