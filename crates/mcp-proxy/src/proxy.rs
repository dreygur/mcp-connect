//! MCP Proxy implementation using rmcp SDK
//!
//! This proxy acts as a bridge between local STDIO clients (IDEs/LLMs) and remote HTTP/SSE servers.
//! It uses rmcp's service layer to handle protocol details and provides seamless bidirectional communication.

use crate::error::{ProxyError, Result};
use crate::strategy::{TransportStrategy, TransportType};
use rmcp::{
    handler::server::ServerHandler,
    model::{
        CallToolRequestParam, CallToolResult, InitializeRequestParam, InitializeResult,
        ListToolsResult, PaginatedRequestParam, ServerInfo, CancelledNotificationParam,
        ProgressNotificationParam, ProtocolVersion, ServerCapabilities, Implementation,
        ToolsCapability,
    },
    service::{RequestContext, NotificationContext, RoleServer, ServiceExt},
    transport::stdio,
    ErrorData as McpError,
};
use tracing::{debug, info, error};

/// Proxy server that forwards requests between local STDIO clients and remote HTTP/SSE servers
pub struct McpProxy {
    server_url: String,
    transport_strategy: TransportStrategy,
    headers: Vec<String>,
    connected_transport_type: Option<TransportType>,
}

impl McpProxy {
    /// Create a new MCP proxy for the given server URL
    pub fn new(server_url: String) -> Self {
        Self {
            server_url,
            transport_strategy: TransportStrategy::default(),
            headers: Vec::new(),
            connected_transport_type: None,
        }
    }

    /// Set the transport strategy for connecting to the remote server
    pub fn with_transport_strategy(mut self, strategy: TransportStrategy) -> Self {
        self.transport_strategy = strategy;
        self
    }

    /// Add custom headers for the remote connection
    pub fn with_headers(mut self, headers: Vec<String>) -> Self {
        self.headers = headers;
        self
    }

    /// Start the proxy server
    pub async fn start(self) -> Result<()> {
        info!("Starting MCP proxy for server: {}", self.server_url);

        // TODO: Connect to remote server first
        // For now, we'll create a simple proxy that forwards to a placeholder
        info!("Note: Remote connection not yet implemented - using placeholder");

        // Create remote client and proxy handler
        let remote_client = RemoteClient::new(self.server_url.clone());
        let proxy_handler = ProxyHandler::new(self.server_url.clone(), remote_client);

        // Start serving on STDIO
        info!("Starting STDIO server for local clients");

        // Debug: Check if stdio() transport is created correctly
        let stdio_transport = stdio();
        info!("Created STDIO transport");

        // Try serving with the handler
        let service_result = proxy_handler.serve(stdio_transport).await;
        info!("Serve call completed with result: {:?}", service_result.is_ok());

        let _service = service_result
            .map_err(|e| ProxyError::Transport(format!("Failed to start STDIO server: {}", e)))?;

        info!("MCP proxy stopped");
        Ok(())
    }

    /// Get the type of transport currently connected
    pub fn connected_transport_type(&self) -> Option<&TransportType> {
        self.connected_transport_type.as_ref()
    }
}

/// Placeholder for remote client - will be replaced with actual rmcp client
#[derive(Clone)]
struct RemoteClient {
    server_url: String,
}

impl RemoteClient {
    fn new(server_url: String) -> Self {
        Self { server_url }
    }

    async fn initialize(&self) -> std::result::Result<InitializeResult, McpError> {
        // TODO: Connect to actual remote server and forward initialize request
        info!("Forwarding initialize to remote server: {}", self.server_url);

        // For now, return a placeholder response
        Ok(InitializeResult {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: Some(false) }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "remote-server".into(),
                version: "unknown".into(),
                title: Some("Remote MCP Server".into()),
                icons: None,
                website_url: None,
            },
            instructions: Some("Connected via mcp-remote proxy".to_string()),
        })
    }

    async fn list_tools(&self) -> std::result::Result<ListToolsResult, McpError> {
        // TODO: Forward to remote server
        info!("Forwarding list_tools to remote server: {}", self.server_url);
        Ok(ListToolsResult {
            tools: vec![],
            next_cursor: None,
        })
    }

    async fn call_tool(&self, request: CallToolRequestParam) -> std::result::Result<CallToolResult, McpError> {
        // TODO: Forward to remote server
        info!("Forwarding call_tool '{}' to remote server: {}", request.name, self.server_url);
        Err(McpError::internal_error("Remote forwarding not yet implemented", None))
    }
}

/// Server handler that forwards all requests to the remote service
#[derive(Clone)]
struct ProxyHandler {
    server_url: String,
    remote_client: RemoteClient,
}

impl ProxyHandler {
    fn new(server_url: String, remote_client: RemoteClient) -> Self {
        Self {
            server_url,
            remote_client,
        }
    }
}

impl ServerHandler for ProxyHandler {
    fn ping(
        &self,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<(), McpError>> + Send + '_ {
        async move {
            debug!("Ping request - responding locally");
            Ok(())
        }
    }

    fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<InitializeResult, McpError>> + Send + '_ {
        async move {
            debug!("Proxying initialization to remote server");
            self.remote_client.initialize().await
        }
    }

    fn complete(
        &self,
        _request: rmcp::model::CompleteRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<rmcp::model::CompleteResult, McpError>> + Send + '_ {
        async move {
            debug!("Complete request - not yet implemented");
            Err(McpError::internal_error("Not implemented", None))
        }
    }

    fn set_level(
        &self,
        _request: rmcp::model::SetLevelRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<(), McpError>> + Send + '_ {
        async move {
            debug!("Set level request - not yet implemented");
            Err(McpError::internal_error("Not implemented", None))
        }
    }

    fn get_prompt(
        &self,
        _request: rmcp::model::GetPromptRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<rmcp::model::GetPromptResult, McpError>> + Send + '_ {
        async move {
            debug!("Get prompt request - not yet implemented");
            Err(McpError::internal_error("Not implemented", None))
        }
    }

    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<rmcp::model::ListPromptsResult, McpError>> + Send + '_ {
        async move {
            debug!("List prompts request - returning empty list");
            Ok(rmcp::model::ListPromptsResult {
                prompts: vec![],
                next_cursor: None,
            })
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<ListToolsResult, McpError>> + Send + '_ {
        async move {
            debug!("Proxying list_tools to remote server");
            self.remote_client.list_tools().await
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<CallToolResult, McpError>> + Send + '_ {
        async move {
            debug!("Proxying call_tool to remote server");
            self.remote_client.call_tool(request).await
        }
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<rmcp::model::ListResourcesResult, McpError>> + Send + '_ {
        async move {
            debug!("List resources request - returning empty list");
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
    ) -> impl std::future::Future<Output = std::result::Result<rmcp::model::ListResourceTemplatesResult, McpError>> + Send + '_ {
        async move {
            debug!("List resource templates request - returning empty list");
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
    ) -> impl std::future::Future<Output = std::result::Result<rmcp::model::ReadResourceResult, McpError>> + Send + '_ {
        async move {
            debug!("Read resource request - not yet implemented");
            Err(McpError::internal_error("Not implemented", None))
        }
    }

    fn subscribe(
        &self,
        _request: rmcp::model::SubscribeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<(), McpError>> + Send + '_ {
        async move {
            debug!("Subscribe request - not yet implemented");
            Err(McpError::internal_error("Not implemented", None))
        }
    }

    fn unsubscribe(
        &self,
        _request: rmcp::model::UnsubscribeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<(), McpError>> + Send + '_ {
        async move {
            debug!("Unsubscribe request - not yet implemented");
            Err(McpError::internal_error("Not implemented", None))
        }
    }



    fn on_cancelled(
        &self,
        _notification: CancelledNotificationParam,
        _context: NotificationContext<RoleServer>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {
            debug!("Cancelled notification - received");
        }
    }

    fn on_progress(
        &self,
        _notification: ProgressNotificationParam,
        _context: NotificationContext<RoleServer>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {
            debug!("Progress notification - received");
        }
    }

    fn on_initialized(
        &self,
        _context: NotificationContext<RoleServer>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {
            info!("Client initialized notification");
        }
    }

    fn on_roots_list_changed(
        &self,
        _context: NotificationContext<RoleServer>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {
            debug!("Roots list changed notification - received");
        }
    }

    fn get_info(&self) -> ServerInfo {
        // Return proxy server info - the real server info will come from initialize()
        InitializeResult {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: Some(false) }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "mcp-remote-proxy".into(),
                version: "0.1.0".into(),
                title: Some("MCP Remote Proxy".into()),
                icons: None,
                website_url: None,
            },
            instructions: Some(format!("Proxying to: {}", self.server_url)),
        }
    }
}
