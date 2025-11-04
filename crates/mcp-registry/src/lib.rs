//! MCP Registry API client.
//!
//! This crate provides an interface to the official MCP Registry API
//! for discovering and fetching server information.

use anyhow::{Context, Result};
use mcp_types::{KeyValue, RemoteConfig, RemoteTransportType};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Base URL for the MCP Registry API
const REGISTRY_BASE_URL: &str = "https://registry.modelcontextprotocol.io";

/// API version to use (stable version)
const API_VERSION: &str = "v0.1";

/// Registry API client for fetching server information.
pub struct RegistryClient {
    client: reqwest::Client,
    base_url: String,
}

impl RegistryClient {
    /// Create a new registry client with default settings.
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("mcp-connect/0.1.0")
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            base_url: format!("{}/{}", REGISTRY_BASE_URL, API_VERSION),
        })
    }

    /// Search for servers in the registry.
    ///
    /// Returns a list of servers matching the query string.
    pub async fn search(&self, query: &str) -> Result<Vec<ServerSummary>> {
        info!("Searching registry for: {}", query);

        // Fetch all servers (registry doesn't have search endpoint yet)
        let servers = self.list_all_servers().await?;

        // Filter servers by query
        let results: Vec<ServerSummary> = servers
            .into_iter()
            .filter(|server| {
                let query_lower = query.to_lowercase();
                server.name.to_lowercase().contains(&query_lower)
                    || server
                        .description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .collect();

        info!("Found {} servers matching '{}'", results.len(), query);
        Ok(results)
    }

    /// Get detailed information about a specific server.
    pub async fn get_server(&self, name: &str, version: &str) -> Result<ServerDetail> {
        info!("Fetching server: {} version {}", name, version);

        let url = format!("{}/servers/{}/versions/{}", self.base_url, urlencoding::encode(name), urlencoding::encode(version));

        debug!("GET {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch server details")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch server '{}': HTTP {}", name, response.status());
        }

        let server: ServerDetail = response
            .json()
            .await
            .context("Failed to parse server details")?;

        info!("Successfully fetched server: {}", name);
        Ok(server)
    }

    /// List all servers in the registry with pagination.
    async fn list_all_servers(&self) -> Result<Vec<ServerSummary>> {
        let mut all_servers = Vec::new();
        let mut cursor: Option<String> = None;
        let limit = 100;

        loop {
            let mut url = format!("{}/servers?limit={}", self.base_url, limit);
            if let Some(c) = &cursor {
                url.push_str(&format!("&cursor={}", urlencoding::encode(c)));
            }

            debug!("GET {}", url);

            let response = self
                .client
                .get(&url)
                .send()
                .await
                .context("Failed to list servers")?;

            if !response.status().is_success() {
                anyhow::bail!("Failed to list servers: HTTP {}", response.status());
            }

            let list_response: ServerListResponse = response
                .json()
                .await
                .context("Failed to parse server list")?;

            all_servers.extend(list_response.servers);

            // Check if there are more pages
            if let Some(next_cursor) = list_response.metadata.next_cursor {
                cursor = Some(next_cursor);
            } else {
                break;
            }
        }

        info!("Fetched {} total servers from registry", all_servers.len());
        Ok(all_servers)
    }

    /// Check if a server has remote (HTTP) transport support.
    pub fn has_remote_transport(server: &ServerDetail) -> bool {
        server.remotes.is_some() && !server.remotes.as_ref().unwrap().is_empty()
    }

    /// Convert ServerDetail to our RemoteConfig format.
    pub fn to_remote_config(server: &ServerDetail) -> Result<Option<RemoteConfig>> {
        let remotes = match &server.remotes {
            Some(r) if !r.is_empty() => r,
            _ => return Ok(None),
        };

        // Take the first remote transport (usually there's only one)
        let remote = &remotes[0];

        let transport_type = match remote.transport_type.as_str() {
            "streamable-http" => RemoteTransportType::StreamableHttp,
            "sse" => RemoteTransportType::Sse,
            _ => anyhow::bail!("Unsupported transport type: {}", remote.transport_type),
        };

        let headers = remote.headers.as_ref().map(|h| {
            h.iter()
                .map(|kv| KeyValue {
                    key: kv.key.clone(),
                    value: kv.value.clone(),
                })
                .collect()
        });

        Ok(Some(RemoteConfig {
            transport_type,
            url: remote.url.clone(),
            headers,
        }))
    }
}

impl Default for RegistryClient {
    fn default() -> Self {
        Self::new().expect("Failed to create registry client")
    }
}

/// Response from the servers list endpoint.
#[derive(Debug, Deserialize)]
struct ServerListResponse {
    servers: Vec<ServerSummary>,
    metadata: ListMetadata,
}

/// Metadata for paginated list responses.
#[derive(Debug, Deserialize)]
struct ListMetadata {
    #[allow(dead_code)]
    count: usize,
    #[serde(rename = "nextCursor")]
    next_cursor: Option<String>,
}

/// Summary information about a server from the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSummary {
    /// Registry name (e.g., "io.github.modelcontextprotocol/github-mcp-server")
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Server status (active, deleted, etc.)
    pub status: String,
    /// Latest version number
    pub version: String,
}

/// Detailed information about a server from the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerDetail {
    /// JSON schema URL
    #[serde(rename = "$schema")]
    pub schema: Option<String>,
    /// Registry name
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Server status
    pub status: String,
    /// Version number
    pub version: String,
    /// Remote transport configurations
    pub remotes: Option<Vec<RemoteTransport>>,
    /// Package configurations (npm, pip, etc.) - not used by mcp-connect
    pub packages: Option<Vec<Package>>,
}

/// Remote transport configuration from registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTransport {
    /// Transport type ("streamable-http" or "sse")
    #[serde(rename = "type")]
    pub transport_type: String,
    /// Remote server URL
    pub url: String,
    /// HTTP headers
    pub headers: Option<Vec<RegistryKeyValue>>,
}

/// Key-value pair from registry API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryKeyValue {
    pub key: String,
    pub value: String,
}

/// Package configuration from registry (npm, pip, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    #[serde(rename = "registryType")]
    pub registry_type: String,
    pub identifier: String,
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_client_creation() {
        let client = RegistryClient::new();
        assert!(client.is_ok());
    }

    #[test]
    fn test_has_remote_transport() {
        let server = ServerDetail {
            schema: None,
            name: "test/server".to_string(),
            description: None,
            status: "active".to_string(),
            version: "1.0.0".to_string(),
            remotes: Some(vec![RemoteTransport {
                transport_type: "streamable-http".to_string(),
                url: "https://example.com/mcp".to_string(),
                headers: None,
            }]),
            packages: None,
        };

        assert!(RegistryClient::has_remote_transport(&server));
    }
}
