// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use logcraft_runtime::{
    plugin_component::plugin::Metadata, state::State, Config, Engine, Plugins,
    DEFAULT_EPOCH_TICK_INTERVAL,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt, fs,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};
use tempfile::NamedTempFile;
use wasmtime::{component::Component, Store};

use crate::plugins::cleanup_plugin;

use super::LGC_PLUGINS_PATH;

pub struct InstanceData {
    interface: Plugins,
    pub metadata: Metadata,
}

#[derive(Clone)]
pub struct PluginManager {
    engine: Engine,
}

impl PluginManager {
    pub fn new() -> Result<Self> {
        // Setup wasmtime
        let mut config = Config::default();
        if let Err(e) = config.enable_cache(&None) {
            tracing::warn!(err = ?e, "failed to load wasm cache");
            bail!("{e}")
        };

        let engine = Engine::builder(&config)?.build();

        Ok(Self { engine })
    }

    pub async fn install_plugin(&self, location: &PluginLocation) -> Result<Metadata> {
        // Create and load plugin in temporary file
        let mut file = NamedTempFile::new()?;
        file.write_all(&location.load().await?)?;
        // Instanciate plugin
        let path = file.path();
        let (instance, _) = self.load_plugin(&path).await?;
        // Check if plugin directory exists
        let plugin_path = PathBuf::from(LGC_PLUGINS_PATH);
        if !plugin_path.exists() {
            fs::create_dir_all(&plugin_path)?;
        }

        // Copying file to avoid cross-device link error
        if let Err(e) = fs::copy(path, plugin_path.join(&instance.metadata.name)) {
            cleanup_plugin(&instance.metadata.name)?;
            bail!("failed to move loaded plugin to plugins directory: {}", e);
        };
        fs::remove_file(path)?;

        Ok(instance.metadata)
    }

    pub async fn load_plugin(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(InstanceData, Store<State>)> {
        // Load the component
        let path = PathBuf::from(LGC_PLUGINS_PATH).join(path);
        let component = Component::from_file(&self.engine.inner, path)?;

        let mut store = wasmtime::Store::new(&self.engine.inner, State::default());

        // TODO: Check for better value
        let deadline = Duration::from_secs(60);
        store.set_epoch_deadline(
            (deadline.as_micros() / DEFAULT_EPOCH_TICK_INTERVAL.as_micros()) as u64,
        );

        let (interface, _) =
        Plugins::instantiate_async(&mut store, &component, &self.engine.linker).await?;

        let metadata = interface
            .logcraft_lgc_plugin()
            .call_load(&mut store)
            .await?;

        Ok((
            InstanceData {
                interface,
                metadata: metadata.clone(),
            },
            store,
        ))
    }
}

/// Designed to be able to execute requests in parallel.
/// Must apparently be colocalized with the Store. Maybe not useful for the moment
#[async_trait]
pub trait PluginActions: Send + 'static {
    async fn load(&self, store: &mut Store<State>) -> Result<Metadata>;
    async fn settings(&self, store: &mut Store<State>) -> Result<String>;
    async fn schema(&self, store: &mut Store<State>) -> Result<String>;
    async fn create(
        &self,
        store: &mut Store<State>,
        config: &str,
        name: &str,
        params: &str,
    ) -> Result<Option<String>>;
    async fn read(
        &self,
        store: &mut Store<State>,
        config: &str,
        name: &str,
        params: &str,
    ) -> Result<Option<String>>;
    async fn update(
        &self,
        store: &mut Store<State>,
        config: &str,
        name: &str,
        params: &str,
    ) -> Result<Option<String>>;
    async fn delete(
        &self,
        store: &mut Store<State>,
        config: &str,
        name: &str,
        params: &str,
    ) -> Result<Option<String>>;
    async fn ping(&self, store: &mut Store<State>, config: &str) -> Result<bool>;
}

#[async_trait]
impl PluginActions for InstanceData {
    async fn load(&self, store: &mut Store<State>) -> Result<Metadata> {
        self.interface.logcraft_lgc_plugin().call_load(store).await
    }

    async fn settings(&self, store: &mut Store<State>) -> Result<String> {
        self.interface
            .logcraft_lgc_plugin()
            .call_settings(store)
            .await
    }

    async fn schema(&self, store: &mut Store<State>) -> Result<String> {
        self.interface
            .logcraft_lgc_plugin()
            .call_schema(store)
            .await
    }

    async fn create(
        &self,
        store: &mut Store<State>,
        config: &str,
        name: &str,
        params: &str,
    ) -> Result<Option<String>> {
        self.interface
            .logcraft_lgc_plugin()
            .call_create(store, config, name, params)
            .await
            .map_err(|e| {
                anyhow!(
                    "when calling read for plugin `{}`: {}",
                    self.metadata.name,
                    e
                )
            })?
            .map_err(|e| {
                anyhow!(
                    "when calling create for plugin `{}`: {}",
                    self.metadata.name,
                    e
                )
            })
    }

    async fn read(
        &self,
        store: &mut Store<State>,
        config: &str,
        name: &str,
        params: &str,
    ) -> Result<Option<String>> {
        self.interface
            .logcraft_lgc_plugin()
            .call_read(store, config, name, params)
            .await?
            .map_err(|e| {
                anyhow!(
                    "when calling read for plugin `{}`: {}",
                    self.metadata.name,
                    e
                )
            })
    }

    async fn update(
        &self,
        store: &mut Store<State>,
        config: &str,
        name: &str,
        params: &str,
    ) -> Result<Option<String>> {
        self.interface
            .logcraft_lgc_plugin()
            .call_update(store, config, name, params)
            .await?
            .map_err(|e| {
                anyhow!(
                    "when calling update for plugin `{}`: {}",
                    self.metadata.name,
                    e
                )
            })
    }

    async fn delete(
        &self,
        store: &mut Store<State>,
        config: &str,
        name: &str,
        params: &str,
    ) -> Result<Option<String>> {
        self.interface
            .logcraft_lgc_plugin()
            .call_delete(store, config, name, params)
            .await?
            .map_err(|e| {
                anyhow!(
                    "when calling delete for plugin `{}`: {}",
                    self.metadata.name,
                    e
                )
            })
    }

    async fn ping(&self, store: &mut Store<State>, config: &str) -> Result<bool> {
        self.interface
            .logcraft_lgc_plugin()
            .call_ping(store, config)
            .await?
            .map_err(|e| {
                anyhow!(
                    "when calling ping for plugin `{}`: {}",
                    self.metadata.name,
                    e
                )
            })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
// #[serde(tag = "type")]
#[serde(tag = "type", content = "location")]
pub enum PluginLocation {
    /// Fetch plugin from local path
    Local(PathBuf),
    // /// Fetch plugin from remote url
    // Remote(Url),
    // /// Fetch plugin from OCI registry
    // Oci(image)
}

impl fmt::Display for PluginLocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PluginLocation::Local(path) => write!(f, "source: {}", path.to_str().unwrap()),
            // PluginLocation::Remote(url) => write!(f, "source: {}", path.to_str().unwrap()),
            // PluginLocation::Oci(image) => write!(f, "source: {}", path.to_str().unwrap()),
        }
    }
}

impl Default for PluginLocation {
    fn default() -> Self {
        PluginLocation::Local(PathBuf::new())
    }
}

impl PluginLocation {
    pub async fn load(&self) -> Result<Vec<u8>> {
        match &self {
            Self::Local(path) => {
                // copy(path, &plugin_path)?;
                tokio::fs::read(path)
                    .await
                    .map_err(|e| anyhow!("reading plugin file: {}", e))
            } // Self::Remote(url) => {
              //   // Retrieve remote file
              //   let resp = reqwest::get(url.as_str())
              //     .await
              //     .context("unable to retrieve remote plugin file")?;

              //   if !resp.status().is_success() {
              //     bail!("unable to fetch plugin file from {}\nStatus: {}", url, resp.status());
              //   };

              //   let mut reader = StreamReader::new(
              //     resp.bytes_stream().map_err(IoError::other)
              //   );
              //   let mut buff = Vec::new();
              //   let _ = tokio::io::copy(&mut reader, &mut buff).await.context("Unable to save plugin to disk")?;
              //   Ok(buff)
              // }
        }
    }
}
