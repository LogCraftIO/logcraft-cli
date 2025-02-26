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
mod schemas;
use schemas::{
    rule::{ErrorResponse, SearchResponse, SplunkRule},
    settings::Splunk,
};

impl Guest for Splunk {
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

        match serde_json::to_vec(&generator.into_root_schema_for::<Splunk>()) {
            Ok(schema) => Ok(schema),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Retrieve plugin detection schema
    fn schema() -> Result<Bytes, String> {
        match serde_json::to_vec(&schemars::schema_for!(SplunkRule)) {
            Ok(schema) => Ok(schema),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Validate detection rule
    fn validate(detection: Bytes) -> Result<(), String> {
        SplunkRule::deserialize(&detection)?.validate().map(|_| ())
    }

    /// Create SavedSearch
    fn create(config: Bytes, detection: Bytes) -> Result<(), String> {
        // Parse service settings and check target application.
        let settings = Splunk::deserialize(&config)?;

        // Convert the JSON value into a typed SplunkRule.
        let rule = SplunkRule::deserialize(&detection)?;

        // Prepare the request.
        let request = settings
            .client(waki::Method::Post, "")
            .map_err(|e| e.to_string())?
            .query(&[("output_mode", "json")])
            .form(
                // Convert the detection rule into a flat map for the request.
                &rule.into_flat_map(true)?,
            );

        // Send the request.
        let res = request.send().map_err(|e| e.to_string())?;
        match res.status_code() {
            201 => Ok(()),
            400 => {
                // Retrieve and parse the response body.
                let body = res.body().map_err(|e| e.to_string())?;

                // Extract the error message from the response.
                if let Ok(resp) = serde_json::from_slice::<ErrorResponse>(&body) {
                    Err(resp.messages[0].text.to_string())
                } else if let Ok(body) = String::from_utf8(body) {
                    Err(format!("RAW ERROR: {body}"))
                } else {
                    Err("bad request".to_string())
                }
            }
            code => Err(http::StatusCode::from_u16(code)
                .map(|status| status.to_string())
                .unwrap_or_else(|_| format!("HTTP/{} Invalid status code", code))),
        }
    }

    /// Get SavedSearch
    fn read(config: Bytes, detection: Bytes) -> Result<Option<Bytes>, String> {
        // Parse service settings and check target application.
        let settings = Splunk::deserialize(&config)?;
        settings.check_app()?;

        // Convert the JSON value into a typed SplunkRule.
        let rule = SplunkRule::deserialize(&detection)?;

        // Validate the detection rule and retrieve the detection as serde_json::Value.
        let detection_value = &rule.validate()?;

        // Prepare the request.
        let request = settings
            .client(waki::Method::Get, &rule.title)
            .map_err(|e| e.to_string())?
            .query(&[("output_mode", "json")]);

        // Send the request.
        let res = request.send().map_err(|e| e.to_string())?;
        match res.status_code() {
            200 => {
                // Retrieve and parse the response body.
                let body = res.body().map_err(|e| e.to_string())?;

                // Extract the first detection rule from the response.
                let entry = serde_json::from_slice::<SearchResponse>(&body)
                    .map_err(|err| format!("unable to parse response: {}", err))?
                    .entry
                    .into_iter()
                    .next()
                    .ok_or_else(|| "no detection rule found in response".to_string())?;

                // Extract the parameters from the detection JSON.
                let params_object = detection_value
                    .get("parameters")
                    .and_then(|v| v.as_object())
                    .ok_or_else(|| "missing parameters field".to_string())?;

                // Filter out unset keys from the detection response.
                let filtered: std::collections::HashMap<String, serde_json::Value> = params_object
                    .iter()
                    .filter_map(|(k, orig)| {
                        // Skip if the original value is null.
                        if orig.is_null() {
                            return None;
                        }
                        // Get and clone the corresponding value from entry.content.
                        let v = entry.content.get(k)?.clone();
                        // ? Splunk returns "boolean" numbers as string (e.g. "1" or "0").
                        // ? We need to convert them back to numbers for detection comparison later.
                        let value = match v {
                            serde_json::Value::String(s)
                                if orig.is_number() && (s == "0" || s == "1") =>
                            {
                                s.parse::<u32>()
                                    .map(|n| serde_json::Value::Number(n.into()))
                                    .unwrap_or_else(|_| serde_json::Value::String(s))
                            }
                            other => other,
                        };
                        Some((k.clone(), value))
                    })
                    .collect();

                // Retrieve the "search" field from the response.
                let search = entry
                    .content
                    .get("search")
                    .cloned()
                    .ok_or_else(|| "no search field found in response".to_string())?;

                // Build the final JSON response.
                let result_json = serde_json::json!({
                    "title": rule.title,
                    "search": search,
                    "parameters": filtered
                });
                serde_json::to_vec_pretty(&result_json)
                    .map(Some)
                    .map_err(|e| e.to_string())
            }
            404 => Ok(None),
            code => Err(http::StatusCode::from_u16(code)
                .map(|status| status.to_string())
                .unwrap_or_else(|_| format!("HTTP/{} Invalid status code", code))),
        }
    }

    /// Update SavedSearch
    fn update(config: Bytes, detection: Bytes) -> Result<(), String> {
        // Parse service settings and check target application.
        let settings = Splunk::deserialize(&config)?;

        // Convert the JSON value into a typed SplunkRule.
        let rule = SplunkRule::deserialize(&detection)?;

        // Prepare the request.
        let request = settings
            .client(waki::Method::Post, &rule.title)
            .map_err(|e| e.to_string())?
            .query(&[("output_mode", "json")])
            .form(
                // Convert the detection rule into a flat map for the request.
                &rule.into_flat_map(false)?,
            );

        // Send the request.
        let res = request.send().map_err(|e| e.to_string())?;
        match res.status_code() {
            200 => Ok(()),
            400 => {
                // Retrieve and parse the response body.
                let body = res.body().map_err(|e| e.to_string())?;

                // Extract the error message from the response.
                if let Ok(resp) = serde_json::from_slice::<ErrorResponse>(&body) {
                    Err(resp.messages[0].text.to_string())
                } else if let Ok(body) = String::from_utf8(body) {
                    Err(format!("RAW ERROR: {body}"))
                } else {
                    Err("bad request".to_string())
                }
            }
            code => Err(http::StatusCode::from_u16(code)
                .map(|status| status.to_string())
                .unwrap_or_else(|_| format!("HTTP/{} Invalid status code", code))),
        }
    }

    /// Delete SavedSearch
    fn delete(config: Bytes, detection: Bytes) -> Result<(), String> {
        // Parse service settings and check target application.
        let settings = Splunk::deserialize(&config)?;

        // Convert the JSON value into a typed SplunkRule.
        let rule = SplunkRule::deserialize(&detection)?;

        // Prepare the request.
        let request = settings
            .client(waki::Method::Delete, &rule.title)
            .map_err(|e| e.to_string())?
            .query(&[("output_mode", "json")]);

        // Send the request.
        let res = request.send().map_err(|e| e.to_string())?;
        match res.status_code() {
            200 | 404 => Ok(()),
            400 => {
                // Retrieve and parse the response body.
                let body = res.body().map_err(|e| e.to_string())?;

                // Extract the error message from the response.
                if let Ok(resp) = serde_json::from_slice::<ErrorResponse>(&body) {
                    Err(resp.messages[0].text.to_string())
                } else if let Ok(body) = String::from_utf8(body) {
                    Err(format!("RAW ERROR: {body}"))
                } else {
                    Err("bad request".to_string())
                }
            }
            code => Err(http::StatusCode::from_u16(code)
                .map(|status| status.to_string())
                .unwrap_or_else(|_| format!("HTTP/{} Invalid status code", code))),
        }
    }

    /// Ping service
    fn ping(config: Bytes) -> Result<bool, String> {
        // Parse service settings and check target application.
        let settings = Splunk::deserialize(&config)?;
        settings.check_app().map(|_| true)
    }
}

export!(Splunk with_types_in bindings);
