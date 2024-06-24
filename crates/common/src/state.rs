// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use crate::detections::{DetectionState, ServiceDetections};
use anyhow::{anyhow, bail, Result};
use console::style;
use dashmap::DashMap;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{BufReader, BufWriter},
    path::PathBuf,
};
use uuid::Uuid;

const LGC_STATE_PATH: &str = ".logcraft/state.json";
const LGC_STATE_VERSION: usize = 1;

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

impl State {
    pub fn clean(&mut self) -> Result<()> {
        self.services.clear();
        self.write()
    }

    pub fn read() -> Result<Self> {
        let path = PathBuf::from(LGC_STATE_PATH);
        if !path.is_file() {
            return Ok(Self {
                lineage: Uuid::new_v4(),
                serial: 0,
                version: LGC_STATE_VERSION,
                lgc_version: env!("CARGO_PKG_VERSION").to_string(),
                services: HashMap::new(),
            });
        }

        let f = fs::File::open(path)?;
        let reader = BufReader::new(f);

        match serde_json::from_reader(reader) {
            Ok(state) => Ok(state),
            Err(e) => {
                bail!("unable to load state file: {}", e)
            }
        }
    }

    pub fn write(&mut self) -> Result<()> {
        let f = fs::File::create(LGC_STATE_PATH)?;

        self.serial += 1;
        self.lgc_version = env!("CARGO_PKG_VERSION").to_string();

        let writer = BufWriter::new(f);
        serde_json::to_writer_pretty(writer, self)
            .map_err(|e| anyhow!("unable to write state file: {}", e))
    }

    pub fn missing_rules(&self, detections: &ServiceDetections) -> ServiceDetections {
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
                    println!(
                        "[-] rule: `{}` will be deleted from `{}`",
                        style(&rule.name).red(),
                        &service_id
                    );
                });
            }
        });

        to_remove.into_iter().collect()
    }
}
