// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr, time::Duration};
use url::{ParseError, Url};

use crate::bindings::exports::logcraft::lgc::plugin::Bytes;

const DEFAULT_USER: &str = "nobody";
const DEFAULT_APP: &str = "search";

// Regular expressions used for token validation
const RE_TOKEN: &str = r#"^(?:[A-Za-z0-9+/=]+|[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+)$"#;

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
/// Splunk backend configuration
pub struct Splunk {
    #[validate(url)]
    /// Splunk URL
    pub url: String,

    /// Authorization type
    auth_type: AuthorizationType,

    /// Authorization token
    #[validate(regex = "RE_TOKEN")]
    token: String,

    #[validate(range(min = 1, max = 60))]
    /// Timeout (seconds)
    timeout: u64,

    /// Application context
    app: Option<String>,

    /// User context
    user: Option<String>,
}

impl Default for Splunk {
    fn default() -> Self {
        Self {
            url: "https://splunk-server:8089".to_string(),
            auth_type: AuthorizationType::Bearer,
            token: "myToken==".to_string(),
            timeout: 30,
            app: Some(DEFAULT_APP.to_string()),
            user: Some(DEFAULT_USER.to_string()),
        }
    }
}

#[derive(Default, Serialize, Deserialize, schemars::JsonSchema)]
enum AuthorizationType {
    #[default]
    Bearer,
    Basic,
}

impl Display for AuthorizationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthorizationType::Bearer => write!(f, "Bearer"),
            AuthorizationType::Basic => write!(f, "Basic"),
        }
    }
}

impl Splunk {
    pub fn client(
        &self,
        method: waki::Method,
        path: &str,
    ) -> Result<waki::RequestBuilder, ParseError> {
        // Prepare the URI
        let uri = Url::from_str(&format!(
            "{}/servicesNS/{}/{}/saved/searches/",
            &self.url,
            self.user.as_deref().unwrap_or(DEFAULT_USER),
            self.app.as_deref().unwrap_or(DEFAULT_APP)
        ))?
        .join(path)?;

        // Build and return the client
        Ok(waki::Client::new()
            .request(method, uri.as_str())
            .connect_timeout(Duration::from_secs(self.timeout))
            .header(waki::header::AUTHORIZATION, self.format_token()))
    }

    pub fn deserialize(detection: &Bytes) -> Result<Self, String> {
        let mut de = serde_json::Deserializer::from_slice(detection);

        serde_path_to_error::deserialize(&mut de).map_err(|e| {
            format!(
                "field: {}, error: {}",
                e.path(),
                e.inner()
                    .to_string()
                    .split_once(" at")
                    .map(|(msg, _)| msg)
                    .unwrap_or(&e.inner().to_string())
            )
        })
    }

    pub fn check_app(&self) -> Result<(), String> {
        // Prepare the URI
        let uri = Url::from_str(&format!(
            "{}/services/apps/local/{}",
            &self.url,
            self.app.as_deref().unwrap_or(DEFAULT_APP)
        ))
        .map_err(|e| e.to_string())?;

        match waki::Client::new()
            .get(uri.as_str())
            .header(waki::header::AUTHORIZATION, self.format_token())
            .connect_timeout(std::time::Duration::from_secs(self.timeout))
            .send()
        {
            Ok(response) => match response.status_code() {
                200 => Ok(()),
                404 => Err(format!(
                    "target app '{}' not found",
                    self.app.as_deref().unwrap_or(DEFAULT_APP)
                )),
                code => Err(format!(
                    "unable to check target app '{}': {}",
                    self.app.as_deref().unwrap_or(DEFAULT_APP),
                    http::StatusCode::from_u16(code)
                        .map(|status| status.to_string())
                        .unwrap_or_else(|_| format!("HTTP/{} Invalid status code", code))
                )),
            },
            Err(e) => Err(e.to_string()),
        }
    }

    fn format_token(&self) -> String {
        format!("{} {}", self.auth_type, self.token)
    }
}
