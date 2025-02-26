// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use std::collections;

use crate::detections::PluginsDetections;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const LGC_DEFAULT_STATE_PATH: &str = ".logcraft/state.json";
const LGC_STATE_VERSION: usize = 1;

pub mod backends;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct State {
    /// State unique ID
    lineage: Uuid,
    /// Serial number of the state file.
    /// Increments every time the state file is written.
    serial: usize,
    /// Version of the state schema
    version: usize,
    /// Version of LogCraft CLI
    lgc_version: String,
    /// List of rules to track service_name => (rule_name, rule_settings)
    pub services: PluginsDetections,
}

impl Default for State {
    fn default() -> Self {
        Self {
            lineage: Uuid::new_v4(),
            serial: 0,
            version: LGC_STATE_VERSION,
            lgc_version: env!("CARGO_PKG_VERSION").to_string(),
            services: std::collections::HashMap::new(),
        }
    }
}

impl State {
    pub fn merge_synced(&mut self, detections: PluginsDetections) {
        for (service, plugin_rules) in detections {
            // If the service already exists, update or remove retrieved rules.
            if let Some(existing_rules) = self.services.get_mut(&service) {
                for (rule_key, rule_val) in plugin_rules {
                    if rule_val.is_null() {
                        existing_rules.remove(&rule_key);
                    } else {
                        existing_rules.insert(rule_key, rule_val);
                    }
                }
            } else {
                // Remove null values retrieved rules.
                let plugin_rules = plugin_rules
                    .into_iter()
                    .filter(|(_, val)| !val.is_null())
                    .collect();

                // Or insert the new service with its rules.
                self.services.insert(service, plugin_rules);
            }
        }
    }

    /// Consumes the detection data for the given service from the state
    /// and returns a mapping of rule keys to their JSON‐serialized values.
    ///
    /// If no detection data is found, an info message is logged and `Ok(None)` is returned.
    pub fn take_serialized_detections(
        &mut self,
        service_name: &str,
    ) -> Result<Option<collections::HashMap<String, Vec<u8>>>, serde_json::Error> {
        if let Some(detections) = self.services.remove(service_name) {
            // Return the serialized detections.
            Ok(Some(
                detections
                    .into_iter()
                    .map(|(rule_key, rule_val)| {
                        // If serialization fails, propagate the error.
                        Ok((rule_key, serde_json::to_vec(&rule_val)?))
                    })
                    .collect::<Result<collections::HashMap<_, _>, _>>()?,
            ))
        } else {
            Ok(None)
        }
    }
}
