use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use super::State;
use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use regex::Regex;
use reqwest::{
    header,
    header::{HeaderMap, HeaderName, HeaderValue},
    Certificate, Client, ClientBuilder, Method, RequestBuilder, Response, StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use url::Url;
use uuid::Uuid;

use super::BackendActions;

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Clone)]
pub struct HttpBackend {
    address: String,
    update_method: Option<String>,
    lock_address: Option<String>,
    unlock_address: Option<String>,
    lock_method: Option<String>,
    unlock_method: Option<String>,
    username: Option<String>,
    password: Option<String>,
    skip_cert_verification: Option<bool>,
    timeout: Option<u64>,
    client_ca_certificate_pem: Option<String>,
    client_certificate_pem: Option<String>,
    client_private_key_pem: Option<String>,
    headers: Option<HashMap<String, String>>,
}

impl HttpBackend {
    fn check_headers(&self) -> Result<HeaderMap> {
        let mut headermap = HeaderMap::new();
        if let Some(headers) = &self.headers {
            for (key, value) in headers {
                if key.is_empty() || value.is_empty() {
                    bail!("remote http state header key or value cannot be empty")
                }

                if !value.is_ascii() {
                    bail!("remote http state header value must only contain ascii characters")
                }

                if !Regex::new("[^a-zA-Z0-9-_]").unwrap().is_match(key) {
                    bail!("remote http state header key value must only contain A-Za-z0-9-_ characters")
                }

                if ["content-type", "content-md5"].contains(&key.to_lowercase().as_str()) {
                    bail!("remote http state header key {} is reserved", key)
                }

                headermap.insert(HeaderName::from_str(key)?, HeaderValue::from_str(value)?);
            }
        }
        Ok(headermap)
    }

    fn client(&self) -> Result<Client> {
        let headermap = self.check_headers()?;
        if headermap.get(header::AUTHORIZATION).is_some() && self.username.is_some() {
            bail!(
                "http remote state request headers {} cannot be set when providing username",
                header::AUTHORIZATION
            )
        }

        let client = ClientBuilder::new()
            .default_headers(headermap)
            .timeout(Duration::from_secs(self.timeout.unwrap_or(60)))
            .danger_accept_invalid_certs(self.skip_cert_verification.unwrap_or_default());

        // Set certificates
        let client = match (
            &self.client_ca_certificate_pem,
            &self.client_certificate_pem,
            &self.client_private_key_pem,
        ) {
            (Some(ca), Some(cert), Some(key)) => {
                let bundle = format!("{}\n{}\n{}", ca, cert, key);
                client.add_root_certificate(Certificate::from_pem(bundle.as_bytes())?)
            }
            (None, Some(cert), Some(key)) => {
                let bundle = format!("{}\n{}", cert, key);
                client.add_root_certificate(Certificate::from_pem(bundle.as_bytes())?)
            }
            (Some(ca), None, None) => {
                client.add_root_certificate(Certificate::from_pem(ca.as_bytes())?)
            }
            _ => client,
        };

        client
            .build()
            .map_err(|e| anyhow::anyhow!("unable to retrieve state: {}", e))
    }

    async fn send_auth(&self, req: RequestBuilder) -> Result<Response> {
        if let Some(usr) = &self.username {
            req.basic_auth(usr, self.password.clone()).send().await
        } else {
            req.send().await
        }
        .map_err(|e| anyhow::anyhow!("unable to retrieve state: {}", e))
    }

    async fn lock(&self, client: &Client, lock_address: &str) -> Result<Uuid> {
        let lock_method = self.lock_method.clone().unwrap_or("LOCK".to_string());

        let lock_id = Uuid::new_v4();

        let req = client
            .request(Method::from_str(&lock_method)?, Url::parse(lock_address)?)
            .query(&[("ID", &lock_id)]);

        match self.send_auth(req).await {
            Ok(resp) => match resp.status() {
                StatusCode::OK => Ok(lock_id),
                // StatusCode::CONFLICT => bail!("unable to lock state: already locked"),
                _ => bail!(
                    "unable to lock state: {} {}",
                    resp.status(),
                    resp.text().await?
                ),
            },
            Err(e) => bail!("unable to lock state: {}", e),
        }
    }

    async fn unlock(&self, client: &Client, lock_id: &str) -> Result<()> {
        let unlock_address = if let Some(address) = &self.unlock_address {
            address
        } else {
            return Ok(());
        };
        let unlock_method = self.unlock_method.clone().unwrap_or("UNLOCK".to_string());
        let req = client
            .request(
                Method::from_str(&unlock_method)?,
                Url::parse(unlock_address)?,
            )
            .query(&[("ID", lock_id)]);

        match self.send_auth(req).await {
            Ok(resp) => match resp.status() {
                StatusCode::OK => Ok(()),
                _ => bail!("unable to unlock state: {}", resp.status()),
            },
            Err(e) => bail!("unable to unlock state: {}", e),
        }
    }
}

#[async_trait]
impl BackendActions for HttpBackend {
    async fn load(&self) -> Result<State> {
        let client = self.client()?;

        let req = client.request(Method::GET, Url::from_str(&self.address)?);

        let resp = self.send_auth(req).await?;
        match resp.status() {
            StatusCode::OK => resp
                .json()
                .await
                .map_err(|e| anyhow::anyhow!("unable to decode state: {}", e)),
            StatusCode::NOT_FOUND => Ok(State::default()),
            _ => bail!("unable to retrieve state: {}", resp.status()),
        }
    }

    async fn save(&self, state: &mut State) -> anyhow::Result<()> {
        let client = self.client()?;

        state.serial += 1;
        state.lgc_version = env!("CARGO_PKG_VERSION").to_string();

        // Lock state - If lock address is not set ignore state locking
        let (req, lock_id) = if let Some(address) = &self.lock_address {
            let lock_id = self.lock(&client, address).await?;
            (
                client
                    .request(
                        Method::from_str(
                            self.update_method.as_ref().unwrap_or(&"POST".to_string()),
                        )?,
                        Url::from_str(&self.address)?,
                    )
                    .query(&[("ID", lock_id)])
                    .json(state),
                &self.lock_address,
            )
        } else {
            (
                client
                    .request(
                        Method::from_str(
                            self.update_method.as_ref().unwrap_or(&"POST".to_string()),
                        )?,
                        Url::from_str(&self.address)?,
                    )
                    .json(state),
                &None,
            )
        };

        match lock_id {
            Some(lock_id) => {
                self.send_auth(req)
                    .await
                    .map_err(|e| anyhow!("unable to save state: {}", e))?;
                self.unlock(&client, lock_id).await
            }
            None => {
                self.send_auth(req).await?;
                Ok(())
            }
        }
    }
}
