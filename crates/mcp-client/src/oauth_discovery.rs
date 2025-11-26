//! OAuth 2.0 Auto-Discovery for MCP servers.
//!
//! Implements RFC 8414 OAuth 2.0 Authorization Server Metadata discovery
//! for MCP servers that require OAuth authentication.

use crate::auth::{ClientToken, OAuthClient, OAuthClientConfig};
use crate::error::{ClientError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tracing::{debug, info, warn};
use url::Url;

/// OAuth 2.0 Authorization Server Metadata (RFC 8414)
#[derive(Debug, Clone, Deserialize)]
pub struct OAuthMetadata {
    /// Authorization endpoint URL
    pub authorization_endpoint: String,

    /// Token endpoint URL
    pub token_endpoint: String,

    /// Token revocation endpoint (optional)
    #[serde(default)]
    pub revocation_endpoint: Option<String>,

    /// Registration endpoint for dynamic client registration (optional)
    #[serde(default)]
    pub registration_endpoint: Option<String>,

    /// Supported response types
    #[serde(default)]
    pub response_types_supported: Vec<String>,

    /// Supported grant types
    #[serde(default)]
    pub grant_types_supported: Vec<String>,

    /// Supported code challenge methods (for PKCE)
    #[serde(default)]
    pub code_challenge_methods_supported: Vec<String>,

    /// Supported scopes
    #[serde(default)]
    pub scopes_supported: Vec<String>,
}

/// Dynamic client registration response (RFC 7591)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClientRegistrationResponse {
    pub client_id: String,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub client_id_issued_at: Option<u64>,
    #[serde(default)]
    pub client_secret_expires_at: Option<u64>,
}

/// Cached token data for persistence
#[derive(Debug, Serialize, Deserialize)]
struct CachedTokenData {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<u64>,
    client_id: String,
    client_secret: Option<String>,
    endpoint: String,
}

/// Result of checking if OAuth is required
#[derive(Debug)]
pub enum OAuthRequirement {
    /// OAuth not required, endpoint is accessible
    NotRequired,
    /// OAuth required with discovered metadata
    Required(OAuthMetadata),
    /// OAuth required but metadata discovery failed
    RequiredButNoMetadata(String),
}

/// OAuth discovery and authentication handler
pub struct OAuthDiscovery {
    http_client: Client,
    client_id: Option<String>,
    client_secret: Option<String>,
    default_redirect_port: u16,
}

impl OAuthDiscovery {
    /// Create a new OAuth discovery handler
    pub fn new() -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ClientError::Connection(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            http_client,
            client_id: None,
            client_secret: None,
            default_redirect_port: 8085,
        })
    }

    /// Create with custom client credentials
    pub fn with_credentials(
        client_id: Option<String>,
        client_secret: Option<String>,
        redirect_port: Option<u16>,
    ) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ClientError::Connection(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            http_client,
            client_id,
            client_secret,
            default_redirect_port: redirect_port.unwrap_or(8085),
        })
    }

    /// Get the token cache directory
    fn get_cache_dir() -> Option<PathBuf> {
        dirs::cache_dir().map(|p| p.join("mcp-connect").join("oauth"))
    }

    /// Get cache file path for an endpoint
    fn get_cache_path(endpoint: &str) -> Option<PathBuf> {
        Self::get_cache_dir().map(|dir| {
            let hash = format!("{:x}", md5::compute(endpoint.as_bytes()));
            dir.join(format!("{}.json", hash))
        })
    }

    /// Load cached token for an endpoint
    pub fn load_cached_token(endpoint: &str) -> Option<(String, Option<String>, String, Option<String>)> {
        let cache_path = Self::get_cache_path(endpoint)?;

        if !cache_path.exists() {
            debug!("No cached token found at {:?}", cache_path);
            return None;
        }

        let data = std::fs::read_to_string(&cache_path).ok()?;
        let cached: CachedTokenData = serde_json::from_str(&data).ok()?;

        // Check if token is expired
        if let Some(expires_at) = cached.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            if now >= expires_at {
                info!("Cached token expired, will need to refresh or re-authenticate");
                // Return the data anyway - we might be able to refresh
            }
        }

        info!("Loaded cached OAuth token for {}", endpoint);
        Some((cached.access_token, cached.refresh_token, cached.client_id, cached.client_secret))
    }

    /// Save token to cache with secure file permissions
    pub fn save_token_to_cache(
        endpoint: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<u64>,
        client_id: &str,
        client_secret: Option<&str>,
    ) -> std::result::Result<(), std::io::Error> {
        let cache_dir = match Self::get_cache_dir() {
            Some(dir) => dir,
            None => return Ok(()), // Silently skip if no cache dir
        };

        std::fs::create_dir_all(&cache_dir)?;

        // Set secure permissions on cache directory (owner only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            let _ = std::fs::set_permissions(&cache_dir, perms);
        }

        let cache_path = match Self::get_cache_path(endpoint) {
            Some(path) => path,
            None => return Ok(()),
        };

        let cached = CachedTokenData {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.map(|s| s.to_string()),
            expires_at,
            client_id: client_id.to_string(),
            client_secret: client_secret.map(|s| s.to_string()),
            endpoint: endpoint.to_string(),
        };

        let data = serde_json::to_string_pretty(&cached)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        std::fs::write(&cache_path, &data)?;

        // Set secure permissions on token file (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            let _ = std::fs::set_permissions(&cache_path, perms);
        }

        info!("Cached OAuth token at {:?}", cache_path);
        Ok(())
    }

    /// Clear cached token for an endpoint
    pub fn clear_cached_token(endpoint: &str) -> std::result::Result<(), std::io::Error> {
        if let Some(cache_path) = Self::get_cache_path(endpoint) {
            if cache_path.exists() {
                std::fs::remove_file(&cache_path)?;
                info!("Cleared cached token for {}", endpoint);
            }
        }
        Ok(())
    }

    /// Attempt dynamic client registration (RFC 7591)
    pub async fn register_client(
        &self,
        registration_endpoint: &str,
        redirect_uri: &str,
    ) -> Result<ClientRegistrationResponse> {
        info!("Attempting dynamic client registration...");

        let request_body = serde_json::json!({
            "client_name": "MCP Connect",
            "redirect_uris": [redirect_uri],
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
            "token_endpoint_auth_method": "none"
        });

        let response = self
            .http_client
            .post(registration_endpoint)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                ClientError::OAuthError(format!("Client registration request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ClientError::OAuthError(format!(
                "Client registration failed ({}): {}",
                status, body
            )));
        }

        let registration: ClientRegistrationResponse = response.json().await.map_err(|e| {
            ClientError::OAuthError(format!("Failed to parse registration response: {}", e))
        })?;

        info!(
            "Dynamic client registration successful, client_id: {}",
            registration.client_id
        );
        Ok(registration)
    }

    /// Check if an endpoint requires OAuth authentication
    pub async fn check_oauth_required(&self, endpoint: &str) -> Result<OAuthRequirement> {
        info!("Checking if OAuth is required for: {}", endpoint);

        // Try a simple request to see if we get 401
        let response = self
            .http_client
            .get(endpoint)
            .header("Accept", "application/json, text/event-stream")
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                debug!("Initial request returned status: {}", status);

                if status == reqwest::StatusCode::UNAUTHORIZED {
                    // Check for WWW-Authenticate header
                    if let Some(www_auth) = resp.headers().get("www-authenticate") {
                        debug!("WWW-Authenticate header: {:?}", www_auth);
                    }

                    // Try to discover OAuth metadata
                    match self.discover_oauth_metadata(endpoint).await {
                        Ok(metadata) => Ok(OAuthRequirement::Required(metadata)),
                        Err(e) => Ok(OAuthRequirement::RequiredButNoMetadata(e.to_string())),
                    }
                } else if status.is_success() || status == reqwest::StatusCode::METHOD_NOT_ALLOWED {
                    // Server is accessible without auth
                    Ok(OAuthRequirement::NotRequired)
                } else {
                    // Other error, might still need auth
                    warn!("Unexpected status {}, attempting OAuth discovery", status);
                    match self.discover_oauth_metadata(endpoint).await {
                        Ok(metadata) => Ok(OAuthRequirement::Required(metadata)),
                        Err(_) => Ok(OAuthRequirement::NotRequired),
                    }
                }
            }
            Err(e) => {
                // Connection error - try OAuth discovery anyway
                debug!("Connection error: {}, trying OAuth discovery", e);
                match self.discover_oauth_metadata(endpoint).await {
                    Ok(metadata) => Ok(OAuthRequirement::Required(metadata)),
                    Err(_) => Err(ClientError::Connection(format!(
                        "Failed to connect to {}: {}",
                        endpoint, e
                    ))),
                }
            }
        }
    }

    /// Discover OAuth metadata from the server's well-known endpoint
    pub async fn discover_oauth_metadata(&self, endpoint: &str) -> Result<OAuthMetadata> {
        let base_url = Url::parse(endpoint)
            .map_err(|e| ClientError::OAuthError(format!("Invalid endpoint URL: {}", e)))?;

        // Construct well-known URL
        let well_known_url = format!(
            "{}://{}{}/.well-known/oauth-authorization-server",
            base_url.scheme(),
            base_url
                .host_str()
                .ok_or_else(|| ClientError::OAuthError("No host in URL".to_string()))?,
            base_url
                .port()
                .map(|p| format!(":{}", p))
                .unwrap_or_default()
        );

        debug!("Fetching OAuth metadata from: {}", well_known_url);

        let response = self
            .http_client
            .get(&well_known_url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                ClientError::OAuthError(format!("Failed to fetch OAuth metadata: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(ClientError::OAuthError(format!(
                "OAuth metadata endpoint returned {}",
                response.status()
            )));
        }

        let metadata: OAuthMetadata = response.json().await.map_err(|e| {
            ClientError::OAuthError(format!("Failed to parse OAuth metadata: {}", e))
        })?;

        debug!("Discovered OAuth metadata: {:?}", metadata);
        Ok(metadata)
    }

    /// Perform OAuth authentication using discovered metadata
    pub async fn authenticate(
        &self,
        metadata: &OAuthMetadata,
        scopes: Option<Vec<String>>,
    ) -> Result<ClientToken> {
        let redirect_url = format!("http://localhost:{}/callback", self.default_redirect_port);

        // Determine client credentials
        let (client_id, client_secret) = if let Some(ref id) = self.client_id {
            // Use provided credentials
            info!("Using provided OAuth client credentials");
            (id.clone(), self.client_secret.clone())
        } else if let Some(ref registration_endpoint) = metadata.registration_endpoint {
            // Try dynamic client registration
            info!("No client credentials provided, attempting dynamic registration at: {}", registration_endpoint);
            match self.register_client(registration_endpoint, &redirect_url).await {
                Ok(registration) => {
                    info!("Dynamic registration successful! client_id: {}", registration.client_id);
                    (registration.client_id, registration.client_secret)
                }
                Err(e) => {
                    warn!("Dynamic client registration failed: {}", e);
                    return Err(ClientError::OAuthError(format!(
                        "No client credentials provided and dynamic registration failed: {}. \
                         Please provide --oauth-client-id (and optionally --oauth-client-secret) \
                         with credentials from the OAuth provider.",
                        e
                    )));
                }
            }
        } else {
            return Err(ClientError::OAuthError(
                "No client credentials provided and server doesn't support dynamic registration. \
                 Please provide --oauth-client-id (and optionally --oauth-client-secret) \
                 with credentials from the OAuth provider."
                    .to_string(),
            ));
        };

        info!("Using client_id: {}, has_secret: {}", client_id, client_secret.is_some());

        // Use provided scopes or default from metadata
        let scopes = scopes.unwrap_or_else(|| {
            if metadata.scopes_supported.is_empty() {
                vec!["mcp".to_string()]
            } else {
                metadata.scopes_supported.clone()
            }
        });

        // Check if PKCE is supported
        let use_pkce = metadata
            .code_challenge_methods_supported
            .contains(&"S256".to_string())
            || metadata.code_challenge_methods_supported.is_empty(); // Default to PKCE if not specified

        let config = OAuthClientConfig {
            client_id,
            client_secret,
            auth_url: metadata.authorization_endpoint.clone(),
            token_url: metadata.token_endpoint.clone(),
            redirect_url: redirect_url.clone(),
            scopes,
            use_pkce,
        };

        let oauth_client = OAuthClient::new(config)?;

        // Generate authorization URL
        let auth_url = oauth_client.generate_auth_url().await?;

        // Start local server to receive callback
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.default_redirect_port))
            .await
            .map_err(|e| {
                ClientError::OAuthError(format!("Failed to start callback server: {}", e))
            })?;

        // Print instructions for user
        eprintln!("\n╔════════════════════════════════════════════════════════════════╗");
        eprintln!("║                    OAuth Authentication Required                ║");
        eprintln!("╠════════════════════════════════════════════════════════════════╣");
        eprintln!("║ Please open the following URL in your browser:                 ║");
        eprintln!("╚════════════════════════════════════════════════════════════════╝");
        eprintln!("\n{}\n", auth_url);
        eprintln!("Waiting for authorization callback...\n");

        // Try to open browser automatically
        if let Err(e) = open::that(&auth_url) {
            debug!("Could not open browser automatically: {}", e);
        }

        // Wait for callback with timeout
        let callback_result = tokio::time::timeout(
            Duration::from_secs(300), // 5 minute timeout
            self.wait_for_callback(&listener),
        )
        .await
        .map_err(|_| ClientError::OAuthError("OAuth authentication timed out".to_string()))?
        .map_err(|e| ClientError::OAuthError(format!("Callback failed: {}", e)))?;

        let (code, state) = callback_result;

        // Exchange code for token
        let token = oauth_client.exchange_code(&code, &state).await?;

        eprintln!("✓ OAuth authentication successful!\n");
        info!("OAuth authentication completed successfully");

        Ok(token)
    }

    /// Wait for OAuth callback on the local server
    async fn wait_for_callback(&self, listener: &TcpListener) -> Result<(String, String)> {
        let (mut socket, _) = listener.accept().await.map_err(|e| {
            ClientError::OAuthError(format!("Failed to accept callback connection: {}", e))
        })?;

        let (reader, mut writer) = socket.split();
        let mut reader = BufReader::new(reader);

        // Read the HTTP request
        let mut request_line = String::new();
        reader.read_line(&mut request_line).await.map_err(|e| {
            ClientError::OAuthError(format!("Failed to read callback request: {}", e))
        })?;

        debug!("Received OAuth callback request: {}", request_line.trim());

        // Parse the request to extract code and state
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(ClientError::OAuthError(
                "Invalid HTTP request in callback".to_string(),
            ));
        }

        let path_and_query = parts[1];

        // Parse query parameters
        let url = Url::parse(&format!("http://localhost{}", path_and_query)).map_err(|e| {
            ClientError::OAuthError(format!("Failed to parse callback URL: {}", e))
        })?;

        let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

        // Check for error response
        if let Some(error) = params.get("error") {
            let description = params
                .get("error_description")
                .map(|s| s.as_str())
                .unwrap_or("Unknown error");
            return Err(ClientError::OAuthError(format!(
                "OAuth error: {} - {}",
                error, description
            )));
        }

        let code = params
            .get("code")
            .ok_or_else(|| {
                ClientError::OAuthError("Missing authorization code in callback".to_string())
            })?
            .clone();

        let state = params
            .get("state")
            .ok_or_else(|| {
                ClientError::OAuthError("Missing state parameter in callback".to_string())
            })?
            .clone();

        // Send success response to browser
        let response = "HTTP/1.1 200 OK\r\n\
            Content-Type: text/html\r\n\
            Connection: close\r\n\r\n\
            <!DOCTYPE html>\
            <html><head><title>Authentication Successful</title></head>\
            <body style=\"font-family: system-ui; text-align: center; padding: 50px;\">\
            <h1 style=\"color: #22c55e;\">✓ Authentication Successful</h1>\
            <p>You can close this window and return to the terminal.</p>\
            </body></html>";

        writer.write_all(response.as_bytes()).await.ok();
        writer.flush().await.ok();

        Ok((code, state))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_discovery_creation() {
        let discovery = OAuthDiscovery::new().unwrap();
        assert!(discovery.client_id.is_none());
        assert_eq!(discovery.default_redirect_port, 8085);
    }

    #[test]
    fn test_oauth_discovery_with_credentials() {
        let discovery = OAuthDiscovery::with_credentials(
            Some("my-app".to_string()),
            Some("secret".to_string()),
            Some(9000),
        ).unwrap();
        assert_eq!(discovery.client_id, Some("my-app".to_string()));
        assert_eq!(discovery.client_secret, Some("secret".to_string()));
        assert_eq!(discovery.default_redirect_port, 9000);
    }
}
