// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use serde_json::{json, Value};

/// Parses the target field into a list of parts for path composition.
pub(crate) fn parse_field(field: &str) -> Vec<&str> {
    if field.starts_with('/') {
        field.trim_start_matches('/').split('/').collect()
    } else {
        field.split('.').collect()
    }
}

/// Builds a nested JSON Schema.
pub(crate) fn build_nested(parts: &[&str], leaf: Value) -> Value {
    parts.iter().rev().fold(leaf, |acc, &part| {
        json!({
            "type": "object",
            "properties": { part: acc },
            "required": [part]
        })
    })
}
