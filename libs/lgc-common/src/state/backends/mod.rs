// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

mod http;
mod local;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents the state backend configuration.
#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StateBackend {
    /// Local state backend.
    Local(local::LocalBackend),
    /// HTTP state backend.
    Http(Box<http::HttpBackend>),
}

impl StateBackend {
    /// Loads the state.
    pub async fn load(&self) -> Result<(bool, super::State)> {
        match self {
            Self::Local(backend) => backend.load().await,
            Self::Http(backend) => backend.load().await,
        }
    }

    /// Saves the state.
    pub async fn save(&self, state: &mut super::State) -> Result<()> {
        match self {
            Self::Local(backend) => backend.save(state).await,
            Self::Http(backend) => backend.save(state).await,
        }
    }

    /// Locks the state.
    pub async fn lock(&self) -> Result<Option<Uuid>> {
        match self {
            Self::Local(backend) => backend.lock().await,
            Self::Http(backend) => backend.lock().await,
        }
    }

    /// Unlocks the state.
    pub async fn unlock(&self, token: Option<Uuid>) -> Result<()> {
        match self {
            Self::Local(backend) => backend.unlock(token).await,
            Self::Http(backend) => backend.unlock(token).await,
        }
    }
}

impl Default for StateBackend {
    fn default() -> Self {
        Self::Local(local::LocalBackend::default())
    }
}

/// State backends actions.
pub trait BackendActions {
    fn load(
        &self,
    ) -> impl std::future::Future<Output = anyhow::Result<(bool, super::State)>> + Send;
    fn save(
        &self,
        state: &mut super::State,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
    fn lock(&self) -> impl std::future::Future<Output = anyhow::Result<Option<uuid::Uuid>>> + Send;
    fn unlock(
        &self,
        token: Option<uuid::Uuid>,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}
