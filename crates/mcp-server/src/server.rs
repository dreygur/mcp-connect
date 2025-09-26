//! MCP Server implementation using the official rmcp SDK

use crate::error::ServerError;
use mcp_types::{Tool, Content};
use rmcp::{
    handler::server::ServerHandler,
    model::{
        CallToolRequestParam, CallToolResult, InitializeRequestParam, InitializeResult,
        ListToolsResult, PaginatedRequestParam, CancelledNotificationParam,
        ProgressNotificationParam, ProtocolVersion,
    },
    service::{serve_server, RequestContext, NotificationContext, RoleServer},
    transport::stdio,
    ErrorData as McpError,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

/// Tool execution handler function type
pub type ToolHandler = Arc<dyn Fn(CallToolRequestParam) -> std::result::Result<CallToolResult, ServerError> + Send + Sync>;

/// Server information structure
#[derive(Clone)]
pub struct ServerInfoData {
    pub name: String,
    pub version: String,
    pub title: Option<String>,
    pub icons: Option<Vec<rmcp::model::Icon>>,
    pub website_url: Option<String>,
}

/// MCP Server using rmcp SDK
#[derive(Clone)]
pub struct McpServer {
    server_info: ServerInfoData,
    tools: Arc<Mutex<Vec<Tool>>>,
    tool_handlers: Arc<Mutex<HashMap<String, ToolHandler>>>,
}

impl McpServer {
    /// Create a new MCP server with basic information
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            server_info: ServerInfoData {
                name: name.into(),
                version: version.into(),
                title: None,
                icons: None,
                website_url: None,
            },
            tools: Arc::new(Mutex::new(Vec::new())),
            tool_handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Set optional title for the server
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.server_info.title = Some(title.into());
        self
    }

    /// Add a tool to the server
    pub async fn add_tool(&self, tool: Tool, handler: ToolHandler) {
        let mut tools = self.tools.lock().await;
        let mut handlers = self.tool_handlers.lock().await;

        handlers.insert(tool.name.to_string(), handler);
        tools.push(tool);
    }

    /// Create a success result for tool execution
    pub fn success_result(content: Vec<Content>) -> CallToolResult {
        CallToolResult {
            content,
            is_error: Some(false),
            meta: None,
            structured_content: None,
        }
    }

    /// Create an error result for tool execution
    pub fn error_result(message: impl Into<String>) -> CallToolResult {
        CallToolResult {
            content: vec![Content::text(message.into())],
            is_error: Some(true),
            meta: None,
            structured_content: None,
        }
    }

    /// Run the server with STDIO transport
    pub async fn run(self) -> std::result::Result<(), ServerError> {
        info!("Starting MCP server");

        // Create server service using rmcp
        let _service = serve_server(self, stdio()).await
            .map_err(|e| ServerError::Transport(format!("Failed to create server: {}", e)))?;

        info!("MCP server stopped");
        Ok(())
    }
}

impl ServerHandler for McpServer {
    fn ping(
        &self,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), McpError>> + Send + '_ {
        async move {
            debug!("Received ping request");
            Ok(())
        }
    }

    fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<InitializeResult, McpError>> + Send + '_ {
        async move {
            debug!("Initialization complete");
            Ok(self.get_info())
        }
    }

    fn complete(
        &self,
        _request: rmcp::model::CompleteRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::CompleteResult, McpError>> + Send + '_ {
        async move {
            // Not implemented - return method not found error
            Err(McpError::method_not_found::<rmcp::model::CompleteRequestMethod>())
        }
    }

    fn set_level(
        &self,
        _request: rmcp::model::SetLevelRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), McpError>> + Send + '_ {
        async move {
            info!("Client has completed initialization");
            Ok(())
        }
    }

    fn get_prompt(
        &self,
        _request: rmcp::model::GetPromptRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::GetPromptResult, McpError>> + Send + '_ {
        async move {
            Err(McpError::method_not_found::<rmcp::model::GetPromptRequestMethod>())
        }
    }

    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::ListPromptsResult, McpError>> + Send + '_ {
        async move {
            Ok(rmcp::model::ListPromptsResult {
                prompts: vec![],
                next_cursor: None,
            })
        }
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::ListResourcesResult, McpError>> + Send + '_ {
        async move {
            Ok(rmcp::model::ListResourcesResult {
                resources: vec![],
                next_cursor: None,
            })
        }
    }

    fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::ListResourceTemplatesResult, McpError>> + Send + '_ {
        async move {
            Ok(rmcp::model::ListResourceTemplatesResult {
                resource_templates: vec![],
                next_cursor: None,
            })
        }
    }

    fn read_resource(
        &self,
        _request: rmcp::model::ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::ReadResourceResult, McpError>> + Send + '_ {
        async move {
            Err(McpError::method_not_found::<rmcp::model::ReadResourceRequestMethod>())
        }
    }

    fn subscribe(
        &self,
        _request: rmcp::model::SubscribeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), McpError>> + Send + '_ {
        async move {
            Ok(())
        }
    }

    fn unsubscribe(
        &self,
        _request: rmcp::model::UnsubscribeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), McpError>> + Send + '_ {
        async move {
            Ok(())
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        async move {
            debug!("Tool execution started");

            let handlers = self.tool_handlers.lock().await;
            if let Some(handler) = handlers.get(request.name.as_ref()) {
                match handler(request) {
                    Ok(result) => Ok(result),
                    Err(e) => Ok(Self::error_result(format!("Tool execution failed: {}", e))),
                }
            } else {
                let error_msg = format!("Tool '{}' not found", request.name);
                Ok(Self::error_result(error_msg))
            }
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        async move {
            debug!("Listing available tools");

            let tools = self.tools.lock().await;
            let tools = tools.clone();

            info!("Returned {} tools", tools.len());
            Ok(ListToolsResult {
                tools,
                next_cursor: None,
            })
        }
    }

    fn on_cancelled(
        &self,
        _notification: CancelledNotificationParam,
        _context: NotificationContext<RoleServer>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {
            debug!("Received cancellation notification");
        }
    }

    fn on_progress(
        &self,
        _notification: ProgressNotificationParam,
        _context: NotificationContext<RoleServer>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {
            debug!("Received progress notification");
        }
    }

    fn on_initialized(
        &self,
        _context: NotificationContext<RoleServer>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {
            info!("Client has completed initialization");
        }
    }

    fn on_roots_list_changed(
        &self,
        _context: NotificationContext<RoleServer>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {
            debug!("Roots list changed notification received");
        }
    }

    fn get_info(&self) -> rmcp::model::ServerInfo {
        InitializeResult {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: rmcp::model::ServerCapabilities {
                tools: Some(rmcp::model::ToolsCapability { list_changed: Some(false) }),
                ..Default::default()
            },
            server_info: rmcp::model::Implementation {
                name: self.server_info.name.clone(),
                version: self.server_info.version.clone(),
                title: self.server_info.title.clone(),
                icons: self.server_info.icons.clone(),
                website_url: self.server_info.website_url.clone(),
            },
            instructions: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = McpServer::new("test-server", "1.0.0")
            .with_title("Test MCP Server");

        assert_eq!(server.server_info.name, "test-server");
        assert_eq!(server.server_info.version, "1.0.0");
        assert_eq!(server.server_info.title, Some("Test MCP Server".to_string()));
    }

    #[tokio::test]
    async fn test_tool_addition() {
        let server = McpServer::new("test-server", "1.0.0");

        let tool = Tool {
            name: "echo".into(),
            description: Some("Echo tool".into()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "Text to echo"
                    }
                },
                "required": ["text"]
            }),
        };

        let handler: ToolHandler = Arc::new(|request| {
            let text = request.arguments
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("no text");

            Ok(McpServer::success_result(vec![Content::text(text.to_string())]))
        });

        server.add_tool(tool, handler).await;

        let tools = server.tools.lock().await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
    }
}
