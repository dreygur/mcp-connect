//! MCP Server - STDIO interface for IDE/LLM communication using rmcp SDK
//!
//! This crate implements an MCP server that communicates with clients (IDEs/LLMs)
//! via standard input/output streams using the official rmcp SDK and MCP protocol.

pub mod error;
pub mod server;

// Re-export core functionality
pub use server::{McpServer, ToolHandler};
pub use error::{ServerError, Result};

// Re-export rmcp types for convenience
pub use rmcp::{
    model::{
        CallToolRequestParam, CallToolResult, Content, Tool, ServerInfo,
        InitializeRequestParam, InitializeResult,
    },
    handler::server::ServerHandler,
    ErrorData as McpError,
};

// Re-export mcp-types for backward compatibility
pub use mcp_types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_basic_functionality() {
        let server = McpServer::new("integration-test-server", "0.1.0")
            .with_title("Integration Test Server");

        // Test server info
        let info = server.get_info();
        assert_eq!(info.name, "integration-test-server");
        assert_eq!(info.version, "0.1.0");
        assert_eq!(info.title, Some("Integration Test Server".to_string()));

        // Add a simple tool
        let tool = Tool {
            name: "hello".into(),
            description: Some("Says hello".into()),
            input_schema: std::sync::Arc::new(
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name to greet"
                        }
                    }
                }).as_object().unwrap().clone()
            ),
            annotations: None,
            icons: None,
            output_schema: None,
            title: None,
        };

        let handler = |request: CallToolRequestParam| -> Result<CallToolResult> {
            let name = request
                .arguments
                .as_ref()
                .and_then(|args| args.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("World");

            Ok(McpServer::success_result(vec![
                Content::text(format!("Hello, {}!", name))
            ]))
        };

        server.add_tool(tool, handler).await.unwrap();

        // Test tool listing
        let tools_result = server.list_tools(None, Default::default()).await.unwrap();
        assert_eq!(tools_result.tools.len(), 1);
        assert_eq!(tools_result.tools[0].name, "hello");

        // Test tool execution
        let call_request = CallToolRequestParam {
            name: "hello".into(),
            arguments: Some(serde_json::json!({"name": "Alice"}).as_object().unwrap().clone()),
        };

        let call_result = server.call_tool(call_request, Default::default()).await.unwrap();
        assert_eq!(call_result.is_error, Some(false));
        assert_eq!(call_result.content.len(), 1);

        // Test initialization
        let init_request = InitializeRequestParam {
            protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
            capabilities: rmcp::model::ClientCapabilities {
                roots: None,
                sampling: None,
                experimental: None,
                elicitation: None,
            },
            client_info: rmcp::model::Implementation {
                name: "test-client".to_string(),
                version: "1.0.0".to_string(),
                title: None,
                icons: None,
                website_url: None,
            },
        };

        let init_result = server.initialize(init_request, Default::default()).await.unwrap();
        assert_eq!(init_result.server_info.name, "integration-test-server");
        assert!(init_result.capabilities.tools.is_some());
    }
}
