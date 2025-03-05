// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use super::{
    helpers,
    policy::{CheckKind, Policy},
};
use serde_json::{json, Value};

impl Policy {
    /// Generates a JSON Schema for a given policy.
    pub fn to_schema(&self) -> Value {
        let mut schema = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "x-severity": self.severity.to_string()
        });

        let parts = helpers::parse_field(&self.field);
        let leaf_field = parts.last().unwrap_or(&self.field.as_str()).to_string();

        // Enforce string type for Pattern and Constraint checks.
        let enforced_type = match self.check {
            CheckKind::Pattern | CheckKind::Constraint => Some("string"),
            _ => None,
        };

        let leaf_schema = self.build_leaf_schema(&leaf_field, enforced_type);

        match parts.as_slice() {
            [] => schema["properties"] = json!({}),
            [field] => match self.check {
                CheckKind::Absence => schema["not"] = json!({ "required": [field] }),
                _ => {
                    schema["properties"] = json!({ *field: leaf_schema });
                    schema["required"] = json!([*field]);
                }
            },
            _ => {
                let nested_schema = helpers::build_nested(&parts, leaf_schema);
                match self.check {
                    CheckKind::Absence => schema["not"] = nested_schema,
                    _ => {
                        schema["properties"] = nested_schema["properties"].clone();
                        schema["required"] = nested_schema["required"].clone();
                    }
                }
            }
        }
        schema
    }

    /// Builds the leaf schema for a given policy.
    fn build_leaf_schema(&self, leaf_field: &str, enforced_type: Option<&str>) -> Value {
        let ignore = self.ignore_case.unwrap_or(false);
        let mut leaf_schema = if let Some(t) = enforced_type {
            json!({ "type": t })
        } else {
            json!({})
        };

        if ignore {
            leaf_schema["x-ignorecase"] = json!(true);
        }
        // Use default message if no custom message is provided.
        let msg = if let Some(ref m) = self.message {
            m.replace("${fieldName}", leaf_field)
        } else {
            self.default_message()
        };
        leaf_schema["x-message"] = json!(msg);

        match self.check {
            CheckKind::Pattern => {
                if let Some(ref regex) = self.regex {
                    let pattern = if ignore && !regex.starts_with("(?i)") {
                        format!("(?i){}", regex)
                    } else {
                        regex.clone()
                    };
                    leaf_schema["pattern"] = json!(pattern);
                }
            }
            CheckKind::Constraint => {
                if let Some(ref cons) = self.constraints {
                    if let Some(min) = cons.min_length {
                        leaf_schema["minLength"] = json!(min);
                    }
                    if let Some(max) = cons.max_length {
                        leaf_schema["maxLength"] = json!(max);
                    }
                    if let Some(ref vals) = cons.values {
                        if ignore {
                            let pattern = format!("^(?i:({}))$", vals.join("|"));
                            leaf_schema["pattern"] = json!(pattern);
                        } else {
                            leaf_schema["enum"] = json!(vals);
                        }
                    }
                }
            }
            _ => {}
        }
        leaf_schema
    }
}
