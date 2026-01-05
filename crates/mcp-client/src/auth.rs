use crate::error::{ClientError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthClientConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_url: String,
    pub scopes: Vec<String>,
    /// Use PKCE (Proof Key for Code Exchange) - recommended for public clients
    #[serde(default = "default_use_pkce")]
    pub use_pkce: bool,
}

fn default_use_pkce() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
    pub scope: Vec<String>,
}

/// OAuth token response from the authorization server
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: Option<u64>,
    refresh_token: Option<String>,
    scope: Option<String>,
}

/// PKCE state for the OAuth flow
#[derive(Debug, Clone)]
struct PkceState {
    code_verifier: String,
    code_challenge: String,
}

/// Authorization state stored during OAuth flow
#[derive(Debug, Clone)]
struct AuthState {
    state: String,
    pkce: Option<PkceState>,
}

pub struct OAuthClient {
    config: OAuthClientConfig,
    http_client: Client,
    token: Arc<RwLock<Option<ClientToken>>>,
    auth_state: Arc<RwLock<Option<AuthState>>>,
}

impl OAuthClient {
    pub fn new(config: OAuthClientConfig) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ClientError::OAuthError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            http_client,
            token: Arc::new(RwLock::new(None)),
            auth_state: Arc::new(RwLock::new(None)),
        })
    }

    /// Generate a cryptographically secure random string for PKCE
    fn generate_code_verifier() -> String {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        let random_bytes: [u8; 32] = rand::random();
        URL_SAFE_NO_PAD.encode(random_bytes)
    }

    /// Generate the code challenge from the verifier using S256 method
    fn generate_code_challenge(verifier: &str) -> String {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        URL_SAFE_NO_PAD.encode(hash)
    }

    pub async fn generate_auth_url(&self) -> Result<String> {
        let state = format!("state_{}", uuid::Uuid::new_v4());

        // Generate PKCE if enabled
        let pkce = if self.config.use_pkce {
            let code_verifier = Self::generate_code_verifier();
            let code_challenge = Self::generate_code_challenge(&code_verifier);
            Some(PkceState {
                code_verifier,
                code_challenge,
            })
        } else {
            None
        };

        // Store the state and PKCE for later verification
        {
            let mut auth_state = self.auth_state.write().await;
            *auth_state = Some(AuthState {
                state: state.clone(),
                pkce: pkce.clone(),
            });
        }

        let scopes = self.config.scopes.join(" ");
        let mut auth_url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
            self.config.auth_url,
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(&self.config.redirect_url),
            urlencoding::encode(&scopes),
            urlencoding::encode(&state)
        );

        // Add PKCE parameters if enabled
        if let Some(ref pkce_state) = pkce {
            auth_url.push_str(&format!(
                "&code_challenge={}&code_challenge_method=S256",
                urlencoding::encode(&pkce_state.code_challenge)
            ));
        }

        debug!("Generated OAuth authorization URL: {}", auth_url);
        Ok(auth_url)
    }

    pub async fn exchange_code(&self, code: &str, state: &str) -> Result<ClientToken> {
        // Verify state and get PKCE verifier
        let auth_state = {
            let auth_state = self.auth_state.read().await;
            auth_state.clone()
        };

        let auth_state = auth_state
            .ok_or_else(|| ClientError::OAuthError("No auth state found".to_string()))?;

        if auth_state.state != state {
            return Err(ClientError::OAuthError("State mismatch - possible CSRF attack".to_string()));
        }

        // Build token request
        let mut params = vec![
            ("grant_type", "authorization_code".to_string()),
            ("code", code.to_string()),
            ("redirect_uri", self.config.redirect_url.clone()),
            ("client_id", self.config.client_id.clone()),
        ];

        // Add client secret if available (confidential clients)
        if let Some(ref secret) = self.config.client_secret {
            params.push(("client_secret", secret.clone()));
        }

        // Add PKCE code verifier if used
        if let Some(ref pkce_state) = auth_state.pkce {
            params.push(("code_verifier", pkce_state.code_verifier.clone()));
        }

        debug!("Exchanging authorization code for token at: {}", self.config.token_url);

        // Make the token request
        let response = self
            .http_client
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| ClientError::OAuthError(format!("Token request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ClientError::OAuthError(format!(
                "Token endpoint returned {}: {}",
                status, body
            )));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| ClientError::OAuthError(format!("Failed to parse token response: {}", e)))?;

        // Calculate expiration time
        let expires_at = token_response.expires_in.and_then(|seconds| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .map(|d| d.as_secs() + seconds)
        });

        // Parse scope from response or use configured scopes
        let scope = token_response
            .scope
            .map(|s| s.split_whitespace().map(String::from).collect())
            .unwrap_or_else(|| self.config.scopes.clone());

        let client_token = ClientToken {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at,
            scope,
        };

        info!("Successfully exchanged authorization code for access token");

        // Store the token
        {
            let mut token = self.token.write().await;
            *token = Some(client_token.clone());
        }

        // Clear the auth state
        {
            let mut auth_state = self.auth_state.write().await;
            *auth_state = None;
        }

        Ok(client_token)
    }

    pub async fn get_token(&self) -> Option<ClientToken> {
        let token = self.token.read().await;
        token.clone()
    }

    pub async fn refresh_token(&self) -> Result<ClientToken> {
        let current_token = {
            let token = self.token.read().await;
            token.clone()
        };

        let current_token = current_token
            .ok_or_else(|| ClientError::OAuthError("No token found".to_string()))?;

        let refresh_token = current_token
            .refresh_token
            .ok_or_else(|| ClientError::OAuthError("No refresh token available".to_string()))?;

        // Build refresh token request
        let mut params = vec![
            ("grant_type", "refresh_token".to_string()),
            ("refresh_token", refresh_token),
            ("client_id", self.config.client_id.clone()),
        ];

        // Add client secret if available
        if let Some(ref secret) = self.config.client_secret {
            params.push(("client_secret", secret.clone()));
        }

        debug!("Refreshing access token at: {}", self.config.token_url);

        // Make the refresh request
        let response = self
            .http_client
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| ClientError::OAuthError(format!("Token refresh failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ClientError::OAuthError(format!(
                "Token refresh returned {}: {}",
                status, body
            )));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| ClientError::OAuthError(format!("Failed to parse refresh response: {}", e)))?;

        // Calculate expiration time
        let expires_at = token_response.expires_in.and_then(|seconds| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .map(|d| d.as_secs() + seconds)
        });

        // Parse scope from response or keep current scopes
        let scope = token_response
            .scope
            .map(|s| s.split_whitespace().map(String::from).collect())
            .unwrap_or(current_token.scope);

        let new_token = ClientToken {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at,
            scope,
        };

        info!("Successfully refreshed access token");

        // Update stored token
        {
            let mut token = self.token.write().await;
            *token = Some(new_token.clone());
        }

        Ok(new_token)
    }

    pub async fn is_token_valid(&self) -> bool {
        if let Some(token) = self.get_token().await {
            if let Some(expires_at) = token.expires_at {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                return now < expires_at;
            }
            return true; // If no expiration time, assume valid
        }
        false
    }

    pub async fn get_valid_token(&self) -> Result<String> {
        if self.is_token_valid().await {
            if let Some(token) = self.get_token().await {
                return Ok(token.access_token);
            }
        }

        // Try to refresh the token
        let refreshed = self.refresh_token().await?;
        Ok(refreshed.access_token)
    }

    pub async fn revoke_token(&self) -> Result<()> {
        let mut token = self.token.write().await;
        *token = None;
        Ok(())
    }

    pub fn parse_callback_url(&self, callback_url: &str) -> Result<(String, String)> {
        let url = url::Url::parse(callback_url)
            .map_err(|e| ClientError::OAuthError(format!("Invalid callback URL: {}", e)))?;

        let params: std::collections::HashMap<String, String> = url.query_pairs()
            .into_owned()
            .collect();

        let code = params.get("code")
            .ok_or_else(|| ClientError::OAuthError("Missing authorization code".to_string()))?
            .clone();

        let state = params.get("state")
            .ok_or_else(|| ClientError::OAuthError("Missing state parameter".to_string()))?
            .clone();

        Ok((code, state))
    }

    pub async fn set_token(&self, token: ClientToken) {
        let mut current_token = self.token.write().await;
        *current_token = Some(token);
    }

    pub fn get_authorization_header(&self, access_token: &str) -> String {
        format!("Bearer {}", access_token)
    }
}

/// Trait for providing OAuth tokens dynamically
#[async_trait::async_trait]
pub trait TokenProvider: Send + Sync {
    /// Get a valid access token, refreshing if necessary
    async fn get_token(&self) -> Result<String>;
}

/// Static token provider - just returns a fixed token string
pub struct StaticTokenProvider {
    token: String,
}

impl StaticTokenProvider {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

#[async_trait::async_trait]
impl TokenProvider for StaticTokenProvider {
    async fn get_token(&self) -> Result<String> {
        Ok(self.token.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> OAuthClientConfig {
        OAuthClientConfig {
            client_id: "test_client_id".to_string(),
            client_secret: Some("test_client_secret".to_string()),
            auth_url: "https://example.com/oauth/authorize".to_string(),
            token_url: "https://example.com/oauth/token".to_string(),
            redirect_url: "http://localhost:8080/callback".to_string(),
            scopes: vec!["read".to_string(), "write".to_string()],
            use_pkce: true,
        }
    }

    #[tokio::test]
    async fn test_oauth_client_creation() {
        let config = create_test_config();
        let client = OAuthClient::new(config);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_generate_auth_url() {
        let config = create_test_config();
        let client = OAuthClient::new(config).unwrap();

        let auth_url = client.generate_auth_url().await;
        assert!(auth_url.is_ok());
        assert!(auth_url.unwrap().starts_with("https://example.com/oauth/authorize"));
    }

    #[test]
    fn test_parse_callback_url() {
        let config = create_test_config();
        let client = OAuthClient::new(config).unwrap();

        let callback_url = "http://localhost:8080/callback?code=test_code&state=test_state";
        let result = client.parse_callback_url(callback_url);

        assert!(result.is_ok());
        let (code, state) = result.unwrap();
        assert_eq!(code, "test_code");
        assert_eq!(state, "test_state");
    }
}
