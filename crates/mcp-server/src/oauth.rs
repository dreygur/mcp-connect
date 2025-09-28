use crate::error::{Result, ServerError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_url: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
    pub scope: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AuthSession {
    pub user_id: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub state: String,
    pub code_challenge: String,
    pub expires_at: SystemTime,
}

pub struct OAuthManager {
    config: OAuthConfig,
    tokens: Arc<RwLock<HashMap<String, OAuthToken>>>, // user_id -> token
    sessions: Arc<RwLock<HashMap<String, AuthSession>>>, // auth_code -> session
    auth_codes: Arc<RwLock<HashMap<String, String>>>, // code -> user_id
}

impl OAuthManager {
    pub fn new(config: OAuthConfig) -> Result<Self> {
        Ok(Self {
            config,
            tokens: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            auth_codes: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn generate_auth_url(&self, user_id: &str) -> Result<String> {
        let state = format!("state_{}", uuid::Uuid::new_v4());
        let code_challenge = format!("challenge_{}", uuid::Uuid::new_v4());
        let auth_code = format!("code_{}", uuid::Uuid::new_v4());

        let session = AuthSession {
            user_id: user_id.to_string(),
            client_id: self.config.client_id.clone(),
            redirect_uri: self.config.redirect_url.clone(),
            state: state.clone(),
            code_challenge: code_challenge.clone(),
            expires_at: SystemTime::now() + std::time::Duration::from_secs(600), // 10 minutes
        };

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(auth_code.clone(), session);
        }

        let scopes = self.config.scopes.join(" ");
        let auth_url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
            self.config.auth_url,
            self.config.client_id,
            urlencoding::encode(&self.config.redirect_url),
            urlencoding::encode(&scopes),
            state,
            code_challenge
        );

        Ok(auth_url)
    }

    pub async fn exchange_code(
        &self,
        user_id: &str,
        code: &str,
        state: &str,
    ) -> Result<OAuthToken> {
        // Find session by state
        let session = {
            let sessions = self.sessions.read().await;
            sessions.values().find(|s| s.state == state && s.user_id == user_id).cloned()
        };

        let session = session.ok_or_else(|| ServerError::InvalidOAuthState("Invalid state or session not found".to_string()))?;

        // Check if session is expired
        if SystemTime::now() > session.expires_at {
            return Err(ServerError::InvalidOAuthState("Session expired".to_string()));
        }

        // Generate token
        let access_token = format!("mcp_access_{}", uuid::Uuid::new_v4());
        let refresh_token = format!("mcp_refresh_{}", uuid::Uuid::new_v4());
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() + 3600; // 1 hour

        let oauth_token = OAuthToken {
            access_token: access_token.clone(),
            refresh_token: Some(refresh_token),
            expires_at: Some(expires_at),
            scope: self.config.scopes.clone(),
        };

        // Store the token
        {
            let mut tokens = self.tokens.write().await;
            tokens.insert(user_id.to_string(), oauth_token.clone());
        }

        // Store auth code mapping
        {
            let mut auth_codes = self.auth_codes.write().await;
            auth_codes.insert(code.to_string(), user_id.to_string());
        }

        Ok(oauth_token)
    }

    pub async fn get_token(&self, user_id: &str) -> Option<OAuthToken> {
        let tokens = self.tokens.read().await;
        tokens.get(user_id).cloned()
    }

    pub async fn refresh_token(&self, user_id: &str) -> Result<OAuthToken> {
        let current_token = {
            let tokens = self.tokens.read().await;
            tokens.get(user_id).cloned()
        };

        let current_token = current_token
            .ok_or_else(|| ServerError::InvalidOAuthState("No token found for user".to_string()))?;

        if current_token.refresh_token.is_none() {
            return Err(ServerError::InvalidOAuthState("No refresh token available".to_string()));
        }

        // Generate new tokens
        let access_token = format!("mcp_access_{}", uuid::Uuid::new_v4());
        let refresh_token = format!("mcp_refresh_{}", uuid::Uuid::new_v4());
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() + 3600; // 1 hour

        let new_token = OAuthToken {
            access_token,
            refresh_token: Some(refresh_token),
            expires_at: Some(expires_at),
            scope: current_token.scope,
        };

        // Update stored token
        {
            let mut tokens = self.tokens.write().await;
            tokens.insert(user_id.to_string(), new_token.clone());
        }

        Ok(new_token)
    }

    pub async fn is_token_valid(&self, user_id: &str) -> bool {
        if let Some(token) = self.get_token(user_id).await {
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

    pub async fn validate_token(&self, access_token: &str) -> Option<String> {
        let tokens = self.tokens.read().await;
        for (user_id, token) in tokens.iter() {
            if token.access_token == access_token {
                if let Some(expires_at) = token.expires_at {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    if now < expires_at {
                        return Some(user_id.clone());
                    }
                } else {
                    return Some(user_id.clone());
                }
            }
        }
        None
    }

    pub async fn revoke_token(&self, user_id: &str) -> Result<()> {
        let mut tokens = self.tokens.write().await;
        tokens.remove(user_id);
        Ok(())
    }

    pub fn parse_callback_url(&self, callback_url: &str) -> Result<(String, String)> {
        let url = url::Url::parse(callback_url)
            .map_err(|e| ServerError::InvalidOAuthConfig(format!("Invalid callback URL: {}", e)))?;

        let params: HashMap<String, String> = url.query_pairs()
            .into_owned()
            .collect();

        let code = params.get("code")
            .ok_or_else(|| ServerError::InvalidOAuthState("Missing authorization code".to_string()))?
            .clone();

        let state = params.get("state")
            .ok_or_else(|| ServerError::InvalidOAuthState("Missing state parameter".to_string()))?
            .clone();

        Ok((code, state))
    }

    pub async fn cleanup_expired_sessions(&self) {
        let now = SystemTime::now();

        // Clean up expired sessions
        {
            let mut sessions = self.sessions.write().await;
            sessions.retain(|_, session| now < session.expires_at);
        }

        // Clean up expired tokens
        {
            let mut tokens = self.tokens.write().await;
            tokens.retain(|_, token| {
                if let Some(expires_at) = token.expires_at {
                    let current_time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    current_time < expires_at
                } else {
                    true // Keep tokens without expiration
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> OAuthConfig {
        OAuthConfig {
            client_id: "test_client_id".to_string(),
            client_secret: Some("test_client_secret".to_string()),
            auth_url: "https://example.com/oauth/authorize".to_string(),
            token_url: "https://example.com/oauth/token".to_string(),
            redirect_url: "http://localhost:8080/callback".to_string(),
            scopes: vec!["read".to_string(), "write".to_string()],
        }
    }

    #[tokio::test]
    async fn test_oauth_manager_creation() {
        let config = create_test_config();
        let manager = OAuthManager::new(config);
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_generate_auth_url() {
        let config = create_test_config();
        let manager = OAuthManager::new(config).unwrap();

        let auth_url = manager.generate_auth_url("test_user").await;
        assert!(auth_url.is_ok());
        assert!(auth_url.unwrap().starts_with("https://example.com/oauth/authorize"));
    }

    #[test]
    fn test_parse_callback_url() {
        let config = create_test_config();
        let manager = OAuthManager::new(config).unwrap();

        let callback_url = "http://localhost:8080/callback?code=test_code&state=test_state";
        let result = manager.parse_callback_url(callback_url);

        assert!(result.is_ok());
        let (code, state) = result.unwrap();
        assert_eq!(code, "test_code");
        assert_eq!(state, "test_state");
    }
}
