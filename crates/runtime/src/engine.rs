// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::Result;
use crossbeam_channel::Sender;
use std::sync::Arc;
use std::{path::PathBuf, time::Duration};
use wasmtime::component::Linker;
use wasmtime::{InstanceAllocationStrategy, PoolingAllocationConfig};

/// The default [`EngineBuilder::epoch_tick_interval`].
pub const DEFAULT_EPOCH_TICK_INTERVAL: Duration = Duration::from_millis(10);

use crate::state::State;

const MB: u64 = 1 << 20;
const GB: u64 = 1 << 30;
const WASM_PAGE_SIZE: u64 = 64 * 1024;

/// Global configuration for `EngineBuilder`.
///
/// This is currently only used for advanced (undocumented) use cases.
pub struct Config {
    inner: wasmtime::Config,
}

impl Config {
    /// Enable the Wasmtime compilation cache. If `path` is given it will override
    /// the system default path.
    ///
    /// For more information, see the [Wasmtime cache config documentation][docs].
    ///
    /// [docs]: https://docs.wasmtime.dev/cli-cache.html
    pub fn enable_cache(&mut self, config_path: &Option<PathBuf>) -> Result<()> {
        match config_path {
            Some(p) => self.inner.cache_config_load(p)?,
            None => self.inner.cache_config_load_default()?,
        };

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut inner = wasmtime::Config::new();
        inner.async_support(true);
        inner.epoch_interruption(true);
        inner.wasm_component_model(true);

        // TODO: Enable if compilation time is acceptable.
        // * Components must be compiled at Plugin installation
        // * Cache will keep the latest installed plugin for fast loading
        // inner.cranelift_opt_level(wasmtime::OptLevel::SpeedAndSize);

        // By default enable the pooling instance allocator in Wasmtime. This
        // drastically reduces syscall/kernel overhead for wasm execution,
        // especially in async contexts where async stacks must be allocated.
        // The general goal here is that the default settings here rarely, if
        // ever, need to be modified. As a result there aren't fine-grained
        // knobs for each of these settings just yet and instead they're
        // generally set to defaults. Environment-variable-based fallbacks are
        // supported though as an escape valve for if this is a problem.
        let mut pooling_config = PoolingAllocationConfig::default();

        pooling_config
            .total_component_instances(1_000)
            // This number accounts for internal data structures that Wasmtime allocates for each instance.
            // Instance allocation is proportional to the number of "things" in a wasm module like functions,
            // globals, memories, etc. Instance allocations are relatively small and are largely inconsequential
            // compared to other runtime state, but a number needs to be chosen here so a relatively large threshold
            // of 10MB is arbitrarily chosen. It should be unlikely that any reasonably-sized module hits this limit.
            .max_component_instance_size(MB as usize)
            .max_core_instances_per_component(200)
            .max_tables_per_component(20)
            .table_elements(30_000)
            // The number of memories an instance can have effectively limits the number of inner components
            // a composed component can have (since each inner component has its own memory). We default to 32 for now, and
            // we'll see how often this limit gets reached.
            .max_memories_per_component(32)
            .total_memories(1_000)
            .total_tables(2_000)
            // Nothing is lost from allowing the maximum size of memory for
            // all instance as it's still limited through other the normal
            // `StoreLimitsAsync` accounting method too.
            .memory_pages(4 * GB / WASM_PAGE_SIZE)
            // These numbers are completely arbitrary at something above 0.
            .linear_memory_keep_resident((2 * MB) as usize)
            .table_keep_resident((MB / 2) as usize);

        inner.allocation_strategy(InstanceAllocationStrategy::Pooling(pooling_config));

        Self { inner }
    }
}

pub struct EngineBuilder {
    engine: wasmtime::Engine,
    linker: Linker<State>,
    epoch_tick_interval: Duration,
}

impl EngineBuilder {
    fn new(config: &Config) -> Result<Self> {
        let engine = wasmtime::Engine::new(&config.inner)?;
        let mut linker: Linker<State> = Linker::new(&engine);

        // Add wasi and wasi_http to linker
        wasmtime_wasi::add_to_linker_async(&mut linker)?;
        wasmtime_wasi_http::proxy::add_only_http_to_linker(&mut linker)?;

        Ok(Self {
            engine,
            linker,
            epoch_tick_interval: DEFAULT_EPOCH_TICK_INTERVAL,
        })
    }

    fn spawn_epoch_ticker(&self) -> Sender<()> {
        let engine = self.engine.clone();
        let interval = self.epoch_tick_interval;
        let (send, recv) = crossbeam_channel::bounded(0);
        std::thread::spawn(move || loop {
            match recv.recv_timeout(interval) {
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => (),
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                res => panic!("unexpected epoch_ticker_signal: {res:?}"),
            }
            engine.increment_epoch();
        });
        send
    }

    /// Builds an [`Engine`] from this builder.
    pub fn build(self) -> Engine {
        Engine {
            _epoch_ticker_signal: self.spawn_epoch_ticker(),
            inner: self.engine,
            linker: Arc::new(self.linker),
        }
    }
}

/// An `Engine` is a global context for the initialization and execution of components
#[derive(Clone)]
pub struct Engine {
    pub inner: wasmtime::Engine,
    pub linker: Arc<Linker<State>>,
    // Matching receiver closes on drop
    _epoch_ticker_signal: Sender<()>,
}

impl AsRef<wasmtime::Engine> for Engine {
    fn as_ref(&self) -> &wasmtime::Engine {
        &self.inner
    }
}

impl Engine {
    /// Creates a new [`EngineBuilder`] with the given [`Config`].
    pub fn builder(config: &Config) -> Result<EngineBuilder> {
        EngineBuilder::new(config)
    }
}
