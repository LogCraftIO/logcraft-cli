// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use anyhow::{bail, Context, Result};
use fs4::tokio::AsyncFileExt;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use tokio::{fs, sync::Mutex};
use uuid::Uuid;

use super::BackendActions;
use crate::state::{State, LGC_DEFAULT_STATE_PATH};

// Define the ENOLCK error code (37 on Linux)
const ENOLCK: i32 = 37;

#[derive(Serialize, Deserialize, Clone)]
/// Mimic OpenTofu HTTP state backend configuration variables.
/// [Documentation](https://opentofu.org/docs/language/settings/backends/http)
pub struct LocalBackend {
    /// REST endpoint.
    path: PathBuf,
    /// This field is skipped during serialization/deserialization.
    #[serde(skip)]
    lock_file: Arc<Mutex<Option<fs::File>>>,
}

impl Default for LocalBackend {
    fn default() -> Self {
        Self {
            path: PathBuf::from(LGC_DEFAULT_STATE_PATH),
            lock_file: Arc::new(Mutex::new(None)),
        }
    }
}

impl BackendActions for LocalBackend {
    /// Loads the state.
    /// If the file does not exist, returns (false, State::default()).
    async fn load(&self) -> Result<(bool, State)> {
        if fs::metadata(&self.path).await.is_err() {
            return Ok((false, State::default()));
        }
        let contents = fs::read_to_string(&self.path)
            .await
            .with_context(|| format!("unable to read state file: {}", self.path.display()))?;
        let state: State = serde_json::from_str(&contents)
            .with_context(|| format!("unable to parse state file: {}", self.path.display()))?;
        Ok((true, state))
    }

    /// Saves the state.
    async fn save(&self, state: &mut State) -> Result<()> {
        state.serial += 1;
        state.lgc_version = env!("CARGO_PKG_VERSION").to_string();
        let contents = serde_json::to_string_pretty(state).with_context(|| {
            format!(
                "unable to serialize state for file: {}",
                self.path.display()
            )
        })?;

        // Create parent directories if they don't exist.
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).await.with_context(|| {
                format!("unable to create directories for {}", parent.display())
            })?;
        }

        // Write the state to disk.
        fs::write(&self.path, contents)
            .await
            .with_context(|| format!("unable to write state file: {}", self.path.display()))
    }

    /// Locks the state.
    /// The locked file handle is stored so that the lock remains active.
    async fn lock(&self) -> Result<Option<Uuid>> {
        // Try to open the file. If it doesn't exist, just skip locking.
        let file = match fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .await
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // File not found, so we ignore locking.
                return Ok(None);
            }
            Err(e) => {
                bail!("unable to open state file {}: {}", self.path.display(), e);
            }
        };

        if let Err(e) = file.try_lock_exclusive() {
            if let Some(code) = e.raw_os_error() {
                if code == ENOLCK {
                    // Proceed without locking.
                    tracing::warn!(
                        "filesystem does not support file locking on `{}`; proceeding without lock",
                        self.path.display()
                    );
                } else {
                    match e.kind() {
                        std::io::ErrorKind::WouldBlock => {
                            bail!("state file `{}` is locked", self.path.display());
                        }
                        _ => {
                            bail!(
                                "unable to acquire lock on state file `{}`: {}",
                                self.path.display(),
                                e
                            )
                        }
                    }
                }
            } else {
                bail!(
                    "unable to acquire lock on state file `{}`: {}",
                    self.path.display(),
                    e
                );
            }
        }

        // Store the file handle in our async lock.
        let mut guard = self.lock_file.lock().await;
        *guard = Some(file);
        Ok(None)
    }

    /// Unlocks the state.
    async fn unlock(&self, _lock_token: Option<Uuid>) -> Result<()> {
        let mut guard = self.lock_file.lock().await;
        if let Some(file) = guard.as_mut() {
            // Try to unlock the file.
            match file.unlock_async().await {
                Ok(()) => {}
                // If the unlock fails because the file is gone, ignore that error.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    return Err(e).with_context(|| {
                        format!("unable to unlock state file {}", self.path.display())
                    });
                }
            }
        }
        // Clear the stored file handle.
        *guard = None;
        Ok(())
    }
}
