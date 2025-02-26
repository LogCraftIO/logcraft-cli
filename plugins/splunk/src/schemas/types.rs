// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The user context under which the saved search runs.
#[derive(Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DispatchAs {
    #[default]
    Owner,
    User,
}

/// Saved search scheduling priority.
#[derive(Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SchedulePriority {
    #[default]
    Default,
    Higher,
    Highest,
}

/// Saved search schedule window.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleWindow {
    Auto,
    // A fixed schedule window specified in minutes.
    Minutes(u32),
}

impl Default for ScheduleWindow {
    fn default() -> Self {
        ScheduleWindow::Minutes(0)
    }
}

/// Specifies whether to use parallel reduce processing.
#[derive(Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleAs {
    #[default]
    Auto,
    Classic,
    Prjob,
}

/// Alerting count type.
#[derive(Serialize, Deserialize, JsonSchema)]
pub enum CountType {
    #[serde(rename = "number of events")]
    NumberOfEvents,

    #[serde(rename = "number of hosts")]
    NumberOfHosts,

    #[serde(rename = "number of sources")]
    NumberOfSources,

    #[serde(rename = "custom")]
    Custom,

    #[serde(rename = "always")]
    Always,
}

/// Alerting relation.
#[derive(Serialize, Deserialize, JsonSchema)]
pub enum Relation {
    #[serde(rename = "greater than")]
    GreaterThan,

    #[serde(rename = "less than")]
    LessThan,

    #[serde(rename = "equal to")]
    EqualTo,

    #[serde(rename = "not equal to")]
    NotEqualTo,

    #[serde(rename = "drops by")]
    DropsBy,

    #[serde(rename = "rises by")]
    RisesBy,
}

/// Summary Index types
#[derive(Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SummaryIndex {
    #[default]
    Event,
    Metric,
}

/// Durable track time types
#[derive(Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DurableTrackTime {
    #[default]
    None,
    _Time,
    _IndexTime,
}

/// Backfill types
#[derive(Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Backfill {
    #[default]
    Auto,
    TimeInterval,
    TimeWhole,
}

/// Alert track types
#[derive(Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AlertTrack {
    #[default]
    Auto,
    True,
    False,
}
