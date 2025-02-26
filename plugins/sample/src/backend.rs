// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};

const RE_TOKEN: &str = r#"^([A-Za-z0-9+/=]+|\w+\.\w+\.\w+)$"#;

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub(super) struct SampleBackend {
    // Custom definitions
    /// Authorization scheme
    authorization_scheme: AuthorizationScheme,

    // Common types
    #[validate(length(min = 1, max = 10))]
    /// Backend name
    name: String,

    #[validate(url)]
    /// Backend URL
    url: String,

    #[validate(email)]
    /// Contact email
    email: String,

    #[validate(regex = "RE_TOKEN")]
    /// Authorization token
    token: String,

    #[validate(range(min = 0, max = 30))]
    /// Timeout in seconds
    timeout: u64,

    /// Custom type
    custom_type: CustomType,
}

impl Default for SampleBackend {
    fn default() -> Self {
        Self {
            authorization_scheme: AuthorizationScheme::Bearer,
            name: "dev".to_string(),
            url: "https://example.com".to_string(),
            email: "john.doe@foo.bar".to_string(),
            token: "someToken".to_string(),
            timeout: 10,
            custom_type: CustomType::default(),
        }
    }
}

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
struct CustomType {
    custom_field: String,
}

impl Default for CustomType {
    fn default() -> Self {
        Self {
            custom_field: "custom_field_value".to_string(),
        }
    }
}

#[derive(Default, Serialize, Deserialize, schemars::JsonSchema)]
enum AuthorizationScheme {
    #[default]
    Bearer,
    Basic,
}
