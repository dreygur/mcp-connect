use rmcp::{
    handler::server::ServerHandler,
    model::*,
    service::{RequestContext, NotificationContext, RoleServer, ServiceExt},
    transport::stdio,
    ErrorData as McpError,
};
use std::sync::Arc;

#[derive(Clone)]
struct MinimalServer;

impl ServerHandler for MinimalServer {
    fn ping(&self, _: RequestContext<RoleServer>) -> impl std::future::Future<Output = Result<(), McpError>> + Send + '_ {
        async { Ok(()) }
    }

    fn initialize(&self, _: InitializeRequestParam, _: RequestContext<RoleServer>) -> impl std::future::Future<Output = Result<InitializeResult, McpError>> + Send + '_ {
        async {
            Ok(InitializeResult {
                protocol_version: ProtocolVersion::V_2024_11_05,
                capabilities: ServerCapabilities::default(),
                server_info: Implementation {
                    name: "minimal".into(),
                    version: "1.0.0".into(),
                    title: None,
                    icons: None,
                    website_url: None,
                },
                instructions: None,
            })
        }
    }

    fn get_info(&self) -> ServerInfo {
        InitializeResult {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: "minimal".into(),
                version: "1.0.0".into(),
                title: None,
                icons: None,
                website_url: None,
            },
            instructions: None,
        }
    }

    // Implement all other required methods with minimal implementations
    fn complete(&self, _: CompleteRequestParam, _: RequestContext<RoleServer>) -> impl std::future::Future<Output = Result<CompleteResult, McpError>> + Send + '_ {
        async { Err(McpError::method_not_found::<CompleteRequestMethod>()) }
    }

    fn set_level(&self, _: SetLevelRequestParam, _: RequestContext<RoleServer>) -> impl std::future::Future<Output = Result<(), McpError>> + Send + '_ {
        async { Ok(()) }
    }

    fn get_prompt(&self, _: GetPromptRequestParam, _: RequestContext<RoleServer>) -> impl std::future::Future<Output = Result<GetPromptResult, McpError>> + Send + '_ {
        async { Err(McpError::method_not_found::<GetPromptRequestMethod>()) }
    }

    fn list_prompts(&self, _: Option<PaginatedRequestParam>, _: RequestContext<RoleServer>) -> impl std::future::Future<Output = Result<ListPromptsResult, McpError>> + Send + '_ {
        async { Ok(ListPromptsResult { prompts: vec![], next_cursor: None }) }
    }

    fn list_tools(&self, _: Option<PaginatedRequestParam>, _: RequestContext<RoleServer>) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        async { Ok(ListToolsResult { tools: vec![], next_cursor: None }) }
    }

    fn call_tool(&self, _: CallToolRequestParam, _: RequestContext<RoleServer>) -> impl std::future::Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        async { Err(McpError::internal_error("No tools available", None)) }
    }

    fn list_resources(&self, _: Option<PaginatedRequestParam>, _: RequestContext<RoleServer>) -> impl std::future::Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        async { Ok(ListResourcesResult { resources: vec![], next_cursor: None }) }
    }

    fn list_resource_templates(&self, _: Option<PaginatedRequestParam>, _: RequestContext<RoleServer>) -> impl std::future::Future<Output = Result<ListResourceTemplatesResult, McpError>> + Send + '_ {
        async { Ok(ListResourceTemplatesResult { resource_templates: vec![], next_cursor: None }) }
    }

    fn read_resource(&self, _: ReadResourceRequestParam, _: RequestContext<RoleServer>) -> impl std::future::Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        async { Err(McpError::method_not_found::<ReadResourceRequestMethod>()) }
    }

    fn subscribe(&self, _: SubscribeRequestParam, _: RequestContext<RoleServer>) -> impl std::future::Future<Output = Result<(), McpError>> + Send + '_ {
        async { Ok(()) }
    }

    fn unsubscribe(&self, _: UnsubscribeRequestParam, _: RequestContext<RoleServer>) -> impl std::future::Future<Output = Result<(), McpError>> + Send + '_ {
        async { Ok(()) }
    }

    fn on_cancelled(&self, _: CancelledNotificationParam, _: NotificationContext<RoleServer>) -> impl std::future::Future<Output = ()> + Send + '_ {
        async {}
    }

    fn on_progress(&self, _: ProgressNotificationParam, _: NotificationContext<RoleServer>) -> impl std::future::Future<Output = ()> + Send + '_ {
        async {}
    }

    fn on_initialized(&self, _: NotificationContext<RoleServer>) -> impl std::future::Future<Output = ()> + Send + '_ {
        async {}
    }

    fn on_roots_list_changed(&self, _: NotificationContext<RoleServer>) -> impl std::future::Future<Output = ()> + Send + '_ {
        async {}
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = MinimalServer;
    println!("Starting minimal server...");
    let result = server.serve(stdio()).await;
    println!("Server result: {:?}", result.is_ok());
    if let Err(e) = result {
        println!("Error: {}", e);
    }
    Ok(())
}
