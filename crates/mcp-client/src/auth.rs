use crate::error::{Result, ClientError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthClientConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_url: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
    pub scope: Vec<String>,
}

pub struct OAuthClient {
    config: OAuthClientConfig,
    token: Arc<RwLock<Option<ClientToken>>>,
    auth_state: Arc<RwLock<Option<String>>>,
}

impl OAuthClient {
    pub fn new(config: OAuthClientConfig) -> Result<Self> {
        Ok(Self {
            config,
            token: Arc::new(RwLock::new(None)),
            auth_state: Arc::new(RwLock::new(None)),
        })
    }

    pub async fn generate_auth_url(&self) -> Result<String> {
        let state = format!("state_{}", uuid::Uuid::new_v4());

        // Store the state for later verification
        {
            let mut auth_state = self.auth_state.write().await;
            *auth_state = Some(state.clone());
        }

        let scopes = self.config.scopes.join(" ");
        let auth_url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
            self.config.auth_url,
            self.config.client_id,
            urlencoding::encode(&self.config.redirect_url),
            urlencoding::encode(&scopes),
            state
        );

        Ok(auth_url)
    }

    pub async fn exchange_code(&self, _code: &str, state: &str) -> Result<ClientToken> {
        // Verify state
        let expected_state = {
            let auth_state = self.auth_state.read().await;
            auth_state.clone()
        };

        let expected_state = expected_state
            .ok_or_else(|| ClientError::OAuthError("No auth state found".to_string()))?;

        if expected_state != state {
            return Err(ClientError::OAuthError("State mismatch".to_string()));
        }

        // In a real implementation, this would make an HTTP request to the token endpoint
        // For now, we'll create a mock token
        let access_token = format!("client_access_{}", uuid::Uuid::new_v4());
        let refresh_token = format!("client_refresh_{}", uuid::Uuid::new_v4());
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() + 3600; // 1 hour

        let client_token = ClientToken {
            access_token,
            refresh_token: Some(refresh_token),
            expires_at: Some(expires_at),
            scope: self.config.scopes.clone(),
        };

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

        if current_token.refresh_token.is_none() {
            return Err(ClientError::OAuthError("No refresh token available".to_string()));
        }

        // Generate new tokens (in a real implementation, this would call the token endpoint)
        let access_token = format!("client_access_{}", uuid::Uuid::new_v4());
        let refresh_token = format!("client_refresh_{}", uuid::Uuid::new_v4());
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() + 3600; // 1 hour

        let new_token = ClientToken {
            access_token,
            refresh_token: Some(refresh_token),
            expires_at: Some(expires_at),
            scope: current_token.scope,
        };

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
                    .unwrap()
                    .as_secs();
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
