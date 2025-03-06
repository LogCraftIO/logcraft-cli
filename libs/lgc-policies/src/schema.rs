// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use super::{
    helpers,
    policy::{CheckKind, Policy},
};
use serde_json::{json, Value};

const FIELD_PARAM: &str = "${fieldName}";

impl Policy {
    /// Generates a JSON Schema for a given policy.
    pub fn to_schema(&self) -> Result<Value, &str> {
        // Use default message if no custom message is provided.
        let msg = if let Some(ref m) = self.message {
            m.replace(FIELD_PARAM, &self.field)
        } else {
            self.default_message()
        };

        // Prepare the schema with the custom message.
        let mut schema = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "x-message": msg,
        });

        let parts = helpers::parse_field(&self.field);

        // Enforce string type for Pattern and Constraint checks.
        let enforced_type = match self.check {
            CheckKind::Pattern | CheckKind::Constraint => Some("string"),
            _ => None,
        };

        // Build the schema based on the check kind.
        let leaf_schema = self.build_leaf_schema(enforced_type)?;
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
        Ok(schema)
    }

    /// Builds the leaf schema for a given policy.
    fn build_leaf_schema(&self, enforced_type: Option<&str>) -> Result<Value, &str> {
        let ignore = self.ignorecase.unwrap_or(false);
        let mut leaf_schema = if let Some(t) = enforced_type {
            json!({ "type": t })
        } else {
            json!({})
        };

        match self.check {
            CheckKind::Pattern => {
                if let Some(ref regex) = self.regex {
                    let pattern = if ignore && !regex.starts_with("(?i)") {
                        format!("(?i){}", regex)
                    } else {
                        regex.clone()
                    };
                    leaf_schema["pattern"] = json!(pattern);
                } else {
                    return Err("pattern check requires a regex.");
                }
            }
            CheckKind::Constraint => {
                if let Some(ref cons) = self.validations {
                    match (cons.min_length, cons.max_length) {
                        (Some(min), Some(max)) => {
                            if min > max {
                                return Err("minLength must be less than or equal to maxLength.");
                            } else {
                                leaf_schema["minLength"] = json!(min);
                                leaf_schema["maxLength"] = json!(max);
                            }
                        }
                        (Some(min), None) => {
                            leaf_schema["minLength"] = json!(min);
                        }
                        (None, Some(max)) => {
                            leaf_schema["maxLength"] = json!(max);
                        }
                        _ => {}
                    }
                    if let Some(ref vals) = cons.values {
                        if ignore {
                            let pattern = format!("^(?i:({}))$", vals.join("|"));
                            leaf_schema["pattern"] = json!(pattern);
                        } else {
                            leaf_schema["enum"] = json!(vals);
                        }
                    }
                } else {
                    return Err("constraint check requires validations to be defined.");
                }
            }
            _ => {}
        }
        Ok(leaf_schema)
    }
}
