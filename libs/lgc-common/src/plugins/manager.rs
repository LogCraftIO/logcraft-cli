// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use anyhow::{anyhow, bail};
use std::{fs, path};
use wasmtime::Store;

use lgc_runtime::{
    plugin_component::plugin::{BytesParam, BytesResult, Metadata},
    state::State,
    Config, Engine, Plugins, DEFAULT_EPOCH_TICK_INTERVAL,
};

pub struct InstanceData {
    interface: Plugins,
    pub metadata: Metadata,
}

#[derive(Clone)]
pub struct PluginManager {
    engine: Engine,
}

impl PluginManager {
    pub fn new() -> anyhow::Result<Self> {
        // Setup wasmtime
        let mut config = Config::default();
        if let Err(e) = config.enable_cache(&None) {
            tracing::warn!(err = ?e, "failed to load wasm cache");
            bail!("{e}")
        };

        let engine = Engine::builder(&config)?.build();

        Ok(Self { engine })
    }

    pub async fn load_plugin(
        &self,
        path: impl AsRef<path::Path>,
    ) -> anyhow::Result<(InstanceData, Store<State>)> {
        // Load the component
        let mut store = wasmtime::Store::new(&self.engine.inner, State::default());

        // TODO: Check for better value
        let deadline = std::time::Duration::from_secs(60);
        store.set_epoch_deadline(
            (deadline.as_micros() / DEFAULT_EPOCH_TICK_INTERVAL.as_micros()) as u64,
        );

        let component = wasmtime::component::Component::from_file(&self.engine.inner, path)?;
        let interface =
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

    pub fn plugin_names(&self, base_dir: impl AsRef<path::Path>) -> anyhow::Result<Vec<String>> {
        fs::read_dir(base_dir)
        .map(|entries| {
            entries.filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.extension()?.to_str()? == "wasm" {
                    path.file_stem()?.to_str().map(String::from)
                } else {
                    None
                }
            }).collect()
        })
        .map_err(|e| e.into())
    }
}

/// Designed to be able to execute requests in parallel.
/// Must apparently be colocated with the Store. Maybe not useful for the moment
pub trait PluginActions: Send + 'static {
    fn load(
        &self,
        store: &mut Store<State>,
    ) -> impl std::future::Future<Output = anyhow::Result<Metadata>> + Send;
    fn settings(
        &self,
        store: &mut Store<State>,
    ) -> impl std::future::Future<Output = anyhow::Result<BytesResult>> + Send;
    fn schema(
        &self,
        store: &mut Store<State>,
    ) -> impl std::future::Future<Output = anyhow::Result<BytesResult>> + Send;
    fn create(
        &self,
        store: &mut Store<State>,
        config: BytesParam<'_>,
        detection: BytesParam<'_>,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
    fn read(
        &self,
        store: &mut Store<State>,
        config: BytesParam<'_>,
        detection: BytesParam<'_>,
    ) -> impl std::future::Future<Output = anyhow::Result<Option<BytesResult>>> + Send;
    fn update(
        &self,
        store: &mut Store<State>,
        config: BytesParam<'_>,
        detection: BytesParam<'_>,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
    fn delete(
        &self,
        store: &mut Store<State>,
        config: BytesParam<'_>,
        detection: BytesParam<'_>,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
    fn ping(
        &self,
        store: &mut Store<State>,
        config: BytesParam<'_>,
    ) -> impl std::future::Future<Output = anyhow::Result<bool>> + Send;
    fn validate(
        &self,
        store: &mut Store<State>,
        config: BytesParam,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

impl PluginActions for InstanceData {
    async fn load(&self, store: &mut Store<State>) -> anyhow::Result<Metadata> {
        self.interface.logcraft_lgc_plugin().call_load(store).await
    }

    async fn settings(&self, store: &mut Store<State>) -> anyhow::Result<BytesResult> {
        match self
            .interface
            .logcraft_lgc_plugin()
            .call_settings(store)
            .await
        {
            Ok(inner_result) => inner_result.map_err(|e| anyhow!(e)),
            Err(e) => Err(anyhow!(e)),
        }
    }

    async fn schema(&self, store: &mut Store<State>) -> anyhow::Result<BytesResult> {
        match self
            .interface
            .logcraft_lgc_plugin()
            .call_schema(store)
            .await
        {
            Ok(inner_result) => inner_result.map_err(|e| anyhow!(e)),
            Err(e) => Err(anyhow!(e)),
        }
    }

    async fn create(
        &self,
        store: &mut Store<State>,
        config: BytesParam<'_>,
        detection: BytesParam<'_>,
    ) -> anyhow::Result<()> {
        match self
            .interface
            .logcraft_lgc_plugin()
            .call_create(store, config, detection)
            .await
        {
            Ok(inner_result) => inner_result.map_err(|e| anyhow!(e)),
            Err(e) => Err(anyhow!(e)),
        }
    }

    async fn read(
        &self,
        store: &mut Store<State>,
        config: BytesParam<'_>,
        detection: BytesParam<'_>,
    ) -> anyhow::Result<Option<BytesResult>> {
        match self
            .interface
            .logcraft_lgc_plugin()
            .call_read(store, config, detection)
            .await
        {
            Ok(inner_result) => inner_result.map_err(|e| anyhow!(e)),
            Err(e) => Err(anyhow!(e)),
        }
    }

    async fn update(
        &self,
        store: &mut Store<State>,
        config: BytesParam<'_>,
        detection: BytesParam<'_>,
    ) -> anyhow::Result<()> {
        match self
            .interface
            .logcraft_lgc_plugin()
            .call_update(store, config, detection)
            .await
        {
            Ok(inner_result) => inner_result.map_err(|e| anyhow!(e)),
            Err(e) => Err(anyhow!(e)),
        }
    }

    async fn delete(
        &self,
        store: &mut Store<State>,
        config: BytesParam<'_>,
        detection: BytesParam<'_>,
    ) -> anyhow::Result<()> {
        match self
            .interface
            .logcraft_lgc_plugin()
            .call_delete(store, config, detection)
            .await
        {
            Ok(inner_result) => inner_result.map_err(|e| anyhow!(e)),
            Err(e) => Err(anyhow!(e)),
        }
    }

    async fn ping(&self, store: &mut Store<State>, config: BytesParam<'_>) -> anyhow::Result<bool> {
        match self
            .interface
            .logcraft_lgc_plugin()
            .call_ping(store, config)
            .await
        {
            Ok(inner_result) => inner_result.map_err(|e| anyhow!(e)),
            Err(e) => Err(anyhow!(e)),
        }
    }

    async fn validate(
        &self,
        store: &mut Store<State>,
        detection: BytesParam<'_>,
    ) -> anyhow::Result<()> {
        match self
            .interface
            .logcraft_lgc_plugin()
            .call_validate(store, detection)
            .await
        {
            Ok(inner_result) => inner_result.map_err(|e| anyhow!(e)),
            Err(e) => Err(anyhow!(e)),
        }
    }
}
