// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use std::time::Duration;

mod engine;
pub mod state;
pub use engine::{Config, Engine};

/// The default [`EngineBuilder::epoch_tick_interval`].
pub const DEFAULT_EPOCH_TICK_INTERVAL: Duration = Duration::from_millis(10);

// Plugins wit definition macro builder
#[cfg(debug_assertions)]
wasmtime::component::bindgen!({
    path: "../bindings",
    world: "logcraft:lgc/plugins",
    async: true,
    ownership: Borrowing {
        duplicate_if_necessary: true
    },
    tracing: true,  // Enable tracing in debug mode
});

#[cfg(not(debug_assertions))]
wasmtime::component::bindgen!({
    path: "../bindings",
    world: "logcraft:lgc/plugins",
    async: true,
    ownership: Borrowing {
        duplicate_if_necessary: true
    }
});

/// Plugin component bindings created by the wasm-bindgen macro
pub mod plugin_component {
    pub use crate::exports::logcraft::lgc::plugin;

    pub use crate::Plugins;
}
