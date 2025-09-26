use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;


/// OAuth 2.0 Token Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// Stored token information with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub access_token: String,
    pub token_type: String,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub server_url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Dynamic Client Registration Request (RFC 7591)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRegistrationRequest {
    pub redirect_uris: Vec<String>,
    pub client_name: Option<String>,
    pub client_uri: Option<String>,
    pub logo_uri: Option<String>,
    pub scope: Option<String>,
    pub contacts: Option<Vec<String>>,
    pub tos_uri: Option<String>,
    pub policy_uri: Option<String>,
    pub jwks_uri: Option<String>,
    pub software_id: Option<String>,
    pub software_version: Option<String>,

    // Additional fields for MCP
    #[serde(flatten)]
    pub additional_metadata: HashMap<String, serde_json::Value>,
}

/// Dynamic Client Registration Response (RFC 7591)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRegistrationResponse {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub client_id_issued_at: Option<u64>,
    pub client_secret_expires_at: Option<u64>,
    pub redirect_uris: Vec<String>,
    pub token_endpoint_auth_method: Option<String>,
    pub grant_types: Option<Vec<String>>,
    pub response_types: Option<Vec<String>>,
    pub client_name: Option<String>,
    pub client_uri: Option<String>,
    pub logo_uri: Option<String>,
    pub scope: Option<String>,
    pub contacts: Option<Vec<String>>,
    pub tos_uri: Option<String>,
    pub policy_uri: Option<String>,
    pub jwks_uri: Option<String>,
    pub software_id: Option<String>,
    pub software_version: Option<String>,
    pub registration_access_token: Option<String>,
    pub registration_client_uri: Option<String>,

    #[serde(flatten)]
    pub additional_metadata: HashMap<String, serde_json::Value>,
}

/// Static OAuth Client Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticClientInfo {
    pub client_id: String,
    pub client_secret: Option<String>,
}

/// OAuth Server Metadata Discovery (RFC 8414)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthServerMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub registration_endpoint: Option<String>,
    pub jwks_uri: Option<String>,
    pub response_types_supported: Option<Vec<String>>,
    pub grant_types_supported: Option<Vec<String>>,
    pub token_endpoint_auth_methods_supported: Option<Vec<String>>,
    pub scopes_supported: Option<Vec<String>>,
    pub code_challenge_methods_supported: Option<Vec<String>>,

    #[serde(flatten)]
    pub additional_metadata: HashMap<String, serde_json::Value>,
}

/// PKCE (Proof Key for Code Exchange) parameters
#[derive(Debug, Clone)]
pub struct PkceChallenge {
    pub code_verifier: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}

/// OAuth Authorization Request parameters
#[derive(Debug, Clone)]
pub struct AuthorizationRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: String,
    pub pkce_challenge: PkceChallenge,
}

/// OAuth Authorization Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationResponse {
    pub code: String,
    pub state: String,
}

/// OAuth Configuration
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub server_url: String,
    pub server_metadata: Option<OAuthServerMetadata>,
    pub static_client_info: Option<StaticClientInfo>,
    pub callback_port: Option<u16>,
    pub callback_host: String,
    pub auth_timeout_secs: u64,
    pub scope: Option<String>,
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            server_metadata: None,
            static_client_info: None,
            callback_port: None,
            callback_host: "localhost".to_string(),
            auth_timeout_secs: 300, // 5 minutes
            scope: None,
        }
    }
}
