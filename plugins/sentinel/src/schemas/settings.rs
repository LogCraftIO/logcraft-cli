// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr, time::Duration};
use url::Url;
use uuid::Uuid;

use crate::bindings::exports::logcraft::lgc::plugin::Bytes;

const AZURE_AUTH_DEFAULT_ENDPOINT: &str = "https://login.microsoftonline.com";
const AZURE_MGT_ENDPOINT: &str = "https://management.azure.com";
const AZURE_MGT_SCOPE: &str = ".default";
// const AZURE_API_VERSION: &str = "2023-09-01";
const AZURE_API_VERSION: &str = "2024-09-01";

// Regular expressions used for token validation
const RE_IDS: &str = r#"^[A-Za-z0-9][A-Za-z0-9-]+[A-Za-z0-9]$"#;

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
/// Splunk backend configuration
pub struct Sentinel {
    /// Azure client id
    #[validate(regex = "RE_IDS")]
    pub client_id: String,

    /// Azure client secret
    #[validate(regex = "RE_IDS")]
    pub client_secret: String,

    /// Azure subscription id
    pub subscription_id: uuid::Uuid,

    /// Azure tenant id
    pub tenant_id: String,

    /// Azure api version
    pub api_version: Option<String>,

    /// Azure resource group name
    pub resource_group: String,

    /// Azure workspace name
    #[validate(regex = "RE_IDS")]
    pub workspace: String,

    /// Timeout in seconds
    #[validate(range(min = 1, max = 60))]
    pub timeout: Option<u64>,

    /// Azure auth endpoint
    pub auth_endpoint: Option<String>,

    /// Azure management endpoint
    pub management_endpoint: Option<String>,

    /// Azure management scope
    pub management_scope: Option<String>,
}

impl Default for Sentinel {
    fn default() -> Self {
        Self {
            client_id: "AZURE_CLIENT_ID".to_string(),
            client_secret: "AZURE_CLIENT_SECRET".to_string(),
            tenant_id: "azure-tenant-id".to_string(),
            subscription_id: Uuid::default(),
            api_version: Some(AZURE_API_VERSION.to_string()),
            resource_group: "my-resource-group".to_string(),
            workspace: "my-workspace".to_string(),
            timeout: Some(30),
            auth_endpoint: Some(AZURE_AUTH_DEFAULT_ENDPOINT.to_string()),
            management_endpoint: Some(AZURE_MGT_ENDPOINT.to_string()),
            management_scope: Some(AZURE_MGT_SCOPE.to_string()),
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

impl Sentinel {
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

    pub fn client(&self, method: waki::Method, path: &str) -> Result<waki::RequestBuilder, String> {
        /*
        PUT https://management.azure.com/
            subscriptions/d0cfe6b2-9ac0-4464-9919-dccaee2e48c0/
            resourceGroups/myRg/
            providers/Microsoft.OperationalInsights/
            workspaces/myWorkspace/providers/Microsoft.SecurityInsights/
            alertRules/myFirstFusionRule?api-version=2024-09-01
        */
        // Prepare the URI
        let uri = Url::from_str(&format!(
            "{}/subscriptions/{}/resourceGroups/{}/providers/Microsoft.OperationalInsights/workspaces/{}/providers/Microsoft.SecurityInsights/alertRules/",
            self.management_endpoint.as_deref().unwrap_or(AZURE_MGT_ENDPOINT),
            &self.subscription_id,
            &self.resource_group,
            &self.workspace
        ))
        .map_err(|e| e.to_string())?
        .join(path)
        .map_err(|e| e.to_string())?;

        // Build and return the client
        Ok(waki::Client::new()
            .request(method, uri.as_str())
            .connect_timeout(Duration::from_secs(self.timeout.unwrap_or(60)))
            .header(waki::header::AUTHORIZATION, self.get_credentials()?)
            .query(&[(
                "api-version",
                self.api_version.as_deref().unwrap_or(AZURE_API_VERSION),
            )]))
    }

    fn get_credentials(&self) -> Result<String, String> {
        let req = waki::Client::new()
            .post(&format!(
                "{AZURE_AUTH_DEFAULT_ENDPOINT}/{}/oauth2/v2.0/token",
                self.tenant_id
            ))
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
                (
                    "scope",
                    [AZURE_MGT_ENDPOINT, AZURE_MGT_SCOPE].join("/").as_str(),
                ),
            ]);

        match req.send() {
            Ok(resp) => match resp.status_code() {
                200 => {
                    let resp: AzureAuthz = serde_json::from_slice(
                        &resp
                            .body()
                            .map_err(|e| format!("unable to parse azure authz response: {e}"))?,
                    )
                    .map_err(|e| format!("unable to parse azure authz response: {e}"))?;

                    // return Err(resp.access_token);
                    Ok(format!(
                        "{} {}",
                        AuthorizationType::Bearer,
                        resp.access_token
                    ))
                }
                _ => Err(AzureError::from_slices(
                    resp.body()
                        .map_err(|e| format!("invalid UTF-8 response: {e}"))?,
                )),
            },
            Err(e) => Err(format!("{}", e)),
        }
    }

    pub fn check_workspace(&self) -> Result<(), String> {
        let workspace_endpoint = format!(
            "{AZURE_MGT_ENDPOINT}/subscriptions/{}/resourcegroups/{}/providers/Microsoft.OperationalInsights/workspaces/{}/providers/Microsoft.SecurityInsights/alertRules",
            self.subscription_id,
            self.resource_group,
            self.workspace,
        );

        match waki::Client::new()
            .get(&workspace_endpoint)
            .header("Authorization", self.get_credentials()?)
            .query(&[(
                "api-version",
                self.api_version.as_deref().unwrap_or(AZURE_API_VERSION),
            )])
            .send()
        {
            Ok(resp) => match resp.status_code() {
                200 => Ok(()),
                _ => Err(AzureError::from_slices(
                    resp.body()
                        .map_err(|e| format!("invalid UTF-8 response: {e}"))?,
                )),
            },
            Err(e) => Err(e.to_string()),
        }
    }
}

#[derive(Deserialize)]
struct AzureAuthz {
    access_token: String,
}

#[derive(Deserialize)]
pub struct AzureError {
    error: ErrorBody,
}

#[derive(Deserialize)]
struct ErrorBody {
    code: String,
    message: String,
}

impl Display for AzureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (code: {})", self.error.message, self.error.code)
    }
}

impl AzureError {
    pub fn from_slices(body: Vec<u8>) -> String {
        match serde_json::from_slice::<Self>(&body) {
            Ok(resp) => format!("{}: {}", resp.error.code, resp.error.message),
            Err(_) => String::from_utf8(body).unwrap_or("empty response".to_string()),
        }
    }
}
