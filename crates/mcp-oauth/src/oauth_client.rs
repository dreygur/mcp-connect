use crate::{OAuthError, Result};
use crate::browser::BrowserLauncher;
use crate::callback_server::{CallbackServer, find_available_port};
use crate::client_registration::ClientRegistration;
use crate::coordination::{CoordinationManager, hash_server_url};
use crate::pkce::generate_pkce_challenge;
use crate::token_manager::TokenManager;
use crate::types::*;
use std::path::PathBuf;
use tokio::time::Duration;
use tracing::{debug, info};
use uuid::Uuid;

/// Main OAuth 2.0 client for MCP Remote
///
/// This is the primary interface for OAuth authentication flows.
/// It coordinates all the OAuth components to provide a complete solution.
pub struct OAuthClient {
    config: OAuthConfig,
    token_manager: TokenManager,
    coordination_manager: CoordinationManager,
}

impl OAuthClient {
    /// Create a new OAuth client
    ///
    /// # Arguments
    /// * `server_url` - MCP server URL
    /// * `auth_dir` - Directory for storing authentication data (typically ~/.mcp-auth)
    pub fn new(server_url: String, auth_dir: PathBuf) -> Result<Self> {
        let config = OAuthConfig {
            server_url: server_url.clone(),
            ..Default::default()
        };

        let token_manager = TokenManager::new(auth_dir.clone())?;
        let server_url_hash = hash_server_url(&server_url);
        let coordination_manager = CoordinationManager::new(auth_dir, server_url_hash);

        Ok(Self {
            config,
            token_manager,
            coordination_manager,
        })
    }

    /// Set static OAuth client information
    ///
    /// Use this when you have pre-registered OAuth client credentials.
    pub fn with_static_client_info(mut self, client_id: String, client_secret: Option<String>) -> Self {
        self.config.static_client_info = Some(StaticClientInfo {
            client_id,
            client_secret,
        });
        self
    }

    /// Set OAuth server metadata
    ///
    /// Use this when you have pre-discovered server metadata.
    pub fn with_server_metadata(mut self, metadata: OAuthServerMetadata) -> Self {
        self.config.server_metadata = Some(metadata);
        self
    }

    /// Set callback port preference
    ///
    /// # Arguments
    /// * `port` - Preferred port for OAuth callback server (0 for auto-select)
    pub fn with_callback_port(mut self, port: u16) -> Self {
        self.config.callback_port = Some(port);
        self
    }

    /// Set callback host preference
    ///
    /// # Arguments
    /// * `host` - Host for OAuth callback URL (default: localhost)
    pub fn with_callback_host(mut self, host: String) -> Self {
        self.config.callback_host = host;
        self
    }

    /// Set authentication timeout
    ///
    /// # Arguments
    /// * `timeout_secs` - Maximum seconds to wait for user authorization
    pub fn with_auth_timeout(mut self, timeout_secs: u64) -> Self {
        self.config.auth_timeout_secs = timeout_secs;
        self
    }

    /// Set OAuth scope
    ///
    /// # Arguments
    /// * `scope` - OAuth scope to request
    pub fn with_scope(mut self, scope: String) -> Self {
        self.config.scope = Some(scope);
        self
    }

    /// Get a valid access token for the MCP server
    ///
    /// This is the main method to call - it handles the complete OAuth flow:
    /// 1. Tries to load existing valid token
    /// 2. Refreshes token if needed and possible
    /// 3. Initiates new OAuth flow if no valid token exists
    ///
    /// # Returns
    /// Valid access token string
    pub async fn get_access_token(&mut self) -> Result<String> {
        info!("Getting access token for server: {}", self.config.server_url);

        // First, try to get server metadata if we don't have it
        if self.config.server_metadata.is_none() {
            info!("Discovering OAuth server metadata...");
            self.config.server_metadata = Some(
                ClientRegistration::discover_server_metadata(&self.config.server_url).await?
            );
        }

        let server_metadata = self.config.server_metadata.as_ref().unwrap();

        // Try to get client info (static or dynamic registration)
        let (client_id, client_secret) = self.get_or_register_client().await?;

        // Try to load and validate existing token
        match self.token_manager.get_valid_token(
            server_metadata,
            &client_id,
            client_secret.as_deref(),
            &self.config.server_url,
        ).await {
            Ok(token) => {
                info!("Using existing valid access token");
                return Ok(token);
            }
            Err(e) => {
                debug!("Could not get valid existing token: {}", e);
                info!("Starting new OAuth authorization flow...");
            }
        }

        // Start new OAuth flow
        self.start_oauth_flow(&client_id, client_secret.as_deref()).await
    }

    /// Start a new OAuth authorization flow
    async fn start_oauth_flow(&self, client_id: &str, client_secret: Option<&str>) -> Result<String> {
        let server_metadata = self.config.server_metadata.as_ref().unwrap();

        // Check for existing instances before starting new OAuth flow
        if let Some(lock_data) = self.coordination_manager.check_lockfile().await? {
            info!("Another instance is handling authentication on port {} (pid: {})",
                  lock_data.port, lock_data.pid);

            // Try to wait for the other instance to complete authentication
            if self.coordination_manager.wait_for_authentication(lock_data.port).await? {
                info!("Authentication completed by another instance");
                // Try to load the token that should now be available
                match self.token_manager.get_valid_token(
                    server_metadata,
                    client_id,
                    client_secret,
                    &self.config.server_url,
                ).await {
                    Ok(token) => return Ok(token),
                    Err(e) => {
                        debug!("Failed to load token after coordination: {}", e);
                        info!("Proceeding with our own auth flow");
                    }
                }
            }
        }

        // Find available port for callback server
        let callback_port = find_available_port(self.config.callback_port.unwrap_or(0))?;

        // Create callback server
        let callback_server = CallbackServer::new(callback_port)?;
        let redirect_uri = callback_server.callback_url(&self.config.callback_host);

        info!("Starting OAuth callback server on port: {}", callback_port);

        // Create lock file to coordinate with other instances
        self.coordination_manager.create_lockfile(callback_port).await?;

        // Generate PKCE challenge
        let pkce_challenge = generate_pkce_challenge()?;

        // Generate random state for security
        let state = Uuid::new_v4().to_string();

        // Build authorization URL
        let auth_url = self.build_authorization_url(
            server_metadata,
            client_id,
            &redirect_uri,
            &state,
            &pkce_challenge,
        )?;

        info!("Opening browser for OAuth authorization...");

        // Launch browser (this will print URL as fallback if browser launch fails)
        BrowserLauncher::launch(&auth_url).await?;

        // Wait for authorization callback
        let timeout_duration = Duration::from_secs(self.config.auth_timeout_secs);
        let auth_response = callback_server.wait_for_callback(timeout_duration).await?;

        // Verify state parameter
        if auth_response.state != state {
            return Err(OAuthError::PkceVerification(
                "State parameter mismatch - possible CSRF attack".to_string()
            ));
        }

        info!("Authorization successful, exchanging code for tokens...");

        // Exchange authorization code for tokens
        let stored_token = self.token_manager.exchange_code_for_token(
            server_metadata,
            client_id,
            client_secret,
            &auth_response.code,
            &redirect_uri,
            &pkce_challenge.code_verifier,
            &self.config.server_url,
        ).await?;

        info!("OAuth flow completed successfully!");

        // Clean up coordination lock file
        if let Err(e) = self.coordination_manager.delete_lockfile().await {
            debug!("Failed to clean up lock file: {}", e);
        }

        Ok(stored_token.access_token)
    }

    /// Get client info (either static or through dynamic registration)
    async fn get_or_register_client(&self) -> Result<(String, Option<String>)> {
        // Use static client info if provided
        if let Some(ref static_info) = self.config.static_client_info {
            info!("Using static OAuth client credentials");
            return Ok((static_info.client_id.clone(), static_info.client_secret.clone()));
        }

        // Use dynamic client registration
        let server_metadata = self.config.server_metadata.as_ref().unwrap();

        if server_metadata.registration_endpoint.is_none() {
            return Err(OAuthError::InvalidConfiguration(
                "No static client info provided and server does not support dynamic registration".to_string()
            ));
        }

        info!("Using dynamic client registration");

        let client_registration = ClientRegistration::new(server_metadata.clone());

        // For callback URL, we need to determine the port first
        let callback_port = find_available_port(self.config.callback_port.unwrap_or(0))?;
        let redirect_uri = format!("http://{}:{}/callback", self.config.callback_host, callback_port);

        let registration_response = client_registration.register_client(
            &redirect_uri,
            Some("MCP Remote"),
        ).await?;

        Ok((
            registration_response.client_id,
            registration_response.client_secret,
        ))
    }

    /// Build OAuth authorization URL
    fn build_authorization_url(
        &self,
        server_metadata: &OAuthServerMetadata,
        client_id: &str,
        redirect_uri: &str,
        state: &str,
        pkce_challenge: &PkceChallenge,
    ) -> Result<String> {
        let mut auth_url = url::Url::parse(&server_metadata.authorization_endpoint)?;

        {
            let mut query = auth_url.query_pairs_mut();
            query.append_pair("response_type", "code");
            query.append_pair("client_id", client_id);
            query.append_pair("redirect_uri", redirect_uri);
            query.append_pair("state", state);
            query.append_pair("code_challenge", &pkce_challenge.code_challenge);
            query.append_pair("code_challenge_method", &pkce_challenge.code_challenge_method);

            if let Some(ref scope) = self.config.scope {
                query.append_pair("scope", scope);
            }
        }

        debug!("Authorization URL: {}", auth_url);
        Ok(auth_url.to_string())
    }

    /// Clear stored tokens for this server
    ///
    /// This forces a new OAuth flow on the next token request.
    pub async fn clear_tokens(&self) -> Result<()> {
        self.token_manager.delete_token(&self.config.server_url).await
    }

    /// Check if we have a stored token for this server
    pub async fn has_stored_token(&self) -> bool {
        self.token_manager.load_token(&self.config.server_url)
            .await
            .map(|token| token.is_some())
            .unwrap_or(false)
    }

    /// Get the server URL this client is configured for
    pub fn server_url(&self) -> &str {
        &self.config.server_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_oauth_client_creation() {
        let temp_dir = tempdir().unwrap();
        let client = OAuthClient::new(
            "https://example.com/oauth".to_string(),
            temp_dir.path().to_path_buf(),
        ).unwrap();

        assert_eq!(client.server_url(), "https://example.com/oauth");
        assert!(!client.has_stored_token().await);
    }

    #[test]
    fn test_oauth_client_builder_pattern() {
        let temp_dir = tempdir().unwrap();
        let client = OAuthClient::new(
            "https://example.com".to_string(),
            temp_dir.path().to_path_buf(),
        ).unwrap()
            .with_callback_port(8080)
            .with_auth_timeout(600)
            .with_scope("mcp read write".to_string())
            .with_static_client_info("test_client".to_string(), Some("test_secret".to_string()));

        assert_eq!(client.config.callback_port, Some(8080));
        assert_eq!(client.config.auth_timeout_secs, 600);
        assert_eq!(client.config.scope, Some("mcp read write".to_string()));
        assert!(client.config.static_client_info.is_some());
    }

    #[tokio::test]
    async fn test_clear_tokens() {
        let temp_dir = tempdir().unwrap();
        let client = OAuthClient::new(
            "https://example.com".to_string(),
            temp_dir.path().to_path_buf(),
        ).unwrap();

        // Should not error even if no tokens exist
        assert!(client.clear_tokens().await.is_ok());
    }
}
