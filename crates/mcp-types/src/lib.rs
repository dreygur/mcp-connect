//! MCP Types - Official rmcp SDK integration
//!
//! This crate provides MCP protocol types by re-exporting the official rmcp SDK types
//! while maintaining backward compatibility with the existing codebase.

// Re-export core rmcp types
pub use rmcp::model::*;
pub use rmcp::{ErrorData, RmcpError};

// Additional convenience re-exports
pub use rmcp::service::{Service, ServiceExt, ServiceError};
pub use rmcp::{ClientHandler, ServerHandler};

// Common serde re-exports
pub use rmcp::serde_json;

// Backward compatibility type aliases
pub type JsonRpcMessage = serde_json::Value;
pub type McpError = rmcp::ErrorData;

// Legacy JSON-RPC types for backward compatibility
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

// Utility functions using rmcp methods
pub fn text_content(text: impl Into<String>) -> Content {
    Content::text(text)
}

pub fn success_result(content: Vec<Content>) -> CallToolResult {
    CallToolResult {
        content,
        is_error: Some(false),
        meta: None,
        structured_content: None,
    }
}

pub fn error_result(message: impl Into<String>) -> CallToolResult {
    CallToolResult {
        content: vec![text_content(message.into())],
        is_error: Some(true),
        meta: None,
        structured_content: None,
    }
}

// Error handling utilities
pub mod error {
    pub use rmcp::ErrorData;
    pub use rmcp::RmcpError;

    /// Common error codes
    pub mod codes {
        pub const PARSE_ERROR: i32 = -32700;
        pub const INVALID_REQUEST: i32 = -32600;
        pub const METHOD_NOT_FOUND: i32 = -32601;
        pub const INVALID_PARAMS: i32 = -32602;
        pub const INTERNAL_ERROR: i32 = -32603;
    }
}

// Convert from legacy CallToolRequest to rmcp CallToolRequestParam
impl From<CallToolRequestParam> for CallToolRequest {
    fn from(rmcp_param: CallToolRequestParam) -> Self {
        CallToolRequest {
            name: rmcp_param.name.to_string(),
            arguments: rmcp_param.arguments,
        }
    }
}

// Backward compatibility CallToolRequest
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CallToolRequest {
    pub name: String,
    pub arguments: Option<serde_json::Map<String, serde_json::Value>>,
}

// Backward compatibility CallToolResponse
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CallToolResponse {
    pub content: Vec<Content>,
    #[serde(rename = "isError")]
    pub is_error: Option<bool>,
}

impl From<CallToolResult> for CallToolResponse {
    fn from(result: CallToolResult) -> Self {
        CallToolResponse {
            content: result.content,
            is_error: result.is_error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_creation() {
        let content = text_content("Hello, world!");
        // Content was created successfully
        match content {
            Content::Text(_) => {
                // Success - it's a text content
            }
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_result_creation() {
        let result = success_result(vec![text_content("Success")]);
        assert_eq!(result.is_error, Some(false));
        assert_eq!(result.content.len(), 1);

        let error = error_result("Failed");
        assert_eq!(error.is_error, Some(true));
    }

    #[test]
    fn test_type_conversions() {
        let rmcp_result = success_result(vec![text_content("Success")]);
        let legacy_response: CallToolResponse = rmcp_result.into();
        assert_eq!(legacy_response.is_error, Some(false));
    }
}
