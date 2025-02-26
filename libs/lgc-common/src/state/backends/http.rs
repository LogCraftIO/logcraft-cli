// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, Result};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use uuid::Uuid;

use super::BackendActions;
use crate::state::State;

/// Default values for HTTP backend settings.
const DEFAULT_UPDATE_METHOD: &str = "POST";
const DEFAULT_LOCK_METHOD: &str = "LOCK";
const DEFAULT_UNLOCK_METHOD: &str = "UNLOCK";
const DEFAULT_RETRY_MAX: u32 = 2;
const DEFAULT_RETRY_WAIT_MIN: u64 = 1;
const DEFAULT_RETRY_WAIT_MAX: u64 = 30;

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Clone)]
/// Mimic OpenTofu HTTP state backend configuration variables.
/// [Documentation](https://opentofu.org/docs/language/settings/backends/http)
pub struct HttpBackend {
    /// REST endpoint.
    pub address: String,
    /// Update method.
    pub update_method: Option<String>,
    /// Lock REST endpoint. If not set, locking is disabled.
    pub lock_address: Option<String>,
    pub lock_method: Option<String>,
    /// Unlock REST endpoint. If not set, unlocking is disabled.
    pub unlock_address: Option<String>,
    pub unlock_method: Option<String>,
    /// HTTP Basic authentication username & password.
    pub username: Option<String>,
    pub password: Option<String>,
    pub skip_cert_verification: Option<bool>,
    /// The number of HTTP request retries.
    pub retry_max: Option<u32>,
    /// The minimum time in seconds to wait between HTTP request attempts.
    pub retry_wait_min: Option<u64>,
    /// The maximum time in seconds to wait between HTTP request attempts.
    pub retry_wait_max: Option<u64>,
    /// CA certificate in PEM format.
    pub client_ca_certificate_pem: Option<String>,
    /// Client certificate in PEM format.
    pub client_certificate_pem: Option<String>,
    /// Client private key in PEM format.
    pub client_private_key_pem: Option<String>,
    /// Extra HTTP headers.
    pub headers: Option<HashMap<String, String>>,
}

impl HttpBackend {
    /// Builds and configures the HTTP client.
    fn build_client(&self) -> Result<Client> {
        let mut builder = Client::builder();

        if self.skip_cert_verification.unwrap_or(false) {
            builder = builder.danger_accept_invalid_certs(true);
        }

        if let Some(ref ca_pem) = self.client_ca_certificate_pem {
            let cert = reqwest::Certificate::from_pem(ca_pem.as_bytes())
                .map_err(|e| anyhow!("state backend invalid CA certificate: {}", e))?;
            builder = builder.add_root_certificate(cert);
        }

        if let (Some(cert_pem), Some(key_pem)) =
            (&self.client_certificate_pem, &self.client_private_key_pem)
        {
            let identity_pem = format!("{}\n{}", cert_pem, key_pem);
            let identity = reqwest::Identity::from_pem(identity_pem.as_bytes())
                .map_err(|e| anyhow!("state backend invalid client identity: {}", e))?;
            builder = builder.identity(identity);
        }

        if let Some(ref headers_map) = self.headers {
            let mut headers = reqwest::header::HeaderMap::new();
            for (key, value) in headers_map {
                let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
                    .map_err(|e| anyhow!("state backend invalid header name '{}': {}", key, e))?;
                let header_value = reqwest::header::HeaderValue::from_str(value).map_err(|e| {
                    anyhow!("state backend invalid header value for '{}': {}", key, e)
                })?;
                headers.insert(header_name, header_value);
            }
            builder = builder.default_headers(headers);
        }

        builder
            .build()
            .map_err(|e| anyhow!("failed to build HTTP client for state backend: {}", e))
    }

    /// Executes an HTTP operation with retry logic using exponential backoff.
    async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = reqwest::Result<T>>,
    {
        let max_retries = self.retry_max.unwrap_or(DEFAULT_RETRY_MAX);
        let wait_min = self.retry_wait_min.unwrap_or(DEFAULT_RETRY_WAIT_MIN);
        let wait_max = self.retry_wait_max.unwrap_or(DEFAULT_RETRY_WAIT_MAX);

        let mut attempt = 0;
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    attempt += 1;
                    if attempt > max_retries {
                        return Err(anyhow!(
                            "state operation failed after `{}` attempts: {}",
                            attempt,
                            err
                        ));
                    }
                    let wait_secs = std::cmp::min(wait_min * 2u64.pow(attempt - 1), wait_max);
                    sleep(Duration::from_secs(wait_secs)).await;
                }
            }
        }
    }
}

impl BackendActions for HttpBackend {
    /// Loads the state.
    async fn load(&self) -> Result<(bool, State)> {
        let client = self.build_client()?;
        let url = &self.address;
        let username = self.username.as_deref();
        let password = self.password.as_deref();

        let response = self
            .execute_with_retry(|| {
                let mut req = client.get(url);
                if let Some(user) = username {
                    req = req.basic_auth(user, password);
                }
                req.send()
            })
            .await
            .map_err(|e| anyhow!("state loading request failed: {}", e))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok((false, State::default()));
        }
        if !response.status().is_success() {
            return Err(anyhow!(
                "state loading request failed with status: {}",
                response.status()
            ));
        }
        let state: State = response
            .json()
            .await
            .map_err(|e| anyhow!("unable to parse state load response: {}", e))?;
        Ok((true, state))
    }

    /// Saves the state.
    async fn save(&self, state: &mut State) -> Result<()> {
        let client = self.build_client()?;
        // Update state metadata.
        state.serial += 1;
        state.lgc_version = env!("CARGO_PKG_VERSION").to_string();

        let method_str = self
            .update_method
            .as_deref()
            .unwrap_or(DEFAULT_UPDATE_METHOD);
        let method = method_str.parse::<Method>().map_err(|e| {
            anyhow!(
                "state backend invalid update method '{}': {}",
                method_str,
                e
            )
        })?;
        let url = &self.address;
        let username = self.username.as_deref();
        let password = self.password.as_deref();

        let response = self
            .execute_with_retry(|| {
                let mut req = client.request(method.clone(), url);
                if let Some(ref user) = username {
                    req = req.basic_auth(user, password.as_ref());
                }
                req.json(&state).send()
            })
            .await
            .map_err(|e| anyhow!("state save request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "state save request failed with status: {}",
                response.status()
            ));
        }
        Ok(())
    }

    /// Locks the state.
    async fn lock(&self) -> Result<Option<Uuid>> {
        let url = match &self.lock_address {
            Some(addr) => addr,
            // Locking is disabled.
            None => return Ok(None),
        };

        let client = self.build_client()?;
        let method_str = self.lock_method.as_deref().unwrap_or(DEFAULT_LOCK_METHOD);
        let method = method_str
            .parse::<Method>()
            .map_err(|e| anyhow!("state backend invalid lock method '{}': {}", method_str, e))?;
        let username = self.username.as_deref();
        let password = self.password.as_deref();

        let response = self
            .execute_with_retry(|| {
                let mut req = client.request(method.clone(), url);
                if let Some(ref user) = username {
                    req = req.basic_auth(user, password.as_ref());
                }
                req.send()
            })
            .await
            .map_err(|e| anyhow!("state lock request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "state lock request failed with status: {}",
                response.status()
            ));
        }

        let text = response
            .text()
            .await
            .map_err(|e| anyhow!("failed to read state lock response: {}", e))?;
        if text.trim().is_empty() {
            return Ok(None);
        }
        #[derive(Deserialize)]
        struct LockResponse {
            lock_id: Option<Uuid>,
        }
        let lock_response: LockResponse = serde_json::from_str(&text)
            .map_err(|e| anyhow!("failed to parse state lock response: {}", e))?;
        Ok(lock_response.lock_id)
    }

    /// Unlocks the state by sending an HTTP request to the configured unlock address.
    async fn unlock(&self, lock_token: Option<Uuid>) -> Result<()> {
        let url = match &self.unlock_address {
            Some(addr) => addr,
            // Unlocking is disabled.
            None => return Ok(()),
        };

        let client = self.build_client()?;
        let method_str = self
            .unlock_method
            .as_deref()
            .unwrap_or(DEFAULT_UNLOCK_METHOD);
        let method = method_str.parse::<Method>().map_err(|e| {
            anyhow!(
                "state backend invalid unlock method '{}': {}",
                method_str,
                e
            )
        })?;
        let username = self.username.as_deref();
        let password = self.password.as_deref();

        let response = self
            .execute_with_retry(|| {
                let mut req = client.request(method.clone(), url);
                if let Some(ref user) = username {
                    req = req.basic_auth(user, password.as_ref());
                }
                if let Some(lock_id) = lock_token {
                    #[derive(Serialize)]
                    struct UnlockPayload {
                        lock_id: Uuid,
                    }
                    let payload = UnlockPayload { lock_id };
                    req = req.json(&payload);
                }
                req.send()
            })
            .await
            .map_err(|e| anyhow!("state unlock request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "state unlock request failed with status: {}",
                response.status()
            ));
        }
        Ok(())
    }
}
