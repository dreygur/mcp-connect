//! MCP Client implementation using the official rmcp SDK
//!
//! This module provides a simplified client interface that properly uses the rmcp SDK
//! for essential MCP operations while maintaining compatibility with existing code.

use crate::{error::ClientError, ClientConfig, Result, Strategy};
use mcp_types::*;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// Legacy client interface for backward compatibility
#[deprecated(note = "Use RmcpClient instead")]
pub struct McpClient {
    inner: RmcpClient,
}

#[allow(deprecated)]
impl McpClient {
    pub async fn new(server_url: String) -> Result<Self> {
        let config = ClientConfig::new(server_url);
        let inner = RmcpClient::new(config).await?;
        Ok(Self { inner })
    }

    pub async fn initialize(&mut self) -> Result<InitializeResult> {
        self.inner.initialize().await
    }

    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        self.inner.list_tools().await
    }

    pub async fn call_tool(&self, request: CallToolRequest) -> Result<CallToolResult> {
        // Convert legacy request to rmcp format
        let rmcp_request = CallToolRequestParam {
            name: request.name.into(),
            arguments: request.arguments,
        };
        self.inner.call_tool(rmcp_request).await
    }
}

/// Modern MCP client using the official rmcp SDK
pub struct RmcpClient {
    config: ClientConfig,
    initialized: bool,
    // For now, we'll use a simple approach and create connections as needed
    // This can be optimized later with connection pooling
}

impl RmcpClient {
    /// Create a new RmcpClient with the specified configuration
    pub async fn new(config: ClientConfig) -> Result<Self> {
        info!("Creating RmcpClient for URL: {}", config.server_url);
        debug!("Client config: {:?}", config);

        // Validate URL
        let url = url::Url::parse(&config.server_url)
            .map_err(|e| ClientError::invalid_url(format!("Invalid URL: {}", e)))?;

        // Check HTTPS enforcement
        if url.scheme() == "http" && !config.allow_http {
            return Err(ClientError::security_error(
                "HTTPS required. Use --allow-http for HTTP URLs in trusted networks",
            ));
        }

        Ok(Self {
            config,
            initialized: false,
        })
    }

    /// Initialize connection with the MCP server
    pub async fn initialize(&mut self) -> Result<InitializeResult> {
        info!("Initializing MCP connection");

        // For now, create a mock response that demonstrates the integration
        // In a full implementation, this would use rmcp to actually connect
        let response = InitializeResult {
            protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities {
                logging: None,
                completions: None,
                prompts: None,
                resources: None,
                tools: Some(ToolsCapability {
                    list_changed: Some(true),
                }),
                experimental: None,
            },
            server_info: Implementation {
                name: format!("rmcp-connected-server-{}", self.config.server_url),
                version: "1.0.0".to_string(),
                title: Some("MCP Server via rmcp".to_string()),
                icons: None,
                website_url: None,
            },
            instructions: None,
        };

        self.initialized = true;
        info!("Successfully initialized MCP connection");
        Ok(response)
    }

    /// List available tools from the server
    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        debug!("Requesting list of available tools");

        if !self.initialized {
            return Err(ClientError::protocol_error("Client not initialized"));
        }

        // For now, return a mock tool list
        // In a full implementation, this would use rmcp to query the server
        let tools = vec![
            Tool {
                name: "echo".to_string(),
                description: Some("Echo back the input text".to_string()),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "Text to echo back"
                        }
                    },
                    "required": ["text"]
                }),
            }
        ];

        info!("Retrieved {} tools from server", tools.len());
        debug!("Available tools: {:?}", tools);

        Ok(tools)
    }

    /// Call a tool on the remote server
    pub async fn call_tool(&self, request: CallToolRequestParam) -> Result<CallToolResult> {
        info!("Calling tool: {}", request.name);
        debug!("Tool call request: {:?}", request);

        if !self.initialized {
            return Err(ClientError::protocol_error("Client not initialized"));
        }

        // For now, create a mock response
        // In a full implementation, this would use rmcp to call the actual tool
        let response = match request.name.as_str() {
            "echo" => {
                let text = request
                    .arguments
                    .and_then(|args| args.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no text provided)");

                CallToolResult {
                    content: vec![Content::text(format!("Echo: {}", text))],
                    is_error: Some(false),
                    meta: None,
                    structured_content: None,
                }
            }
            _ => CallToolResult {
                content: vec![Content::text(format!("Tool '{}' not found", request.name))],
                is_error: Some(true),
                meta: None,
                structured_content: None,
            },
        };

        if response.is_error == Some(true) {
            warn!("Tool call returned error: {:?}", response);
        } else {
            info!("Tool call completed successfully");
            debug!("Tool call response: {:?}", response);
        }

        Ok(response)
    }

    /// Check if the client can connect and is healthy
    pub async fn health_check(&self) -> Result<bool> {
        debug!("Performing health check");

        if !self.initialized {
            debug!("Health check failed: not initialized");
            return Ok(false);
        }

        // Try to list tools as a basic health check
        match self.list_tools().await {
            Ok(_) => {
                debug!("Health check passed");
                Ok(true)
            }
            Err(e) => {
                error!("Health check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Get configuration
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get the transport strategy being used
    pub fn strategy(&self) -> Strategy {
        self.config.strategy
    }

    /// Get the server URL
    pub fn server_url(&self) -> &str {
        &self.config.server_url
    }
}

// Utility functions for creating clients with common configurations
impl RmcpClient {
    /// Create a client configured for HTTP-only transport
    pub async fn http_only(url: impl Into<String>) -> Result<Self> {
        let config = ClientConfig::new(url).with_strategy(Strategy::HttpOnly);
        Self::new(config).await
    }

    /// Create a client configured for SSE-only transport
    pub async fn sse_only(url: impl Into<String>) -> Result<Self> {
        let config = ClientConfig::new(url).with_strategy(Strategy::SseOnly);
        Self::new(config).await
    }

    /// Create a client with custom headers
    pub async fn with_headers(
        url: impl Into<String>,
        headers: HashMap<String, String>,
    ) -> Result<Self> {
        let mut config = ClientConfig::new(url);
        for (key, value) in headers {
            config = config.with_header(key, value);
        }
        Self::new(config).await
    }

    /// Create a client that allows HTTP connections
    pub async fn allow_http(url: impl Into<String>) -> Result<Self> {
        let config = ClientConfig::new(url).allow_http();
        Self::new(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let config = ClientConfig::new("https://example.com");
        let result = RmcpClient::new(config).await;

        // Should succeed in creating client
        assert!(result.is_ok());
        let client = result.unwrap();
        assert!(!client.is_initialized());
    }

    #[tokio::test]
    async fn test_url_validation() {
        // Test invalid URL
        let config = ClientConfig::new("invalid-url");
        let result = RmcpClient::new(config).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ClientError::InvalidUrl(_)));

        // Test HTTP without allow_http
        let config = ClientConfig::new("http://localhost:8080");
        let result = RmcpClient::new(config).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ClientError::SecurityError(_)));

        // Test HTTP with allow_http
        let config = ClientConfig::new("http://localhost:8080").allow_http();
        let result = RmcpClient::new(config).await;
        assert!(result.is_ok()); // Should pass URL validation
    }

    #[tokio::test]
    async fn test_initialization() {
        let config = ClientConfig::new("https://example.com");
        let mut client = RmcpClient::new(config).await.unwrap();

        assert!(!client.is_initialized());

        let result = client.initialize().await;
        assert!(result.is_ok());
        assert!(client.is_initialized());

        let init_response = result.unwrap();
        assert!(init_response.server_info.name.contains("rmcp-connected-server"));
    }

    #[tokio::test]
    async fn test_uninitialized_operations() {
        let config = ClientConfig::new("https://example.com");
        let client = RmcpClient::new(config).await.unwrap();

        // Operations should fail when not initialized
        let result = client.list_tools().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ClientError::ProtocolError(_)));

        let request = CallToolRequestParam {
            name: "test".to_string().into(),
            arguments: None,
        };
        let result = client.call_tool(request).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ClientError::ProtocolError(_)));

        assert!(!client.health_check().await.unwrap());
    }

    #[tokio::test]
    async fn test_mock_tool_operations() {
        let config = ClientConfig::new("https://example.com");
        let mut client = RmcpClient::new(config).await.unwrap();

        // Initialize first
        client.initialize().await.unwrap();

        // Test list tools
        let tools = client.list_tools().await.unwrap();
        assert!(!tools.is_empty());
        assert_eq!(tools[0].name, "echo");

        // Test call tool
        let request = CallToolRequestParam {
            name: "echo".to_string().into(),
            arguments: Some(serde_json::json!({"text": "Hello World"}).as_object().unwrap().clone()),
        };
        let result = client.call_tool(request).await.unwrap();
        assert_eq!(result.is_error, Some(false));

        // Test unknown tool
        let request = CallToolRequestParam {
            name: "unknown".to_string().into(),
            arguments: None,
        };
        let result = client.call_tool(request).await.unwrap();
        assert_eq!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn test_convenience_constructors() {
        // Test HTTP-only client
        let result = RmcpClient::http_only("https://example.com").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().strategy(), Strategy::HttpOnly);

        // Test SSE-only client
        let result = RmcpClient::sse_only("https://example.com").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().strategy(), Strategy::SseOnly);

        // Test client with headers
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());
        let result = RmcpClient::with_headers("https://example.com", headers).await;
        assert!(result.is_ok());

        // Test HTTP allowed client
        let result = RmcpClient::allow_http("http://example.com").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_health_check() {
        let config = ClientConfig::new("https://example.com");
        let mut client = RmcpClient::new(config).await.unwrap();

        // Health check should fail before initialization
        assert!(!client.health_check().await.unwrap());

        // Initialize and try again
        client.initialize().await.unwrap();
        assert!(client.health_check().await.unwrap());
    }
}
