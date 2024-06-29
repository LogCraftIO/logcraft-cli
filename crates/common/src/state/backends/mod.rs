use super::State;
use anyhow::Result;
use async_trait::async_trait;
use local::LocalBackend;
use serde::{Deserialize, Serialize};

// Backends
mod http;
mod local;

use http::HttpBackend;

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum StateBackend {
    /// Local state backend
    Local(LocalBackend),
    /// Http state backend
    Http(Box<HttpBackend>),
}

impl StateBackend {
    pub async fn load(&self) -> Result<State> {
        match self {
            Self::Local(path) => path.load().await,
            Self::Http(backend) => backend.load().await,
        }
    }
}

impl Default for StateBackend {
    fn default() -> Self {
        Self::Local(LocalBackend::default())
    }
}

#[async_trait]
pub trait BackendActions {
    async fn load(&self) -> Result<State>;
    async fn save(&self, state: &mut State) -> Result<()>;
}
