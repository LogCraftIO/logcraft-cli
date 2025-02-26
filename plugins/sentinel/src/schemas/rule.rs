// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use once_cell::sync::Lazy;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_with_macros::skip_serializing_none;
use std::collections::HashMap;

use super::types;
use crate::bindings::exports::logcraft::lgc::plugin::Bytes;

const RE_ISO8601: &str = r#"^P(?=\d|T\d)(\d+Y)?(\d+M)?(\d+D)?(?:T(\d+H)?(\d+M)?(\d+S)?)?$"#;

static RULE_SCHEMA: Lazy<serde_json::Value> = Lazy::new(|| {
    serde_json::to_value(schemars::schema_for!(SentinelRule)).expect("Failed to generate schema")
});

static SCHEMA_VALIDATOR: Lazy<jsonschema::Validator> = Lazy::new(|| {
    jsonschema::validator_for(&RULE_SCHEMA).expect("Failed to create schema validator")
});

/// Top-level Sentinel rule.
#[skip_serializing_none] // ! Must be set before derive Ser/Deser macros.
#[derive(Default, Serialize, Deserialize, JsonSchema)]
pub struct SentinelRule {
    /// The name of the alert rule.
    pub name: String,
    /// Sentinel alert rule kind.
    pub kind: types::AlertRuleKind,
    /// Etag of the azure resource.
    pub etag: Option<String>,
    /// The alert rules properties.
    pub properties: Properties,
}

impl SentinelRule {
    pub fn validate(&self) -> Result<serde_json::Value, String> {
        // Convert the rule into a JSON value.
        let detection = serde_json::to_value(self).map_err(|e| e.to_string())?;

        // Validate the rule against the json schema.
        SCHEMA_VALIDATOR.validate(&detection).map_err(|e| {
            format!(
                "field: `{}`",
                e.instance_path
                    .to_string()
                    .trim_start_matches('/')
                    .replace('/', ".")
            )
        })?;

        Ok(detection)
    }

    pub fn deserialize(detection: &Bytes) -> Result<Self, String> {
        let mut de = serde_json::Deserializer::from_slice(detection);

        serde_path_to_error::deserialize(&mut de).map_err(|e| {
            let message = e
                .inner()
                .to_string()
                .split_once(" at")
                .map(|(msg, _)| String::from(msg))
                .unwrap_or(e.inner().to_string());

            if e.path().to_string() == "." {
                format!("error: {message}")
            } else {
                format!("field: `{}`, error: {message}", e.path())
            }
        })
    }
}

#[skip_serializing_none] // ! Must be set before derive Ser/Deser macros.
#[derive(Default, Serialize, Deserialize, JsonSchema)]
pub struct Properties {
    // ******* General options *******
    /// Determines whether this alert rule is enabled or disabled.
    pub enabled: bool,

    /// The display name for alerts created by this alert rule.
    #[serde(rename = "displayName")]
    pub display_name: String,

    /// Query that creates alerts for this rule.
    #[serde(rename(deserialize = "query"))]
    pub query: String,

    // ******* Scheduling options *******
    /// The frequency (in ISO 8601 duration format) for this alert rule to run.
    #[serde(rename = "queryFrequency")]
    pub query_frequency: String,

    /// The period (in ISO 8601 duration format) that this alert rule looks at.
    #[serde(rename = "queryPeriod")]
    pub query_period: String,

    /// Severity level of the alert.
    pub severity: types::AlertSeverity,

    /// The suppression (in ISO 8601 duration format) to wait since last time this alert rule been triggered.
    #[serde(rename = "suppressionDuration")]
    #[validate(regex = "RE_ISO8601")]
    suppression_duration: String,

    /// Determines whether the suppression for this alert rule is enabled or disabled.
    #[serde(rename = "suppressionEnabled")]
    suppression_enabled: bool,

    /// The operation against the threshold that triggers alert rule.
    #[serde(rename = "triggerOperator")]
    pub trigger_operator: types::TriggerOperator,

    /// The threshold triggers this alert rule.
    #[serde(rename = "triggerThreshold")]
    pub trigger_threshold: u32,

    /// The alert details override settings
    #[serde(rename = "alertDetailsOverride")]
    pub alert_details_override: Option<types::AlertDetailsOVerride>,

    /// The Name of the alert rule template used to create this rule.
    #[serde(rename = "alertRuleTemplateName")]
    pub alert_rule_template_name: Option<String>,

    /// Dictionary of string key-value pairs of columns to be attached to the alert
    #[serde(rename = "customDetails")]
    pub custom_details: Option<HashMap<String, String>>,

    /// The description of the alert rule.
    pub description: Option<String>,

    /// Array of the entity mappings of the alert rule
    #[serde(rename = "entityMappings")]
    pub entity_mappings: Option<Vec<types::EntityMapping>>,

    /// The event grouping settings.
    #[serde(rename = "eventGroupingSettings")]
    pub event_grouping_settings: Option<types::EventGroupingSettings>,

    /// The settings of the incidents that created from alerts triggered by this analytics rule
    #[serde(rename = "incidentConfiguration")]
    pub incident_configuration: Option<types::IncidentConfiguration>,

    /// The tactics of the alert rule
    pub tactics: Option<Vec<types::AttackTactic>>,

    /// The techniques of the alert rule
    pub techniques: Option<Vec<String>>,

    /// The version of the alert rule template used to create this rule - in format <a.b.c>, where all are numbers, for example 0 <1.0.2>
    #[serde(rename = "templateVersion")]
    pub template_version: Option<String>,
}
