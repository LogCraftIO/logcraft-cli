// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use crate::detections::{DetectionState, ServiceDetections};
use anyhow::Result;
use console::style;
use dashmap::DashMap;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

const LGC_DEFAULT_STATE_PATH: &str = ".logcraft/state.json";
const LGC_STATE_VERSION: usize = 1;

pub mod backends;
use backends::{BackendActions, StateBackend};

#[derive(Debug, Serialize, Deserialize)]
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
    pub services: ServiceDetections,
}

impl Default for State {
    fn default() -> Self {
        Self {
            lineage: Uuid::new_v4(),
            serial: 0,
            version: LGC_STATE_VERSION,
            lgc_version: env!("CARGO_PKG_VERSION").to_string(),
            services: HashMap::new(),
        }
    }
}

impl State {
    pub async fn save(&mut self, backend: &StateBackend) -> Result<()> {
        match backend {
            StateBackend::Local(path) => path.save(self).await,
            StateBackend::Http(backend) => backend.save(self).await,
        }
    }

    pub fn missing_rules(&self, detections: &ServiceDetections, silent: bool) -> ServiceDetections {
        let to_remove: DashMap<String, HashSet<DetectionState>> = DashMap::new();

        detections.par_iter().for_each(|(service_id, rules)| {
            if let Some(state_rules) = self.services.get(service_id) {
                state_rules.difference(rules).for_each(|rule| {
                    to_remove
                        .entry(service_id.clone())
                        .and_modify(|s| {
                            s.insert(rule.clone());
                        })
                        .or_insert(HashSet::from([rule.clone()]));
                    if !silent {
                        println!(
                            "[-] rule: `{}` will be deleted from `{}`",
                            style(&rule.name).red(),
                            &service_id
                        );
                    }
                });
            }
        });

        to_remove.into_iter().collect()
    }
}
