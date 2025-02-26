// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use serde::{de, Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

const RE_CRON: &str = r#"^(\*|[0-5]?[0-9]|\*\/[0-9]+)\s+(\*|1?[0-9]|2[0-3]|\*\/[0-9]+)\s+(\*|[1-2]?[0-9]|3[0-1]|\*\/[0-9]+)\s+(\*|[0-9]|1[0-2]|\*\/[0-9]+|jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)\s+(\*\/[0-9]+|\*|[0-7]|sun|mon|tue|wed|thu|fri|sat)\s*(\*\/[0-9]+|\*|[0-9]+)?"#;

// Custom deserializer to handle boolean values that could be numbers
fn deserialize_opt_boolean<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    // First, try to deserialize an Option<String> (the field could be missing or null).
    let opt_str = Option::<String>::deserialize(deserializer)?;
    match opt_str {
        None => Ok(None), // Key wasn't present
        Some(s) => match s.trim() {
            "1" | "true" => Ok(Some(true)),
            "0" | "false" => Ok(Some(false)),
            other => Err(de::Error::custom(format!("Invalid bool '{}'", other))),
        },
    }
}

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
pub struct SampleRule {
    pub title: String,
    pub search: String,
    pub parameters: Parameters,
}

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
pub struct Parameters {
    #[serde(flatten)]
    pub unknown_fields: HashMap<String, serde_json::Value>, // Capture unknown fields here

    #[serde(default, deserialize_with = "deserialize_opt_boolean")]
    pub disabled: Option<bool>,

    #[serde(default)]
    #[schemars(regex = "RE_CRON")]
    pub cron_schedule: Option<String>,

    #[serde(default, deserialize_with = "deserialize_opt_boolean")]
    pub is_visible: Option<bool>,

    #[serde(default)]
    pub description: Option<String>,
}
