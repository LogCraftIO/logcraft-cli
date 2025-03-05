// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use serde::Deserialize;

/// Policy defining a rule for a given field.
#[derive(Debug, Deserialize)]
pub struct Policy {
    /// Field in JSON Pointer style (e.g. "/parameters/disabled").
    pub field: String,
    /// Type of check.
    pub check: CheckKind,
    /// Severity of the policy: warning or error.
    pub severity: Severity,
    /// Custom error message. May contain the placeholder `${fieldName}`.
    pub message: Option<String>,
    /// Whether matching is case-insensitive (default is false).
    pub ignore_case: Option<bool>,
    /// Pattern checks.
    /// jsonschema uses ECMA 262 regex.
    /// [information](https://json-schema.org/understanding-json-schema/reference/regular_expressions)
    pub regex: Option<String>,
    /// For constraint checks: additional parameters.
    pub constraints: Option<Constraint>,
}

/// Type of check to perform.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckKind {
    Existence,
    Absence,
    Pattern,
    Constraint,
}

/// Constraint parameters for the "constraint" check.
#[derive(Debug, Deserialize)]
pub struct Constraint {
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    /// Optional list of allowed values.
    pub values: Option<Vec<String>>,
}

/// Severity output level.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Error,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
        }
    }
}

/// Default error messages for policies.
impl Policy {
    /// Returns the default error message for this policy, with `${fieldName}` replaced.
    pub fn default_message(&self) -> String {
        match (&self.check, &self.severity) {
            (CheckKind::Existence, Severity::Warning) => {
                format!("field '{}' should be present", self.field)
            }
            (CheckKind::Existence, Severity::Error) => {
                format!("field '{}' must be present", self.field)
            }
            (CheckKind::Absence, Severity::Warning) => {
                format!("field '{}' shouldn't be present", self.field)
            }
            (CheckKind::Absence, Severity::Error) => {
                format!("field '{}' must not be present", self.field)
            }
            (CheckKind::Constraint, Severity::Warning) => {
                format!("field '{}' doesn't respect constraint", self.field)
            }
            (CheckKind::Constraint, Severity::Error) => {
                format!("field '{}' doesn't respect constraint", self.field)
            }
            (CheckKind::Pattern, Severity::Warning) => {
                format!("field '{}' doesn't match pattern", self.field)
            }
            (CheckKind::Pattern, Severity::Error) => {
                format!("field '{}' doesn't match pattern", self.field)
            }
        }
    }
}
