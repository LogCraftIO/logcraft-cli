// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

mod engine;
pub mod state;
use std::time::Duration;

pub use engine::{Config, Engine};

/// The default [`EngineBuilder::epoch_tick_interval`].
pub const DEFAULT_EPOCH_TICK_INTERVAL: Duration = Duration::from_millis(10);

wasmtime::component::bindgen!({
    path: "../../wit",
    async: true
});

pub mod plugin_component {
    pub use crate::exports::logcraft::host::plugin;
    pub use crate::Interfaces;
}
