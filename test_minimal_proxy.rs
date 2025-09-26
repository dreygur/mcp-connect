use rmcp::{
    handler::server::ServerHandler,
    model::{
        InitializeRequestParam, InitializeResult, ProtocolVersion, ServerCapabilities,
        Implementation, ToolsCapability,
    },
    service::{serve_server, RequestContext, RoleServer},
    transport::stdio,
    ErrorData as McpError,
};

struct MinimalHandler;

impl ServerHandler for MinimalHandler {
    fn ping(
        &self,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), McpError>> + Send + '_ {
        async move { Ok(()) }
    }

    fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<InitializeResult, McpError>> + Send + '_ {
        async move {
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

    fn get_info(&self) -> rmcp::model::ServerInfo {
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    println!("Starting minimal server");
    let handler = MinimalHandler;
    let _service = serve_server(handler, stdio()).await?;
    println!("Server stopped");
    Ok(())
}
