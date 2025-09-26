use crate::{OAuthError, Result};
use crate::types::{StoredToken, TokenResponse, OAuthServerMetadata};
use chrono::{Duration, Utc};
use reqwest::Client;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, error};

/// Token manager for OAuth 2.0 access and refresh tokens
///
/// This handles token exchange, refresh, storage, and validation for OAuth flows.
pub struct TokenManager {
    http_client: Client,
    storage_dir: PathBuf,
}

impl TokenManager {
    /// Create a new token manager
    ///
    /// # Arguments
    /// * `storage_dir` - Directory to store token files (typically ~/.mcp-auth)
    pub fn new<P: AsRef<Path>>(storage_dir: P) -> Result<Self> {
        Ok(Self {
            http_client: Client::new(),
            storage_dir: storage_dir.as_ref().to_path_buf(),
        })
    }

    /// Exchange authorization code for access token
    ///
    /// # Arguments
    /// * `server_metadata` - OAuth server metadata with token endpoint
    /// * `client_id` - OAuth client ID
    /// * `client_secret` - Optional client secret (for confidential clients)
    /// * `authorization_code` - Authorization code from callback
    /// * `redirect_uri` - Original redirect URI used in authorization
    /// * `code_verifier` - PKCE code verifier
    /// * `server_url` - MCP server URL for token storage key
    ///
    /// # Returns
    /// Stored token with metadata
    pub async fn exchange_code_for_token(
        &self,
        server_metadata: &OAuthServerMetadata,
        client_id: &str,
        client_secret: Option<&str>,
        authorization_code: &str,
        redirect_uri: &str,
        code_verifier: &str,
        server_url: &str,
    ) -> Result<StoredToken> {
        info!("Exchanging authorization code for access token");

        let mut token_request = HashMap::new();
        token_request.insert("grant_type", "authorization_code");
        token_request.insert("client_id", client_id);
        token_request.insert("code", authorization_code);
        token_request.insert("redirect_uri", redirect_uri);
        token_request.insert("code_verifier", code_verifier);

        if let Some(secret) = client_secret {
            token_request.insert("client_secret", secret);
        }

        debug!("Token exchange request: {:?}", token_request);

        let response = self.http_client
            .post(&server_metadata.token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&token_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            error!("Token exchange failed: {} - {}", status, error_body);
            return Err(OAuthError::TokenExchange(
                format!("Token exchange failed with status {}: {}", status, error_body)
            ));
        }

        let token_response: TokenResponse = response.json().await?;
        info!("Successfully exchanged authorization code for access token");

        // Convert to stored token with metadata
        let stored_token = self.create_stored_token(token_response, server_url)?;

        // Save to storage
        self.save_token(&stored_token).await?;

        Ok(stored_token)
    }

    /// Refresh an access token using a refresh token
    ///
    /// # Arguments
    /// * `server_metadata` - OAuth server metadata with token endpoint
    /// * `client_id` - OAuth client ID
    /// * `client_secret` - Optional client secret
    /// * `stored_token` - Current stored token with refresh token
    ///
    /// # Returns
    /// New stored token with updated access token
    pub async fn refresh_token(
        &self,
        server_metadata: &OAuthServerMetadata,
        client_id: &str,
        client_secret: Option<&str>,
        stored_token: &StoredToken,
    ) -> Result<StoredToken> {
        let refresh_token = stored_token.refresh_token
            .as_ref()
            .ok_or_else(|| OAuthError::TokenRefresh("No refresh token available".to_string()))?;

        info!("Refreshing access token using refresh token");

        let mut token_request = HashMap::new();
        token_request.insert("grant_type", "refresh_token");
        token_request.insert("client_id", client_id);
        token_request.insert("refresh_token", refresh_token);

        if let Some(secret) = client_secret {
            token_request.insert("client_secret", secret);
        }

        if let Some(ref scope) = stored_token.scope {
            token_request.insert("scope", scope);
        }

        debug!("Token refresh request for client: {}", client_id);

        let response = self.http_client
            .post(&server_metadata.token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&token_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            error!("Token refresh failed: {} - {}", status, error_body);
            return Err(OAuthError::TokenRefresh(
                format!("Token refresh failed with status {}: {}", status, error_body)
            ));
        }

        let token_response: TokenResponse = response.json().await?;
        info!("Successfully refreshed access token");

        // Create updated stored token, preserving original refresh token if not provided
        let mut new_stored_token = self.create_stored_token(token_response, &stored_token.server_url)?;

        if new_stored_token.refresh_token.is_none() && stored_token.refresh_token.is_some() {
            new_stored_token.refresh_token = stored_token.refresh_token.clone();
        }

        // Save updated token
        self.save_token(&new_stored_token).await?;

        Ok(new_stored_token)
    }

    /// Load stored token for a server URL
    ///
    /// # Arguments
    /// * `server_url` - MCP server URL
    ///
    /// # Returns
    /// Stored token if found and valid
    pub async fn load_token(&self, server_url: &str) -> Result<Option<StoredToken>> {
        let token_file = self.get_token_file_path(server_url);

        if !token_file.exists() {
            debug!("No stored token found for server: {}", server_url);
            return Ok(None);
        }

        debug!("Loading stored token from: {:?}", token_file);

        let token_data = fs::read_to_string(&token_file).await?;
        let stored_token: StoredToken = serde_json::from_str(&token_data)?;

        debug!("Loaded token for server: {}", server_url);
        Ok(Some(stored_token))
    }

    /// Save token to storage
    ///
    /// # Arguments
    /// * `stored_token` - Token to save
    pub async fn save_token(&self, stored_token: &StoredToken) -> Result<()> {
        // Ensure storage directory exists
        fs::create_dir_all(&self.storage_dir).await?;

        let token_file = self.get_token_file_path(&stored_token.server_url);
        let token_data = serde_json::to_string_pretty(stored_token)?;

        debug!("Saving token to: {:?}", token_file);
        fs::write(&token_file, token_data).await?;

        info!("Token saved successfully for server: {}", stored_token.server_url);
        Ok(())
    }

    /// Delete stored token for a server
    ///
    /// # Arguments
    /// * `server_url` - Server URL to delete token for
    pub async fn delete_token(&self, server_url: &str) -> Result<()> {
        let token_file = self.get_token_file_path(server_url);

        if token_file.exists() {
            fs::remove_file(&token_file).await?;
            info!("Deleted stored token for server: {}", server_url);
        }

        Ok(())
    }

    /// Check if a token is expired or will expire soon
    ///
    /// # Arguments
    /// * `stored_token` - Token to check
    /// * `buffer_seconds` - Consider token expired if it expires within this buffer
    ///
    /// # Returns
    /// True if token is expired or will expire soon
    pub fn is_token_expired(&self, stored_token: &StoredToken, buffer_seconds: u64) -> bool {
        match stored_token.expires_at {
            Some(expires_at) => {
                let now = Utc::now();
                let buffer = Duration::seconds(buffer_seconds as i64);
                expires_at <= now + buffer
            }
            None => false, // No expiration time means token doesn't expire
        }
    }

    /// Get or refresh a valid access token
    ///
    /// This is the main method to use - it handles loading, validation, and refresh automatically.
    ///
    /// # Arguments
    /// * `server_metadata` - OAuth server metadata
    /// * `client_id` - OAuth client ID
    /// * `client_secret` - Optional client secret
    /// * `server_url` - MCP server URL
    ///
    /// # Returns
    /// Valid access token string
    pub async fn get_valid_token(
        &self,
        server_metadata: &OAuthServerMetadata,
        client_id: &str,
        client_secret: Option<&str>,
        server_url: &str,
    ) -> Result<String> {
        // Load existing token
        let mut stored_token = match self.load_token(server_url).await? {
            Some(token) => token,
            None => return Err(OAuthError::TokenStorage("No stored token found".to_string())),
        };

        // Check if token is expired or will expire soon (60 second buffer)
        if self.is_token_expired(&stored_token, 60) {
            info!("Access token is expired or will expire soon, refreshing...");

            stored_token = self.refresh_token(
                server_metadata,
                client_id,
                client_secret,
                &stored_token,
            ).await?;
        }

        Ok(stored_token.access_token)
    }

    /// Create a StoredToken from a TokenResponse
    fn create_stored_token(&self, token_response: TokenResponse, server_url: &str) -> Result<StoredToken> {
        let now = Utc::now();
        let expires_at = token_response.expires_in
            .map(|expires_in| now + Duration::seconds(expires_in as i64));

        Ok(StoredToken {
            access_token: token_response.access_token,
            token_type: token_response.token_type,
            refresh_token: token_response.refresh_token,
            scope: token_response.scope,
            expires_at,
            server_url: server_url.to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    /// Get the file path for storing a token for a given server URL
    fn get_token_file_path(&self, server_url: &str) -> PathBuf {
        // Create a safe filename from the server URL
        let safe_filename = server_url
            .replace("://", "_")
            .replace('/', "_")
            .replace(':', "_")
            .replace('?', "_")
            .replace('&', "_")
            + ".json";

        self.storage_dir.join(safe_filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_get_token_file_path() {
        let temp_dir = tempdir().unwrap();
        let token_manager = TokenManager::new(temp_dir.path()).unwrap();

        let path = token_manager.get_token_file_path("https://api.example.com/oauth");
        assert!(path.to_string_lossy().contains("https_api.example.com_oauth.json"));
    }

    #[test]
    fn test_is_token_expired() {
        let temp_dir = tempdir().unwrap();
        let token_manager = TokenManager::new(temp_dir.path()).unwrap();

        // Token that expires in 30 seconds
        let expires_soon = StoredToken {
            access_token: "token".to_string(),
            token_type: "Bearer".to_string(),
            refresh_token: None,
            scope: None,
            expires_at: Some(Utc::now() + Duration::seconds(30)),
            server_url: "https://example.com".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Should be considered expired with 60 second buffer
        assert!(token_manager.is_token_expired(&expires_soon, 60));

        // Should not be expired with 10 second buffer
        assert!(!token_manager.is_token_expired(&expires_soon, 10));

        // Token with no expiration should never be considered expired
        let no_expiry = StoredToken {
            expires_at: None,
            ..expires_soon
        };

        assert!(!token_manager.is_token_expired(&no_expiry, 3600));
    }

    #[tokio::test]
    async fn test_create_stored_token() {
        let temp_dir = tempdir().unwrap();
        let token_manager = TokenManager::new(temp_dir.path()).unwrap();

        let token_response = TokenResponse {
            access_token: "access123".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: Some(3600),
            refresh_token: Some("refresh123".to_string()),
            scope: Some("mcp".to_string()),
        };

        let stored_token = token_manager.create_stored_token(token_response, "https://example.com").unwrap();

        assert_eq!(stored_token.access_token, "access123");
        assert_eq!(stored_token.token_type, "Bearer");
        assert_eq!(stored_token.refresh_token, Some("refresh123".to_string()));
        assert_eq!(stored_token.scope, Some("mcp".to_string()));
        assert_eq!(stored_token.server_url, "https://example.com");
        assert!(stored_token.expires_at.is_some());
    }
}
