// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The alert rule kind
#[derive(Default, Serialize, Deserialize, JsonSchema)]
pub enum AlertRuleKind {
    #[default]
    Scheduled,
}

#[derive(Default, Serialize, Deserialize, JsonSchema)]
/// Alert severity levels.
pub enum AlertSeverity {
    #[default]
    Informational,
    Low,
    Medium,
    High,
}

#[derive(Default, Serialize, Deserialize, JsonSchema)]
/// Trigger operators.
pub enum TriggerOperator {
    #[default]
    GreaterThan,
    Equal,
    LessThan,
    NotEqual,
}

#[derive(Serialize, Deserialize, JsonSchema)]
/// The alert details override settings
pub struct AlertDetailsOVerride {
    /// The format containing columns name(s) to override the alert description
    #[serde(rename = "alertDescriptionFormat")]
    pub alert_description_format: String,
    /// The format containing columns name(s) to override the alert name
    #[serde(rename = "alertDisplayNameFormat")]
    pub alert_display_name_format: String,
    /// List of additional dynamic properties to override
    #[serde(rename = "alertDynamicProperties")]
    pub alert_dynamic_properties: Vec<AlertPropertyMapping>,
    /// The column name to take the alert severity from
    #[serde(rename = "alertSeverityColumnName")]
    pub alert_severity_column_name: String,
    /// The column name to take the alert tactics from
    #[serde(rename = "alertTacticsColumnName")]
    pub alert_tactics_column_name: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
/// A single alert property mapping to override
pub struct AlertPropertyMapping {
    /// The V3 alert property
    #[serde(rename = "alertProperty")]
    pub alert_properties: AlertProperty,
    /// The column name to use to override this property
    pub value: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
/// The V3 alert property
pub struct AlertProperty {
    /// Alert's link
    #[serde(rename = "AlertLink")]
    pub alert_link: String,
    /// Confidence level property
    #[serde(rename = "ConfidenceLevel")]
    pub confidence_level: String,
    /// Confidence score
    #[serde(rename = "ConfidenceScore")]
    pub confidence_score: String,
    /// Extended links to the alert
    #[serde(rename = "ExtendedLinks")]
    pub extended_links: String,
    /// Product component name alert property
    #[serde(rename = "ProductComponentName")]
    pub product_component_name: String,
    /// Product name alert property
    #[serde(rename = "ProductName")]
    pub product_name: String,
    /// Provider name alert property
    #[serde(rename = "ProviderName")]
    pub provider_name: String,
    /// Remediation steps alert property
    #[serde(rename = "RemediationSteps")]
    pub remediation_steps: String,
    /// Techniques alert property
    #[serde(rename = "Techniques")]
    pub techniques: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
/// Single entity mapping for the alert rule
pub struct EntityMapping {
    /// The V3 type of the mapped entity
    #[serde(rename = "entityType")]
    pub entity_type: EntityMappingType,
    /// Array of field mappings for the given entity mapping
    #[serde(rename = "fieldMappings")]
    pub field_mappings: Vec<FieldMapping>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
/// The V3 type of the mapped entity
pub enum EntityMappingType {
    /// User account entity type
    Account,
    /// Azure resource entity type
    AzureResource,
    /// Cloud app entity type
    CloudApplication,
    /// DNS entity type
    #[serde(rename = "DNS")]
    Dns,
    /// System file entity type
    File,
    /// File-hash entity type
    FileHash,
    /// Host entity type
    Host,
    /// IP address entity type
    #[serde(rename = "IP")]
    Ip,
    /// Mail cluster entity type
    MailCluster,
    /// Mail message entity type
    MailMessage,
    /// Mailbox entity type
    Mailbox,
    /// Malware entity type
    Malware,
    /// Process entity type
    Process,
    /// Registry key entity type
    RegistryKey,
    /// Registry value entity type
    RegistryValue,
    /// Security group entity type
    SecurityGroup,
    /// Submission mail entity type
    SubmissionMail,
    /// URL entity type
    #[serde(rename = "URL")]
    Url,
}

#[derive(Serialize, Deserialize, JsonSchema)]
/// A single field mapping of the mapped entity
pub struct FieldMapping {
    /// The column name to be mapped to the identifier
    #[serde(rename = "columnName")]
    pub column_name: String,
    /// The V3 identifier of the entity
    pub identifier: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
/// Event grouping settings property bag.
pub struct EventGroupingSettings {
    /// The event grouping aggregation kinds
    #[serde(rename = "aggregationKind")]
    pub aggregation_kind: EventGroupingAggregationKind,
}

#[derive(Serialize, Deserialize, JsonSchema)]
/// The event grouping aggregation kinds
pub enum EventGroupingAggregationKind {
    AlertPerResult,
    SingleAlert,
}

#[derive(Serialize, Deserialize, JsonSchema)]
/// Incident Configuration property bag.
pub struct IncidentConfiguration {
    /// Create incidents from alerts triggered by this analytics rule
    #[serde(rename = "createIncident")]
    pub create_incident: bool,
}

#[derive(Serialize, Deserialize, JsonSchema)]
/// Grouping configuration property bag.
pub struct GroupingConfiguration {
    /// Grouping enabled
    pub enabled: bool,
    /// A list of alert details to group by (when matchingMethod is Selected)
    #[serde(rename = "groupByAlertDetails")]
    pub group_by_alert_details: Vec<AlertDetail>,
    /// A list of custom details keys to group by (when matchingMethod is Selected). Only keys defined in the current alert rule may be used.
    #[serde(rename = "groupByCustomDetails")]
    pub group_by_custom_details: Vec<String>,
    /// A list of entity types to group by (when matchingMethod is Selected). Only entities defined in the current alert rule may be used.
    #[serde(rename = "groupByEntities")]
    pub group_by_entities: Vec<EntityMappingType>,
    /// Limit the group to alerts created within the lookback duration (in ISO 8601 duration format)
    #[serde(rename = "lookbackDuration")]
    pub lookback_duration: String,
    /// Grouping matching method. When method is Selected at least one of groupByEntities, groupByAlertDetails, groupByCustomDetails must be provided and not empty.
    #[serde(rename = "matchingMethod")]
    pub matching_method: MatchingMethod,
    /// Re-open closed matching incidents
    #[serde(rename = "reopenClosedIncident")]
    pub reopen_closed_incident: bool,
}

#[derive(Serialize, Deserialize, JsonSchema)]
/// A list of alert details to group by (when matchingMethod is Selected)
pub struct AlertDetail {
    /// Alert display name
    #[serde(rename = "DisplayName")]
    pub display_name: String,
    /// Alert severity
    #[serde(rename = "Severity")]
    pub severity: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
/// Grouping matching method. When method is Selected at least one of groupByEntities, groupByAlertDetails, groupByCustomDetails must be provided and not empty.
pub struct MatchingMethod {
    /// Grouping alerts into a single incident if all the entities match
    #[serde(rename = "AllEntities")]
    pub all_entities: String,
    /// Grouping any alerts triggered by this rule into a single incident
    #[serde(rename = "AnyAlert")]
    pub any_alert: String,
    /// Grouping alerts into a single incident if the selected entities, custom details and alert details match
    #[serde(rename = "Selected")]
    pub selected: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
/// The severity for alerts created by this alert rule.
pub enum AttackTactic {
    Collection,
    CommandAndControl,
    CredentialAccess,
    DefenseEvasion,
    Discovery,
    Execution,
    Exfiltration,
    Impact,
    ImpairProcessControl,
    InhibitResponseFunction,
    InitialAccess,
    LateralMovement,
    Persistence,
    PreAttack,
    PrivilegeEscalation,
    Reconnaissance,
    ResourceDevelopment,
}
