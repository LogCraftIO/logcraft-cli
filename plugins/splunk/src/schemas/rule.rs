// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use once_cell::sync::Lazy;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with_macros::skip_serializing_none;
use std::collections::HashMap;

use super::types;
use crate::bindings::exports::logcraft::lgc::plugin::Bytes;

const RE_CRON: &str = r#"(@(annually|yearly|monthly|weekly|daily|hourly|reboot))|(@every (\d+(ns|us|µs|ms|s|m|h))+)|((((\d+,)+\d+|(\d+(\/|-)\d+)|\d+|\*) ?){5,7})"#;
const RE_SKEW: &str =
    r#"^(?:0|[1-9]\d*(?:%|m|min|minute|mins|minutes|h|hr|hour|hrs|hours|d|day|days))$"#;
const RE_TTL: &str = r#"^[0-9]+p?$"#;

static RULE_SCHEMA: Lazy<serde_json::Value> = Lazy::new(|| {
    serde_json::to_value(schemars::schema_for!(SplunkRule)).expect("Failed to generate schema")
});

static SCHEMA_VALIDATOR: Lazy<jsonschema::Validator> = Lazy::new(|| {
    jsonschema::validator_for(&RULE_SCHEMA).expect("Failed to create schema validator")
});

/// Splunk rule response
#[derive(Deserialize)]
pub struct SearchResponse {
    /// Splunk search response entries.
    pub entry: Vec<Entry>,
}

/// Splunk rule entry
#[derive(Deserialize)]
pub struct Entry {
    pub content: HashMap<String, serde_json::Value>,
}

/// Splunk error response
#[derive(Deserialize)]
pub struct ErrorResponse {
    /// Splunk error message.
    pub messages: Vec<Message>,
}

/// Splunk error message
#[derive(Deserialize)]
pub struct Message {
    /// Splunk error message text.
    pub text: String,
}

/// Top-level Splunk rule.
#[derive(Default, Serialize, Deserialize, JsonSchema)]
pub struct SplunkRule {
    /// Detection rule title.
    pub title: String,
    // ! SavedSearch validation is not implemented yet.
    /// Splunk Saved Search.
    pub search: String,
    /// Splunk Saved Search parameters.
    pub parameters: Parameters,
}

impl SplunkRule {
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
            format!(
                "field: `{}`, error: {}",
                e.path(),
                e.inner()
                    .to_string()
                    .split_once(" at")
                    .map(|(msg, _)| msg)
                    .unwrap_or(&e.inner().to_string())
            )
        })
    }

    /// Convert this struct into a list of `(key, value)` pairs
    pub fn into_flat_map(self, with_name: bool) -> Result<Vec<(String, String)>, String> {
        let mut pairs = Vec::new();
        // Insert the `title` and `search` fields.
        if with_name {
            // Only for creating a new saved search.
            pairs.push(("name".to_owned(), self.title));
        }
        pairs.push(("search".to_owned(), self.search));

        // Serialize `parameters` and flatten its fields into (key, value) pairs.
        let params_value = serde_json::to_value(&self.parameters).map_err(|e| e.to_string())?;

        // If `parameters` is an Object, extract each field's value as a string.
        if let Value::Object(params_map) = params_value {
            for (key, value) in params_map {
                let value_str = match value {
                    Value::String(s) => s,
                    Value::Array(arr) => arr
                        .into_iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<String>>()
                        .join(","),
                    val => val.to_string(),
                };
                pairs.push((key, value_str));
            }
        }

        Ok(pairs)
    }
}

#[skip_serializing_none] // ! Must be set before derive Ser/Deser macros.
#[derive(Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Parameters {
    /// SavedSearch parameters unknown fields sent to Splunk.
    #[serde(flatten)]
    pub unknown_fields: HashMap<String, serde_json::Value>,

    // Basic settings
    /// Indicates if the saved search is enabled. Defaults to false.
    pub disabled: Option<bool>,
    /// Sets the user context under which the saved search runs. Defaults to "owner".
    #[serde(rename = "dispatchAs")]
    pub dispatch_as: Option<types::DispatchAs>,

    // ******* Scheduling options *******
    /// Toggles scheduled execution of the saved search. Defaults to 0.
    #[serde(rename = "enableSched")]
    #[validate(range(min = 0, max = 1))]
    pub enable_sched: Option<u8>,
    /// The cron schedule that is used to run this search. No default.
    #[validate(regex = "RE_CRON")]
    pub cron_schedule: Option<String>,
    /// Lets a scheduled search use a slightly adjusted time window to account for indexing delays. Defaults to "0".
    #[validate(regex = "RE_SKEW")]
    pub allow_skew: Option<String>,
    /// The maximum number of concurrent instances of this search that the scheduler is allowed to run. Defaults to 1.
    pub max_concurrent: Option<u32>,
    /// Defines the interval at which a real-time search refreshes its results. Defaults to true.
    pub realtime_schedule: Option<bool>,
    /// Assigns a priority level to a scheduled search, influencing its execution order relative to others.
    pub schedule_priority: Option<types::SchedulePriority>,
    /// Sets the time range that the scheduled search covers relative to its execution time, ensuring it includes any delayed events. Default to 0.
    pub schedule_window: Option<types::ScheduleWindow>,
    /// Specifies whether a scheduled search should use parallel reduce search processing each time it runs.
    pub schedule_as: Option<types::ScheduleAs>,

    // ******* Workload management options *******
    /// Specifies the name of the workload pool to be used by this search.
    pub workload_pool: Option<String>,

    // ******* Notification options *******
    /// Set the type of count for alerting.
    pub counttype: Option<types::CountType>,
    /// Set the relation for alerting.
    pub relation: Option<types::Relation>,
    /// Specifies the 'counttype' and 'relation' values used to trigger an alert.
    pub quantity: Option<i32>,
    // ! SavedSearch validation is not implemented yet.
    /// Contains a conditional search evaluated against the saved search results that triggers an alert if any results are returned.
    pub alert_condition: Option<String>,

    // ******* Generic action settings *******
    // Generic action settings can be defined per action. Here we use a map
    // to capture any settings with keys like "action.<action_name>".
    // pub action: Option<HashMap<String, serde_json::Value>>,

    // ******* Settings for email action *******
    /// Specifies whether the email action is enabled for this search. Defaults to false.
    #[serde(rename = "action.email")]
    pub action_email: Option<bool>,
    /// Set a comma-delimited list of recipient email addresses.
    #[serde(rename = "action.email.to")]
    pub action_email_to: Option<String>,
    /// Set an email address to use as the sender's address. Defaults to "splunk@localhost".
    #[validate(email)]
    #[serde(rename = "action.email.from")]
    pub action_email_from: Option<String>,
    /// Set the subject of the email delivered to recipients.
    #[serde(rename = "action.email.subject")]
    pub action_email_subject: Option<String>,
    /// Set the address of the MTA server to be used to send the emails. Defaults to "LOCALHOST".
    #[serde(rename = "action.email.mailserver")]
    pub action_email_mailserver: Option<String>,
    /// Set the maximum number of results to email. Defaults to 10000.
    #[serde(rename = "action.email.maxresults")]
    pub action_email_maxresults: Option<i32>,
    /// Specify whether to include a link to search results in the alert notification email. Defaults to 0 (false).
    #[serde(rename = "action.email.include.results_link")]
    #[validate(range(min = 0, max = 1))]
    pub action_email_include_results_link: Option<u8>,
    /// Specify whether to include the query whose results triggered the email. Defaults to 0 (false).
    #[serde(rename = "action.email.include.search")]
    #[validate(range(min = 0, max = 1))]
    pub action_email_include_search: Option<u8>,
    /// pecify whether to include the alert trigger condition. Defaults to 0 (false).
    #[serde(rename = "action.email.include.trigger")]
    #[validate(range(min = 0, max = 1))]
    pub action_email_include_trigger: Option<u8>,
    /// Specify whether to include the alert trigger time. Defaults to 0 (false).
    #[serde(rename = "action.email.include.trigger_time")]
    #[validate(range(min = 0, max = 1))]
    pub action_email_include_trigger_time: Option<u8>,
    /// Specify whether to include saved search title and a link for editing the saved search. Defaults to 1 (true).
    #[serde(rename = "action.email.include.view_link")]
    #[validate(range(min = 0, max = 1))]
    pub action_email_include_view_link: Option<u8>,
    /// Specify whether to include search results or PNG exports in the body of the alert notification email. Defaults to 0 (false).
    #[serde(rename = "action.email.inline")]
    #[validate(range(min = 0, max = 1))]
    pub action_email_inline: Option<u8>,
    /// Specify whether to send results as a CSV file. Defaults to 0 (false).
    #[serde(rename = "action.email.sendcsv")]
    #[validate(range(min = 0, max = 1))]
    pub action_email_sendcsv: Option<u8>,
    /// Specify whether to send results as an attachment. Defaults set by the 'allow_empty_attachment' setting in 'alert_actions.conf'
    #[serde(rename = "action.email.allow_empty_attachment")]
    pub action_email_allow_empty_attachment: Option<bool>,
    /// Specify whether to send results as a PDF file. Defaults to 0 (false).
    #[serde(rename = "action.email.sendpdf")]
    #[validate(range(min = 0, max = 1))]
    pub action_email_sendpdf: Option<u8>,
    /// Specify whether to send Dashboard Studio results as a PNG file. Defaults to 0 (false).
    #[serde(rename = "action.email.sendpng")]
    #[validate(range(min = 0, max = 1))]
    pub action_email_sendpng: Option<u8>,
    /// Specify whether to include search results in the alert notification email. Defaults to 0 (false).
    #[serde(rename = "action.email.sendresults")]
    #[validate(range(min = 0, max = 1))]
    pub action_email_sendresults: Option<u8>,

    // ******* Settings for script action *******
    /// Specifies whether the script action is enabled for this search. Defaults to false.
    #[serde(rename = "action.script")]
    pub action_script: Option<bool>,
    /// The filename, with no path, of the shell script to run.
    #[serde(rename = "action.script.filename")]
    pub action_script_filename: Option<String>,

    // ******* Settings for lookup action *******
    /// Specifies whether the lookup action is enabled for this search. Defaults to false.
    #[serde(rename = "action.lookup")]
    pub action_lookup: Option<bool>,
    /// Provide the name of the CSV lookup file to write search results to. Do not provide a file path.
    #[serde(rename = "action.lookup.filename")]
    pub action_lookup_filename: Option<String>,
    /// Specifies whether to append results to the lookup file defined for the 'action.lookup.filename' setting.
    #[serde(rename = "action.lookup.append")]
    pub action_lookup_append: Option<bool>,

    // ******* Settings for summary index action *******
    /// Specifies whether the summary index action is enabled for this search. Defaults to false.
    #[serde(rename = "action.summary_index")]
    pub action_summary_index: Option<bool>,
    /// Specifies the name of the summary index where the results of the scheduled search are saved. Defaults to "summary".
    #[serde(rename = "action.summary_index._name")]
    pub action_summary_index_name: Option<String>,
    /// Specifies the data type of the summary index where the Splunk software saves the results of the scheduled search. Defaults to "event".
    #[serde(rename = "action.summary_index._type")]
    pub action_summary_index_type: Option<types::SummaryIndex>,
    /// Identify one or more fields with numeric values that the Splunk software should convert into dimensions during the summary indexing process.
    #[serde(rename = "action.summary_index._metric_dims")]
    pub action_summary_index_metric_dims: Option<String>,
    /// Specify whether to run the summary indexing action as part of the scheduled search. Defaults to 1 (true).
    #[serde(rename = "action.summary_index.inline")]
    pub action_summary_index_inline: Option<bool>,
    // For additional field/value pairs, we flatten them into a map.
    /// Specifies a field/value pair to add to every event that gets summary indexed by this search.
    #[serde(flatten)]
    pub action_summary_index_fields: Option<HashMap<String, String>>,
    /// By default 'realtime_schedule' is false for a report configured for summary indexing. Set this attribute to 'true' or '1' to override the default behavior. Defaults to 0 (false).
    #[serde(rename = "action.summary_index.force_realtime_schedule")]
    pub action_summary_index_force_realtime_schedule: Option<bool>,

    // ******* Settings for lookup table population parameters *******
    /// Specifies whether the lookup population action is enabled for this search. Defaults to false.
    #[serde(rename = "action.populate_lookup")]
    pub action_populate_lookup: Option<bool>,
    /// A lookup name from transforms.conf. The lookup name cannot be associated with KV store.
    #[serde(rename = "action.populate_lookup.dest")]
    pub action_populate_lookup_dest: Option<String>,

    // ******* Run options *******
    /// pecifies whether this search runs when the Splunk platform starts or any edit that changes search related arguments happen. This includes search and dispatch.* arguments. Defaults to false.
    pub run_on_startup: Option<bool>,
    /// Runs this search exactly the specified number of times. The search is not run again until the Splunk platform is restarted. Defaults to 0 (infinite).
    pub run_n_times: Option<u32>,

    // ******* dispatch search options *******
    /// Indicates the time to live (ttl), in seconds, for the search job artifacts  produced by the scheduled search, if no actions are triggered. Defaults to 2p.
    #[serde(rename = "dispatch.ttl")]
    #[validate(regex = "RE_TTL")]
    pub dispatch_ttl: Option<String>,
    /// The maximum number of timeline buckets. Defaults to 0.
    #[serde(rename = "dispatch.buckets")]
    pub dispatch_buckets: Option<i32>,
    /// The maximum number of results before finalizing the search. Defaults to 500000.
    #[serde(rename = "dispatch.max_count")]
    pub dispatch_max_count: Option<i32>,
    /// The maximum amount of time, in seconds, before finalizing the search. Defaults to 0.
    #[serde(rename = "dispatch.max_time")]
    pub dispatch_max_time: Option<i32>,
    /// Enables or disables lookups for this search. Defaults to 1 (enable).
    #[serde(rename = "dispatch.lookups")]
    #[validate(range(min = 0, max = 1))]
    pub dispatch_lookups: Option<u8>,
    /// Specifies the earliest time for this search. Can be a relative or absolute time.
    #[serde(rename = "dispatch.earliest_time")]
    pub dispatch_earliest_time: Option<String>,
    /// Specifies the latest time for this saved search. Can be a relative or absolute time.
    #[serde(rename = "dispatch.latest_time")]
    pub dispatch_latest_time: Option<String>,
    /// Specifies the earliest index time for this search. Can be a relative or absolute time.
    #[serde(rename = "dispatch.index_earliest")]
    pub dispatch_index_earliest: Option<String>,
    /// Specifies the latest index time for this saved search. Can be a relative or absolute time.
    #[serde(rename = "dispatch.index_latest")]
    pub dispatch_index_latest: Option<String>,
    /// Defines the time format that is used to specify the earliest and latest time. Defaults to "%FT%T.%Q%:z".
    #[serde(rename = "dispatch.time_format")]
    pub dispatch_time_format: Option<String>,
    /// Specifies whether a new search process is started when this saved search is run. Defaults to 1 (true).
    #[serde(rename = "dispatch.spawn_process")]
    #[validate(range(min = 0, max = 1))]
    pub dispatch_spawn_process: Option<u8>,
    /// Specifies the amount of inactive time, in seconds, after which the job is automatically canceled. Defaults to 0 (never).
    #[serde(rename = "dispatch.auto_cancel")]
    pub dispatch_auto_cancel: Option<i32>,
    /// Specifies the amount of inactive time, in seconds, after which the search job is automatically paused. Defaults to 0 (never).
    #[serde(rename = "dispatch.auto_pause")]
    pub dispatch_auto_pause: Option<i32>,
    /// Specifies the frequency, in number of intermediary results chunks, that the MapReduce reduce phase should run on the accumulated map values. Defaults to 10.
    #[serde(rename = "dispatch.reduce_freq")]
    pub dispatch_reduce_freq: Option<i32>,
    /*
       Specifies whether the search job can proceed to provide partial results if a search
       peer fails. When set to false, the search job fails if a search peer providing
       results for the search job fails.
       Defaults to true.
    */
    #[serde(rename = "dispatch.allow_partial_results")]
    pub dispatch_allow_partial_results: Option<bool>,
    /// Specifies whether to do real-time window backfilling for scheduled real-time searches. Defaults to false.
    #[serde(rename = "dispatch.rt_backfill")]
    pub dispatch_rt_backfill: Option<bool>,
    /// Specifies whether to use 'indexed-realtime' mode when doing real-time searches. Defaults to 'indexed_realtime_use_by_default' in the limits.conf file.
    #[serde(rename = "dispatch.indexedRealtime")]
    pub dispatch_indexed_realtime: Option<bool>,
    /// Controls the number of seconds to wait for disk flushes to finish. Defaults to 'indexed_realtime_disk_sync_delay' in the limits.conf file.
    #[serde(rename = "dispatch.indexedRealtimeOffset")]
    pub dispatch_indexed_realtime_offset: Option<i32>,
    /// Minimum seconds to wait between component index searches. Defaults to 'indexed_realtime_default_span' in the limits.conf file.
    #[serde(rename = "dispatch.indexedRealtimeMinSpan")]
    pub dispatch_indexed_realtime_min_span: Option<i32>,
    /// The max seconds allowed to search data which falls behind realtime. Defaults to 'indexed_realtime_maximum_span' in the limits.conf file.
    #[serde(rename = "dispatch.rt_maximum_span")]
    pub dispatch_rt_maximum_span: Option<i32>,
    /// The integer value used to calculate the sample ratio (formula is 1 / <integer>). Defaults to 1.
    #[serde(rename = "dispatch.sample_ratio")]
    pub dispatch_sample_ratio: Option<i32>,
    /// Specifies whether the search job will be re-run in case of failure caused by search requests throttling on remote peers. Defaults to false.
    #[serde(rename = "dispatch.rate_limit_retry")]
    pub dispatch_rate_limit_retry: Option<bool>,
    /// Specifies whether to restart a real-time search managed by the scheduler when a search peer becomes available for this saved search. Defaults to 1 (true ).
    #[validate(range(min = 0, max = 1))]
    pub restart_on_searchpeer_add: Option<u8>,

    // ******* durable search options *******
    /// Indicates that a scheduled search is durable and specifies how the search tracks events.
    #[serde(rename = "durable.track_time_type")]
    pub durable_track_time_type: Option<types::DurableTrackTime>,
    /// Specifies the search time delay, in seconds, that a durable search uses to catch events that are ingested or indexed late. Defaults to 0.
    #[serde(rename = "durable.lag_time")]
    pub durable_lag_time: Option<u32>,
    /// Specifies how the Splunk software backfills the lost search results of failed scheduled search jobs.
    #[serde(rename = "durable.backfill_type")]
    pub durable_backfill_type: Option<types::Backfill>,
    /*
        pecifies the maximum number of cron intervals (previous scheduled search jobs) that the Splunk software can attempt to backfill for this search,
        when those jobs have incomplete events.
        Defaults to 0 (unlimited).
    */
    #[serde(rename = "durable.max_backfill_intervals")]
    pub durable_max_backfill_intervals: Option<u32>,

    // ******* auto summarization options *******
    /// Specifies if the scheduler should ensure that the data for this search is automatically summarized. Defaults to false.
    pub auto_summarize: Option<bool>,
    /// A search template to use to construct the auto summarization for this search.
    #[serde(rename = "auto_summarize.command")]
    pub auto_summarize_command: Option<String>,
    /*
    Comma-delimited list of time ranges that each summarized chunk should span.
      This comprises the list of available granularity levels for which summaries
      would be available. For example, a timechart over the last month whose
      granularity is at the day level should set this to "1d". If you need
      the same data summarized at the hour level because you need to have weekly
      charts then use: "1h,1d".
    */
    #[serde(rename = "auto_summarize.timespan")]
    #[validate(regex = "RE_SKEW")]
    pub auto_summarize_timespan: Option<String>,
    /// Cron schedule to use to probe or generate the summaries for this search.
    #[serde(rename = "auto_summarize.cron_schedule")]
    #[validate(regex = "RE_CRON")]
    pub auto_summarize_cron_schedule: Option<String>,
    /// Any dispatch.* options that need to be overridden when running the summary search.
    #[serde(flatten)]
    pub auto_summarize_dispatch: Option<HashMap<String, String>>,
    /// The amount of time to suspend summarization of this search if the summarization is deemed unhelpful. Defaults to 24h.
    #[serde(rename = "auto_summarize.suspend_period")]
    pub auto_summarize_suspend_period: Option<String>,
    /// The minimum summary size when to start testing its helpfulness. Defaults to 52428800 (5MB).
    #[serde(rename = "auto_summarize.max_summary_size")]
    pub auto_summarize_max_summary_size: Option<u32>,
    /// The maximum ratio of summary_size/bucket_size when to stop summarization and deem it unhelpful for a bucket. Defaults to `0.1`.
    #[serde(rename = "auto_summarize.max_summary_ratio")]
    #[validate(range(min = 0.1))]
    pub auto_summarize_max_summary_ratio: Option<f64>,
    /*
    The maximum number of buckets with the suspended summarization before the
    summarization search is completely stopped and the summarization of the
    search is suspended for the value specified in the
    'auto_summarize.suspend_period' setting.
    Defaults to 2.
    */
    #[serde(rename = "auto_summarize.max_disabled_buckets")]
    pub auto_summarize_max_disabled_buckets: Option<u32>,
    /// The maximum amount of time that the summary search is allowed to run. Defaults to 3600.
    #[serde(rename = "auto_summarize.max_time")]
    pub auto_summarize_max_time: Option<u32>,
    /// An auto generated setting.
    #[serde(rename = "auto_summarize.hash")]
    pub auto_summarize_hash: Option<String>,
    /// An auto generated setting.
    #[serde(rename = "auto_summarize.normalized_hash")]
    pub auto_summarize_normalized_hash: Option<String>,
    /// The maximum number of concurrent instances of this auto summarizing search, that the scheduler is allowed to run. Defaults to 1.
    #[serde(rename = "auto_summarize.max_concurrent")]
    pub auto_summarize_max_concurrent: Option<u32>,
    /// Sets the name of the workload pool that is used by this auto summarization.
    #[serde(rename = "auto_summarize.workload_pool")]
    pub auto_summarize_workload_pool: Option<String>,

    // ******* alert suppression / severity / expiration / tracking / viewing settings *******
    /// Specifies whether alert suppression is enabled for this scheduled search. Defaults to false.
    #[serde(rename = "alert.suppress")]
    pub alert_suppress: Option<bool>,
    /// Sets the suppression period. Use [number][time-unit] to specify a time.
    #[serde(rename = "alert.suppress.period")]
    #[validate(regex = "RE_SKEW")]
    pub alert_suppress_period: Option<String>,
    #[serde(rename = "alert.suppress.fields")]
    /// List of fields to use when suppressing per-result alerts. This field *must* be specified if the digest mode is disabled and suppression is enabled.
    pub alert_suppress_fields: Option<String>,
    /*
    Use this setting to define an alert suppression group for a set of alerts
    that are running over the same or very similar datasets. Do this to avoid
    getting multiple triggered alert notifications for the same data.
    */
    #[serde(rename = "alert.suppress.group_name")]
    pub alert_suppress_group_name: Option<String>,
    /// Sets the alert severity level. Defaults to 3.
    #[serde(rename = "alert.severity")]
    #[validate(range(min = 0, max = 6))]
    pub alert_severity: Option<u32>,
    /// Sets the period of time to show the alert on the Triggered Alerts page. Defaults to 24h.
    #[validate(regex = "RE_SKEW")]
    #[serde(rename = "alert.expires")]
    pub alert_expires: Option<String>,
    /// Whether or not the Splunk platform applies the alert actions to the entire result set or to each individual result. Defaults to true.
    #[serde(rename = "alert.digest_mode")]
    pub alert_digest_mode: Option<bool>,
    /// Specifies whether to track the actions triggered by this scheduled search. Defaults to "auto".
    #[serde(rename = "alert.track")]
    pub alert_track: Option<types::AlertTrack>,
    /// Name of the UI view where the emailed link for each result alerts should point to.
    #[serde(rename = "alert.display_view")]
    pub alert_display_view: Option<String>,
    /// Specifies the feature or component that created the alert.
    #[serde(rename = "alert.managedBy")]
    pub alert_managed_by: Option<String>,

    // ******* UI-specific settings *******
    /// Defines the default UI view name (not label) in which to load the results.
    pub displayview: Option<String>,
    /// Defines the view state ID associated with the UI view listed in the 'displayview' setting.
    pub vsid: Option<String>,
    /// Specifies whether this saved search should be listed in the visible saved search list within apps. Defaults to true.
    pub is_visible: Option<bool>,
    /// Human-readable description of this saved search.
    pub description: Option<String>,
    /// Specifies a field used by Splunk UI to denote the app that this search should be dispatched in.
    #[serde(rename = "request.ui_dispatch_app")]
    pub request_ui_dispatch_app: Option<String>,
    /// Specifies a field used by Splunk UI to denote the view this search should be displayed in.
    #[serde(rename = "request.ui_dispatch_view")]
    pub request_ui_dispatch_view: Option<String>,

    // ******* Display Formatting Options *******
    // TODO: There's a lot of subfields in display. Some need to have their types mapped.
    #[serde(flatten)]
    pub display: Option<HashMap<String, serde_json::Value>>,

    // ******* Global settings *******
    #[serde(rename = "embed.enabled")]
    #[validate(range(min = 0, max = 1))]
    pub embed_enabled: Option<u8>,
    /// Specifies whether to defer a continuous saved search during a searchable rolling restart or searchable rolling upgrade of an indexer cluster. Defaults to false (disabled).
    pub defer_scheduled_searchable_idxc: Option<bool>,
    /// Specifies whether to skip a continuous saved realtime search during a searchable rolling restart or searchable rolling upgrade of an indexer cluster. Defaults to false (does not skip).
    pub skip_scheduled_realtime_idxc: Option<bool>,
    /// Whether or not the search scheduler pre-calculates the required fields from the alert condition search and uses the results in the main search. Defaults to false.
    pub precalculate_required_fields_for_alerts: Option<bool>,
    /// Whether or not alert search processes calculate the required fields of alert condition and alert action searches. Defaults to false.
    pub calculate_alert_required_fields_in_search: Option<bool>,
}
