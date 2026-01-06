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
    /// Create a new registry client with default settings (official MCP registry).
    pub fn new() -> Result<Self> {
        Self::with_base_url(REGISTRY_BASE_URL, Some(API_VERSION))
    }

    /// Create a new registry client with a custom base URL.
    ///
    /// # Arguments
    /// * `base_url` - The base URL of the registry (e.g., "https://registry.example.com")
    /// * `api_version` - Optional API version path (e.g., "v1", "v0.1"). If None, uses default.
    pub fn with_base_url(base_url: &str, api_version: Option<&str>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(concat!("mcp-registry/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("Failed to create HTTP client")?;

        let base_url = if let Some(version) = api_version {
            format!("{}/{}", base_url.trim_end_matches('/'), version)
        } else {
            base_url.trim_end_matches('/').to_string()
        };

        Ok(Self {
            client,
            base_url,
        })
    }

    /// Fetch a page of servers from the registry.
    pub async fn fetch_servers_page(
        &self,
        cursor: Option<String>,
        limit: usize,
        search_query: Option<&str>,
    ) -> Result<(Vec<ServerResponse>, Option<String>)> {
        let mut url = format!("{}/servers?limit={}", self.base_url, limit);

        if let Some(c) = cursor {
            url.push_str(&format!("&cursor={}", urlencoding::encode(&c)));
        }

        if let Some(query) = search_query {
            url.push_str(&format!("&search={}", urlencoding::encode(query)));
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

        let next_cursor = list_response.metadata.next_cursor;
        Ok((list_response.servers, next_cursor))
    }

    /// List all servers with full details (using pagination).
    pub async fn list_all_servers_full(&self) -> Result<Vec<ServerResponse>> {
        let mut all_servers = Vec::new();
        let mut cursor: Option<String> = None;
        let limit = 100;

        loop {
            let (servers, next_cursor) = self.fetch_servers_page(cursor, limit, None).await?;
            all_servers.extend(servers);

            if let Some(c) = next_cursor {
                cursor = Some(c);
            } else {
                break;
            }
        }

        info!("Fetched {} total servers from registry", all_servers.len());
        Ok(all_servers)
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
                    || server.description.to_lowercase().contains(&query_lower)
            })
            .collect();

        info!("Found {} servers matching '{}'", results.len(), query);
        Ok(results)
    }

    /// Get detailed information about a specific server.
    pub async fn get_server(&self, name: &str, version: &str) -> Result<ServerResponse> {
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

        let server_response: ServerResponse = response
            .json()
            .await
            .context("Failed to parse server details")?;

        info!("Successfully fetched server: {}", name);
        Ok(server_response)
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

            // Convert ServerResponse items to ServerSummary
            for server_response in list_response.servers {
                let status = server_response.meta.official
                    .as_ref()
                    .map(|o| o.status.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                all_servers.push(ServerSummary {
                    name: server_response.server.name,
                    description: server_response.server.description,
                    status,
                    version: server_response.server.version,
                });
            }

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
        server.remotes.as_ref().map_or(false, |r| !r.is_empty())
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
                .filter_map(|kv| {
                    kv.value.as_ref().map(|v| KeyValue {
                        key: kv.name.clone(),
                        value: v.clone(),
                    })
                })
                .collect()
        });

        Ok(Some(RemoteConfig {
            transport_type,
            url: remote.url.clone(),
            headers,
            oauth: None,
        }))
    }
}


/// Response from the servers list endpoint.
#[derive(Debug, Deserialize)]
struct ServerListResponse {
    servers: Vec<ServerResponse>,
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

/// API response wrapper with server data and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerResponse {
    pub server: ServerDetail,
    #[serde(rename = "_meta")]
    pub meta: ServerMetadata,
}

/// Registry-managed metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetadata {
    #[serde(rename = "io.modelcontextprotocol.registry/official")]
    pub official: Option<OfficialMetadata>,
    /// Allow unknown fields for extensibility
    #[serde(flatten)]
    pub additional: std::collections::HashMap<String, serde_json::Value>,
}

/// Official MCP registry metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficialMetadata {
    pub status: String,
    #[serde(rename = "publishedAt")]
    pub published_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,
    #[serde(rename = "isLatest")]
    pub is_latest: Option<bool>,
}

/// Summary information about a server from the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSummary {
    /// Registry name (e.g., "io.github.modelcontextprotocol/github-mcp-server")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Server status (active, deleted, etc.)
    pub status: String,
    /// Latest version number
    pub version: String,
}

/// Repository metadata for the MCP server source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub url: Option<String>,
    pub source: Option<String>,
    pub id: Option<String>,
    pub subfolder: Option<String>,
}

/// Detailed information about a server from the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerDetail {
    /// JSON schema URL
    #[serde(rename = "$schema")]
    pub schema: Option<String>,
    /// Registry name
    pub name: String,
    /// Human-readable description (optional in practice even though spec says required)
    #[serde(default)]
    pub description: String,
    /// Optional human-readable title
    pub title: Option<String>,
    /// Version number
    pub version: String,
    /// Optional website URL
    #[serde(rename = "websiteUrl")]
    pub website_url: Option<String>,
    /// Repository metadata
    pub repository: Option<Repository>,
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

/// Key-value pair from registry API (for headers and environment variables).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryKeyValue {
    pub name: String,
    pub value: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "isRequired")]
    pub is_required: Option<bool>,
    #[serde(rename = "isSecret")]
    pub is_secret: Option<bool>,
}

/// Package configuration from registry (npm, pip, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    #[serde(rename = "registryType")]
    pub registry_type: String,
    pub identifier: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Allow unknown fields for extensibility (transport, environmentVariables, etc.)
    #[serde(flatten)]
    pub additional: std::collections::HashMap<String, serde_json::Value>,
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
            description: "A test server".to_string(),
            title: None,
            version: "1.0.0".to_string(),
            website_url: None,
            repository: None,
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
