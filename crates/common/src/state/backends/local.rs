use crate::state::LGC_DEFAULT_STATE_PATH;
use anyhow::{anyhow, Ok, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{fs, io, path};

use super::State;

use super::BackendActions;

#[derive(Serialize, Deserialize, Clone)]
pub struct LocalBackend {
    path: path::PathBuf,
}

impl Default for LocalBackend {
    fn default() -> Self {
        Self {
            path: path::PathBuf::from(LGC_DEFAULT_STATE_PATH),
        }
    }
}

#[async_trait]
impl BackendActions for LocalBackend {
    async fn load(&self) -> Result<State> {
        if !self.path.is_file() {
            return Ok(State::default());
        }

        let f = fs::File::open(&self.path)?;
        let reader = io::BufReader::new(f);

        serde_json::from_reader(reader).map_err(|e| anyhow!("unable to load state file: {}", e))
    }

    async fn save(&self, state: &mut State) -> anyhow::Result<()> {
        let f = fs::File::create(&self.path)?;

        state.serial += 1;
        state.lgc_version = env!("CARGO_PKG_VERSION").to_string();

        let writer = io::BufWriter::new(f);
        serde_json::to_writer_pretty(writer, state)
            .map_err(|e| anyhow!("unable to write state file: {}", e))
    }
}
