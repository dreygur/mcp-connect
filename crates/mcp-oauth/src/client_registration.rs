use crate::{OAuthError, Result};
use crate::types::{ClientRegistrationRequest, ClientRegistrationResponse, OAuthServerMetadata};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Dynamic Client Registration implementation according to RFC 7591
///
/// This allows MCP clients to dynamically register with OAuth 2.0 authorization servers
/// without pre-existing client credentials.
pub struct ClientRegistration {
    http_client: Client,
    server_metadata: OAuthServerMetadata,
}

impl ClientRegistration {
    /// Create a new client registration handler
    pub fn new(server_metadata: OAuthServerMetadata) -> Self {
        Self {
            http_client: Client::new(),
            server_metadata,
        }
    }

    /// Register a new OAuth client with the authorization server
    ///
    /// # Arguments
    /// * `redirect_uri` - The callback URI for the OAuth flow
    /// * `client_name` - Optional human-readable client name
    ///
    /// # Returns
    /// Client registration response with client_id and optionally client_secret
    pub async fn register_client(
        &self,
        redirect_uri: &str,
        client_name: Option<&str>,
    ) -> Result<ClientRegistrationResponse> {
        let registration_endpoint = self.server_metadata.registration_endpoint
            .as_ref()
            .ok_or_else(|| OAuthError::ClientRegistration(
                "Server does not support dynamic client registration".to_string()
            ))?;

        info!("Registering OAuth client with server at: {}", registration_endpoint);

        // Build registration request
        let mut request = ClientRegistrationRequest {
            redirect_uris: vec![redirect_uri.to_string()],
            client_name: client_name.map(|s| s.to_string()),
            client_uri: Some("https://github.com/your-org/mcp-remote".to_string()), // TODO: Make configurable
            logo_uri: None,
            scope: Some("mcp".to_string()), // Request MCP scope
            contacts: Some(vec!["support@your-org.com".to_string()]), // TODO: Make configurable
            tos_uri: None,
            policy_uri: None,
            jwks_uri: None,
            software_id: Some("mcp-remote".to_string()),
            software_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            additional_metadata: HashMap::new(),
        };

        // Add MCP-specific metadata
        request.additional_metadata.insert(
            "mcp_version".to_string(),
            Value::String("2024-11-05".to_string())
        );
        request.additional_metadata.insert(
            "application_type".to_string(),
            Value::String("native".to_string())
        );

        debug!("Registration request: {:#?}", request);

        // Send registration request
        let response = self.http_client
            .post(registration_endpoint)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            warn!("Client registration failed: {} - {}", status, error_body);
            return Err(OAuthError::ClientRegistration(
                format!("Registration failed with status {}: {}", status, error_body)
            ));
        }

        let registration_response: ClientRegistrationResponse = response.json().await?;

        info!("Successfully registered OAuth client: {}", registration_response.client_id);
        debug!("Registration response: {:#?}", registration_response);

        Ok(registration_response)
    }

    /// Discover OAuth server metadata from well-known endpoint
    ///
    /// # Arguments
    /// * `server_base_url` - Base URL of the OAuth server
    ///
    /// # Returns
    /// OAuth server metadata including endpoints and capabilities
    pub async fn discover_server_metadata(
        server_base_url: &str,
    ) -> Result<OAuthServerMetadata> {
        let client = Client::new();

        // Try standard OAuth 2.0 Authorization Server Metadata (RFC 8414)
        let well_known_url = format!("{}/.well-known/oauth-authorization-server",
                                    server_base_url.trim_end_matches('/'));

        info!("Discovering OAuth server metadata from: {}", well_known_url);

        let response = client.get(&well_known_url).send().await;

        let metadata = match response {
            Ok(resp) if resp.status().is_success() => {
                debug!("Successfully discovered OAuth metadata");
                resp.json::<OAuthServerMetadata>().await?
            }
            Ok(resp) => {
                warn!("OAuth metadata discovery failed with status: {}", resp.status());
                // Fallback: construct basic metadata from server URL
                Self::construct_fallback_metadata(server_base_url)?
            }
            Err(e) => {
                warn!("OAuth metadata discovery request failed: {}", e);
                // Fallback: construct basic metadata from server URL
                Self::construct_fallback_metadata(server_base_url)?
            }
        };

        debug!("OAuth server metadata: {:#?}", metadata);
        Ok(metadata)
    }

    /// Construct fallback OAuth server metadata when discovery fails
    ///
    /// This creates a basic metadata structure with common endpoint paths
    /// when the server doesn't support metadata discovery.
    fn construct_fallback_metadata(server_base_url: &str) -> Result<OAuthServerMetadata> {
        let base_url = server_base_url.trim_end_matches('/');

        info!("Constructing fallback OAuth metadata for: {}", base_url);

        Ok(OAuthServerMetadata {
            issuer: base_url.to_string(),
            authorization_endpoint: format!("{}/oauth/authorize", base_url),
            token_endpoint: format!("{}/oauth/token", base_url),
            registration_endpoint: Some(format!("{}/oauth/register", base_url)),
            jwks_uri: Some(format!("{}/oauth/jwks", base_url)),
            response_types_supported: Some(vec!["code".to_string()]),
            grant_types_supported: Some(vec![
                "authorization_code".to_string(),
                "refresh_token".to_string(),
            ]),
            token_endpoint_auth_methods_supported: Some(vec![
                "client_secret_basic".to_string(),
                "client_secret_post".to_string(),
                "none".to_string(), // For public clients with PKCE
            ]),
            scopes_supported: Some(vec![
                "mcp".to_string(),
                "openid".to_string(),
            ]),
            code_challenge_methods_supported: Some(vec![
                "S256".to_string(),
                "plain".to_string(),
            ]),
            additional_metadata: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construct_fallback_metadata() {
        let metadata = ClientRegistration::construct_fallback_metadata("https://example.com/auth")
            .unwrap();

        assert_eq!(metadata.issuer, "https://example.com/auth");
        assert_eq!(metadata.authorization_endpoint, "https://example.com/auth/oauth/authorize");
        assert_eq!(metadata.token_endpoint, "https://example.com/auth/oauth/token");
        assert_eq!(metadata.registration_endpoint, Some("https://example.com/auth/oauth/register".to_string()));
    }

    #[test]
    fn test_construct_fallback_metadata_with_trailing_slash() {
        let metadata = ClientRegistration::construct_fallback_metadata("https://example.com/auth/")
            .unwrap();

        assert_eq!(metadata.issuer, "https://example.com/auth");
        assert_eq!(metadata.authorization_endpoint, "https://example.com/auth/oauth/authorize");
    }
}
