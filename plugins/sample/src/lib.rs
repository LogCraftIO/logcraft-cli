// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use bindings::{
    export,
    exports::logcraft::lgc::plugin::{Bytes, Guest, Metadata},
};
use once_cell::sync::Lazy;

mod bindings {
    wit_bindgen::generate!({
        path: "../../libs/bindings",
        world: "logcraft:lgc/plugins"
    });
}

// Local modules
mod backend;
mod schema;
use backend::SampleBackend;
use schema::SampleRule;

// static BACKEND_SCHEMA: Lazy<serde_json::Value> = Lazy::new(|| {
//     serde_json::to_value(
//         schemars::schema_for!(SampleBackend)
//     )
//     .expect("Failed to generate schema")
// });

static RULE_SCHEMA: Lazy<serde_json::Value> = Lazy::new(|| {
    serde_json::to_value(schemars::schema_for!(SampleRule)).expect("Failed to generate schema")
});

static SCHEMA_VALIDATOR: Lazy<jsonschema::Validator> = Lazy::new(|| {
    jsonschema::validator_for(&RULE_SCHEMA).expect("Failed to create schema validator")
});

impl Guest for SampleBackend {
    /// Retrieve plugin metadata
    fn load() -> Metadata {
        Metadata {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Retrieve plugin settings schema
    fn settings() -> Result<Bytes, String> {
        match serde_json::to_vec(&schemars::schema_for!(SampleBackend)) {
            Ok(schema) => Ok(schema),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Retrieve plugin detection schema
    fn schema() -> Result<Bytes, String> {
        match serde_json::to_vec(&schemars::schema_for!(SampleRule)) {
            Ok(schema) => Ok(schema),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Validate detection rule
    fn validate(detection: Bytes) -> Result<(), String> {
        let detection: SampleRule = serde_yaml_ng::from_slice(&detection)
            .map_err(|e| format!("Failed to deserialize detection content: {e}"))?;

        if let Err(e) = SCHEMA_VALIDATOR.validate(
            &serde_json::to_value(&detection)
                .map_err(|e| format!("Failed to serialize detection content: {e}"))?,
        ) {
            return Err(format!(
                "Detection content failed schema validation at {}: {}",
                e.instance, e.instance_path
            ));
        }

        Ok(())
    }

    /// Create SavedSearch
    fn create(_config: Bytes, _detection: Bytes) -> Result<(), String> {
        unimplemented!()
    }

    /// Get SavedSearch
    fn read(_config: Bytes, _detection: Bytes) -> Result<Option<Bytes>, String> {
        unimplemented!()
    }

    /// Update SavedSearch
    fn update(_config: Bytes, _detection: Bytes) -> Result<(), String> {
        unimplemented!()
    }

    /// Delete SavedSearch
    fn delete(_config: Bytes, _detection: Bytes) -> Result<(), String> {
        unimplemented!()
    }

    /// Ping service
    fn ping(_config: Bytes) -> Result<bool, String> {
        unimplemented!()
    }
}

export!(SampleBackend with_types_in bindings);
