// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

mod engine;
pub mod state;
pub use engine::{Config, Engine};

wasmtime::component::bindgen!({
    path: "../../wit",
    async: true
});

pub mod plugin_component {
    pub use crate::exports::logcraft::host::plugin;
    pub use crate::Interfaces;
}
