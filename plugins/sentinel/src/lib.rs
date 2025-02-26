// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use bindings::{
    export,
    exports::logcraft::lgc::plugin::{Bytes, Guest, Metadata},
};

mod bindings {
    wit_bindgen::generate!({
        path: "../../libs/bindings",
        world: "logcraft:lgc/plugins"
    });
}

// Local modules
mod helpers;
mod schemas;
use schemas::{
    rule::SentinelRule,
    settings::{AzureError, Sentinel},
};

impl Guest for Sentinel {
    /// Retrieve plugin metadata
    fn load() -> Metadata {
        Metadata {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Retrieve plugin settings
    fn settings() -> Result<Bytes, String> {
        let generator = schemars::gen::SchemaSettings::default()
            .with(|s| {
                s.option_add_null_type = false;
            })
            .into_generator();

        match serde_json::to_vec(&generator.into_root_schema_for::<Sentinel>()) {
            Ok(schema) => Ok(schema),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Retrieve plugin detection schema
    fn schema() -> Result<Bytes, String> {
        match serde_json::to_vec(&schemars::schema_for!(SentinelRule)) {
            Ok(schema) => Ok(schema),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Validate detection rule
    fn validate(detection: Bytes) -> Result<(), String> {
        SentinelRule::deserialize(&detection)?
            .validate()
            .map(|_| ())
    }

    /// Create SavedSearch
    fn create(config: Bytes, detection: Bytes) -> Result<(), String> {
        // Parse settings
        let settings = Sentinel::deserialize(&config)?;

        // Convert JSON to SentinelRule
        let rule = SentinelRule::deserialize(&detection)?;

        // Prepare the request
        let request = settings
            .client(waki::Method::Put, &rule.name)?
            .header("Content-Type", "application/json")
            .json(&rule);

        let res = request.send().map_err(|e| e.to_string())?;
        match res.status_code() {
            // 200 (Update) and 201 (Create).
            // Update uses this method but the only change is the response code.
            200 | 201 => Ok(()),
            400 => Err(AzureError::from_slices(
                res.body().map_err(|e| e.to_string())?,
            )),
            code => Err(http::StatusCode::from_u16(code)
                .map(|status| status.to_string())
                .unwrap_or_else(|_| format!("HTTP/{} Invalid status code", code))),
        }
    }

    /// Get SavedSearch
    fn read(config: Bytes, detection: Bytes) -> Result<Option<Bytes>, String> {
        // Parse settings
        let settings = Sentinel::deserialize(&config)?;

        // Convert JSON to SentinelRule
        let rule = SentinelRule::deserialize(&detection)?;

        // Validate the detection rule and retrieve the detection as serde_json::Value.
        let detection_value = rule.validate()?;

        // Prepare the request
        let request = settings.client(waki::Method::Get, &rule.name)?;

        let res = request.send().map_err(|e| e.to_string())?;
        match res.status_code() {
            // Returned string isn't used for now.
            200 => {
                let body: serde_json::Value = res.json().map_err(|e| e.to_string())?;
                // Recursively filter the response using the detection as a template.
                let filtered = helpers::filter_response(&detection_value, body);
                Ok(Some(
                    serde_json::to_vec(&filtered).map_err(|e| e.to_string())?,
                ))
            }
            404 => Ok(None),
            400 => Err(AzureError::from_slices(
                res.body().map_err(|e| e.to_string())?,
            )),
            code => Err(http::StatusCode::from_u16(code)
                .map(|status| status.to_string())
                .unwrap_or_else(|_| format!("HTTP/{} Invalid status code", code))),
        }
    }

    /// Update SavedSearch
    fn update(config: Bytes, detection: Bytes) -> Result<(), String> {
        // Azure Sentinel uses the same method for creating and updating rules.
        Self::create(config, detection)
    }

    /// Delete SavedSearch
    fn delete(config: Bytes, detection: Bytes) -> Result<(), String> {
        // Parse settings
        let settings = Sentinel::deserialize(&config)?;

        // Convert JSON to SentinelRule
        let rule = SentinelRule::deserialize(&detection)?;

        // Prepare the request
        let request = settings.client(waki::Method::Delete, &rule.name)?;

        let res = request.send().map_err(|e| e.to_string())?;
        match res.status_code() {
            // Returned string isn't used for now.
            200 | 404 => Ok(()),
            400 => Err(AzureError::from_slices(
                res.body().map_err(|e| e.to_string())?,
            )),
            code => Err(http::StatusCode::from_u16(code)
                .map(|status| status.to_string())
                .unwrap_or_else(|_| format!("HTTP/{} Invalid status code", code))),
        }
    }

    /// Ping service
    fn ping(config: Bytes) -> Result<bool, String> {
        // Parse settings
        let settings = Sentinel::deserialize(&config)?;
        // Check workspace connection
        settings.check_workspace().map(|_| true)
    }
}

export!(Sentinel with_types_in bindings);
