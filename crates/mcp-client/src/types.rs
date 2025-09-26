//! Type definitions and re-exports for MCP client with rmcp integration
//!
//! This module provides type definitions and re-exports to maintain backward
//! compatibility while leveraging the official rmcp SDK types.

// Re-export rmcp model types
pub use rmcp::model::*;

// Re-export common types from mcp-types
pub use mcp_types::{
    CallToolRequest, CallToolResponse, ClientCapabilities, ClientInfo, Content, ErrorData,
    InitializeRequest, JsonRpcMessage, McpError, ServerCapabilities,
    ServerInfo, Tool, error_result, success_result, text_content,
};
// Use rmcp's InitializeResult as InitializeResponse for compatibility
pub use rmcp::model::InitializeResult as InitializeResponse;

// Additional convenience types
use std::collections::HashMap;

/// Legacy JSON-RPC request type for backward compatibility
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

/// Legacy JSON-RPC response type for backward compatibility
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// Legacy JSON-RPC notification type for backward compatibility
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

/// Legacy JSON-RPC error type for backward compatibility
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Transport configuration
#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub headers: HashMap<String, String>,
    pub timeout: Option<std::time::Duration>,
    pub allow_http: bool,
    pub proxy_url: Option<String>,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            headers: HashMap::new(),
            timeout: Some(std::time::Duration::from_secs(30)),
            allow_http: false,
            proxy_url: None,
        }
    }
}

impl TransportConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn allow_http(mut self) -> Self {
        self.allow_http = true;
        self
    }

    pub fn with_proxy(mut self, proxy_url: impl Into<String>) -> Self {
        self.proxy_url = Some(proxy_url.into());
        self
    }
}

/// Tool call result wrapper for easier handling
#[derive(Debug, Clone)]
pub struct ToolCallResult {
    pub name: String,
    pub result: CallToolResult,
}

impl ToolCallResult {
    pub fn new(name: impl Into<String>, result: CallToolResult) -> Self {
        Self {
            name: name.into(),
            result,
        }
    }

    pub fn is_success(&self) -> bool {
        self.result.is_error != Some(true)
    }

    pub fn is_error(&self) -> bool {
        self.result.is_error == Some(true)
    }

    pub fn get_text_content(&self) -> Vec<String> {
        self.result
            .content
            .iter()
            .filter_map(|content| match content {
                Content::Text(text_content) => Some(text_content.text.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn get_first_text(&self) -> Option<String> {
        self.get_text_content().into_iter().next()
    }
}

/// Server information with additional metadata
#[derive(Debug, Clone)]
pub struct ServerMetadata {
    pub info: ServerInfo,
    pub capabilities: ServerCapabilities,
    pub protocol_version: String,
    pub connected_at: std::time::SystemTime,
}

impl ServerMetadata {
    pub fn new(
        info: ServerInfo,
        capabilities: ServerCapabilities,
        protocol_version: impl Into<String>,
    ) -> Self {
        Self {
            info,
            capabilities,
            protocol_version: protocol_version.into(),
            connected_at: std::time::SystemTime::now(),
        }
    }

    pub fn supports_tools(&self) -> bool {
        self.capabilities.tools.is_some()
    }

    pub fn supports_resources(&self) -> bool {
        self.capabilities.resources.is_some()
    }

    pub fn supports_prompts(&self) -> bool {
        self.capabilities.prompts.is_some()
    }

    pub fn uptime(&self) -> std::time::Duration {
        std::time::SystemTime::now()
            .duration_since(self.connected_at)
            .unwrap_or_default()
    }
}

/// Connection status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl ConnectionStatus {
    pub fn is_connected(&self) -> bool {
        matches!(self, ConnectionStatus::Connected)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, ConnectionStatus::Error(_))
    }
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionStatus::Disconnected => write!(f, "Disconnected"),
            ConnectionStatus::Connecting => write!(f, "Connecting"),
            ConnectionStatus::Connected => write!(f, "Connected"),
            ConnectionStatus::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

/// Helper functions for creating common MCP objects
pub fn create_initialize_request(
    client_name: impl Into<String>,
    client_version: impl Into<String>,
) -> InitializeRequest {
    InitializeRequest {
        protocol_version: "2024-11-05".to_string(),
        capabilities: ClientCapabilities {
            roots: None,
            sampling: None,
        },
        client_info: ClientInfo {
            name: client_name.into(),
            version: client_version.into(),
        },
    }
}

pub fn create_call_tool_request(
    name: impl Into<String>,
    arguments: Option<serde_json::Map<String, serde_json::Value>>,
) -> CallToolRequestParam {
    CallToolRequestParam {
        name: name.into().into(),
        arguments,
    }
}

/// Convert from legacy CallToolRequest to rmcp CallToolRequestParam
impl From<CallToolRequest> for CallToolRequestParam {
    fn from(legacy: CallToolRequest) -> Self {
        CallToolRequestParam {
            name: legacy.name.into(),
            arguments: legacy.arguments,
        }
    }
}

/// Convert from rmcp CallToolResult to legacy CallToolResponse
impl From<CallToolResult> for CallToolResponse {
    fn from(rmcp_result: CallToolResult) -> Self {
        CallToolResponse {
            content: rmcp_result.content,
            is_error: rmcp_result.is_error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_config() {
        let config = TransportConfig::new()
            .with_header("Authorization", "Bearer token")
            .with_timeout(std::time::Duration::from_secs(60))
            .allow_http()
            .with_proxy("http://proxy:8080");

        assert_eq!(config.headers.get("Authorization"), Some(&"Bearer token".to_string()));
        assert_eq!(config.timeout, Some(std::time::Duration::from_secs(60)));
        assert!(config.allow_http);
        assert_eq!(config.proxy_url, Some("http://proxy:8080".to_string()));
    }

    #[test]
    fn test_tool_call_result() {
        let result = ToolCallResult::new(
            "test_tool",
            success_result(vec![text_content("Success message")])
        );

        assert!(result.is_success());
        assert!(!result.is_error());
        assert_eq!(result.get_first_text(), Some("Success message".to_string()));

        let error_result = ToolCallResult::new(
            "test_tool",
            error_result("Error message")
        );

        assert!(!error_result.is_success());
        assert!(error_result.is_error());
        assert_eq!(error_result.get_first_text(), Some("Error message".to_string()));
    }

    #[test]
    fn test_server_metadata() {
        let info = ServerInfo {
            name: "test-server".to_string(),
            version: "1.0.0".to_string(),
        };
        let capabilities = ServerCapabilities {
            tools: Some(mcp_types::ToolsCapability { list_changed: true }),
            resources: None,
            prompts: None,
            logging: None,
            completion: None,
        };

        let metadata = ServerMetadata::new(info, capabilities, "2024-11-05");

        assert!(metadata.supports_tools());
        assert!(!metadata.supports_resources());
        assert!(!metadata.supports_prompts());
        assert_eq!(metadata.protocol_version, "2024-11-05");
    }

    #[test]
    fn test_connection_status() {
        let status = ConnectionStatus::Connected;
        assert!(status.is_connected());
        assert!(!status.is_error());

        let error_status = ConnectionStatus::Error("Test error".to_string());
        assert!(!error_status.is_connected());
        assert!(error_status.is_error());

        assert_eq!(status.to_string(), "Connected");
        assert_eq!(error_status.to_string(), "Error: Test error");
    }

    #[test]
    fn test_type_conversions() {
        let legacy_request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: Some(serde_json::json!({"param": "value"}).as_object().unwrap().clone()),
        };

        let rmcp_request: CallToolRequestParam = legacy_request.into();
        assert_eq!(rmcp_request.name.as_str(), "test_tool");

        let rmcp_result = success_result(vec![text_content("Success")]);
        let legacy_response: CallToolResponse = rmcp_result.into();
        assert_eq!(legacy_response.is_error, Some(false));
    }

    #[test]
    fn test_helper_functions() {
        let init_req = create_initialize_request("test-client", "1.0.0");
        assert_eq!(init_req.client_info.name, "test-client");
        assert_eq!(init_req.client_info.version, "1.0.0");
        assert_eq!(init_req.protocol_version, "2024-11-05");

        let tool_req = create_call_tool_request("test_tool", None);
        assert_eq!(tool_req.name.as_str(), "test_tool");
        assert!(tool_req.arguments.is_none());
    }
}
