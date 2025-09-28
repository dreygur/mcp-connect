use crate::error::{ProxyError, Result};
use mcp_client::{OAuthClient, OAuthClientConfig, ClientToken};
use mcp_server::{OAuthManager, OAuthConfig};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct AuthProxyConfig {
    pub server_oauth: Option<OAuthConfig>,
    pub client_oauth: Option<OAuthClientConfig>,
    pub require_auth: bool,
    pub token_validation_endpoint: Option<String>,
}

pub struct AuthenticatedProxy {
    config: AuthProxyConfig,
    server_oauth: Option<Arc<OAuthManager>>,
    client_oauth: Option<Arc<OAuthClient>>,
    authenticated_sessions: Arc<RwLock<HashMap<String, ClientToken>>>,
}

impl AuthenticatedProxy {
    pub fn new(config: AuthProxyConfig) -> Result<Self> {
        let server_oauth = if let Some(server_config) = config.server_oauth.clone() {
            Some(Arc::new(
                OAuthManager::new(server_config)
                    .map_err(|e| ProxyError::Auth(format!("Failed to create OAuth manager: {}", e)))?
            ))
        } else {
            None
        };

        let client_oauth = if let Some(client_config) = config.client_oauth.clone() {
            Some(Arc::new(
                OAuthClient::new(client_config)
                    .map_err(|e| ProxyError::Auth(format!("Failed to create OAuth client: {}", e)))?
            ))
        } else {
            None
        };

        Ok(Self {
            config,
            server_oauth,
            client_oauth,
            authenticated_sessions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn handle_auth_request(&self, method: &str, params: Value, session_id: &str) -> Result<Value> {
        match method {
            "auth/login" => self.handle_login(params, session_id).await,
            "auth/logout" => self.handle_logout(session_id).await,
            "auth/refresh" => self.handle_refresh_token(session_id).await,
            "auth/status" => self.handle_auth_status(session_id).await,
            _ => Err(ProxyError::Protocol(format!("Unknown auth method: {}", method))),
        }
    }

    async fn handle_login(&self, params: Value, session_id: &str) -> Result<Value> {
        let auth_type = params.get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("oauth");

        match auth_type {
            "oauth" => self.handle_oauth_login(params, session_id).await,
            "token" => self.handle_token_login(params, session_id).await,
            _ => Err(ProxyError::Auth(format!("Unsupported auth type: {}", auth_type))),
        }
    }

    async fn handle_oauth_login(&self, params: Value, session_id: &str) -> Result<Value> {
        let client_oauth = self.client_oauth.as_ref()
            .ok_or_else(|| ProxyError::Auth("OAuth client not configured".to_string()))?;

        if let Some(callback_url) = params.get("callback_url").and_then(|u| u.as_str()) {
            // Handle OAuth callback
            let (code, state) = client_oauth.parse_callback_url(callback_url)
                .map_err(|e| ProxyError::Auth(format!("Failed to parse callback: {}", e)))?;

            let token = client_oauth.exchange_code(&code, &state).await
                .map_err(|e| ProxyError::Auth(format!("Failed to exchange code: {}", e)))?;

            // Store the token for this session
            {
                let mut sessions = self.authenticated_sessions.write().await;
                sessions.insert(session_id.to_string(), token);
            }

            info!("OAuth login successful for session: {}", session_id);
            Ok(serde_json::json!({
                "status": "success",
                "message": "Authentication successful"
            }))
        } else {
            // Generate auth URL
            let auth_url = client_oauth.generate_auth_url().await
                .map_err(|e| ProxyError::Auth(format!("Failed to generate auth URL: {}", e)))?;

            Ok(serde_json::json!({
                "auth_url": auth_url,
                "message": "Please visit the auth_url to complete authentication"
            }))
        }
    }

    async fn handle_token_login(&self, params: Value, session_id: &str) -> Result<Value> {
        let access_token = params.get("access_token")
            .and_then(|t| t.as_str())
            .ok_or_else(|| ProxyError::Auth("Missing access_token".to_string()))?;

        let refresh_token = params.get("refresh_token")
            .and_then(|t| t.as_str());

        let expires_in = params.get("expires_in")
            .and_then(|e| e.as_u64());

        let expires_at = expires_in.map(|seconds| {
            std::time::SystemTime::now() + std::time::Duration::from_secs(seconds)
        });

        let token = ClientToken {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.map(|rt| rt.to_string()),
            expires_at: expires_in.map(|seconds| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() + seconds
            }),
            scope: vec![], // Default empty scope
        };

        // Validate token if endpoint is configured
        if let Some(validation_endpoint) = &self.config.token_validation_endpoint {
            self.validate_token(&token.access_token, validation_endpoint).await?;
        }

        // Store the token for this session
        {
            let mut sessions = self.authenticated_sessions.write().await;
            sessions.insert(session_id.to_string(), token);
        }

        info!("Token login successful for session: {}", session_id);
        Ok(serde_json::json!({
            "status": "success",
            "message": "Authentication successful"
        }))
    }

    async fn handle_logout(&self, session_id: &str) -> Result<Value> {
        let mut sessions = self.authenticated_sessions.write().await;
        if sessions.remove(session_id).is_some() {
            info!("Logout successful for session: {}", session_id);
            Ok(serde_json::json!({
                "status": "success",
                "message": "Logout successful"
            }))
        } else {
            warn!("Logout attempted for non-authenticated session: {}", session_id);
            Ok(serde_json::json!({
                "status": "error",
                "message": "Session not authenticated"
            }))
        }
    }

    async fn handle_refresh_token(&self, session_id: &str) -> Result<Value> {
        let client_oauth = self.client_oauth.as_ref()
            .ok_or_else(|| ProxyError::Auth("OAuth client not configured".to_string()))?;

        let current_token = {
            let sessions = self.authenticated_sessions.read().await;
            sessions.get(session_id).cloned()
        };

        let current_token = current_token
            .ok_or_else(|| ProxyError::Auth("No authenticated session found".to_string()))?;

        if current_token.refresh_token.is_none() {
            return Err(ProxyError::Auth("No refresh token available".to_string()));
        }

        // Set the current token in the OAuth client and refresh
        client_oauth.set_token(current_token).await;
        let new_token = client_oauth.refresh_token().await
            .map_err(|e| ProxyError::Auth(format!("Failed to refresh token: {}", e)))?;

        // Update stored token
        {
            let mut sessions = self.authenticated_sessions.write().await;
            sessions.insert(session_id.to_string(), new_token);
        }

        info!("Token refresh successful for session: {}", session_id);
        Ok(serde_json::json!({
            "status": "success",
            "message": "Token refreshed successfully"
        }))
    }

    async fn handle_auth_status(&self, session_id: &str) -> Result<Value> {
        let sessions = self.authenticated_sessions.read().await;
        if let Some(token) = sessions.get(session_id) {
            let is_valid = if let Some(expires_at) = token.expires_at {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                now < expires_at
            } else {
                true
            };

            Ok(serde_json::json!({
                "authenticated": true,
                "token_valid": is_valid,
                "has_refresh_token": token.refresh_token.is_some()
            }))
        } else {
            Ok(serde_json::json!({
                "authenticated": false,
                "token_valid": false,
                "has_refresh_token": false
            }))
        }
    }

    pub async fn is_authenticated(&self, session_id: &str) -> bool {
        let sessions = self.authenticated_sessions.read().await;
        if let Some(token) = sessions.get(session_id) {
            if let Some(expires_at) = token.expires_at {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                return now < expires_at;
            }
            return true;
        }
        false
    }

    pub async fn get_authorization_header(&self, session_id: &str) -> Option<String> {
        let sessions = self.authenticated_sessions.read().await;
        sessions.get(session_id).map(|token| {
            format!("Bearer {}", token.access_token)
        })
    }

    pub async fn authorize_request(&self, session_id: &str, _method: &str) -> Result<()> {
        if !self.config.require_auth {
            return Ok(());
        }

        if !self.is_authenticated(session_id).await {
            return Err(ProxyError::Auth("Authentication required".to_string()));
        }

        Ok(())
    }

    async fn validate_token(&self, token: &str, endpoint: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let response = client
            .get(endpoint)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| ProxyError::Auth(format!("Token validation request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ProxyError::Auth("Token validation failed".to_string()));
        }

        debug!("Token validation successful");
        Ok(())
    }

    pub async fn cleanup_expired_sessions(&self) {
        let mut sessions = self.authenticated_sessions.write().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        sessions.retain(|session_id, token| {
            if let Some(expires_at) = token.expires_at {
                let is_valid = now < expires_at;
                if !is_valid {
                    debug!("Removing expired session: {}", session_id);
                }
                is_valid
            } else {
                true // Keep tokens without expiration
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> AuthProxyConfig {
        AuthProxyConfig {
            server_oauth: None,
            client_oauth: None,
            require_auth: false,
            token_validation_endpoint: None,
        }
    }

    #[tokio::test]
    async fn test_auth_proxy_creation() {
        let config = create_test_config();
        let proxy = AuthenticatedProxy::new(config);
        assert!(proxy.is_ok());
    }

    #[tokio::test]
    async fn test_auth_status_unauthenticated() {
        let config = create_test_config();
        let proxy = AuthenticatedProxy::new(config).unwrap();

        let result = proxy.handle_auth_status("test_session").await;
        assert!(result.is_ok());

        let status = result.unwrap();
        assert_eq!(status["authenticated"], false);
    }
}
