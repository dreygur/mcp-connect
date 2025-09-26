use crate::error::{ProxyError, Result};
use crate::strategy::{create_remote_transport, TransportStrategy, TransportType};
use mcp_client::McpClient;
use mcp_server::{McpServer, StdioTransport};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn};

pub struct McpProxy {
    remote_client: Option<McpClient>,
    local_server: Option<McpServer>,
    server_url: String,
    transport_strategy: TransportStrategy,
    headers: Vec<String>,
    connected_transport_type: Option<TransportType>,
    message_queue: Arc<Mutex<Vec<serde_json::Value>>>,
    running: bool,
}

impl McpProxy {
    pub fn new(server_url: String) -> Self {
        Self {
            remote_client: None,
            local_server: None,
            server_url,
            transport_strategy: TransportStrategy::default(),
            headers: Vec::new(),
            connected_transport_type: None,
            message_queue: Arc::new(Mutex::new(Vec::new())),
            running: false,
        }
    }

    pub fn with_transport_strategy(mut self, strategy: TransportStrategy) -> Self {
        self.transport_strategy = strategy;
        self
    }

    pub fn with_headers(mut self, headers: Vec<String>) -> Self {
        self.headers = headers;
        self
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting MCP proxy for server: {}", self.server_url);

        // Create and connect remote client
        self.connect_remote_client().await?;

        // Create local server with STDIO transport
        self.setup_local_server().await?;

        self.running = true;
        info!("MCP proxy started successfully");

        // Start the message forwarding loop
        self.run_proxy_loop().await
    }

    async fn connect_remote_client(&mut self) -> Result<()> {
        info!("Connecting to remote MCP server: {}", self.server_url);

        let (transport, transport_type) =
            create_remote_transport(&self.server_url, self.transport_strategy.clone(), &self.headers).await?;

        info!("Connected using transport: {:?}", transport_type);
        self.connected_transport_type = Some(transport_type);

        let mut client = McpClient::new("mcp-remote-proxy".to_string(), "0.1.0".to_string());
        let init_response = client.connect(transport).await?;

        info!(
            "Connected to remote server: {} v{}",
            init_response.server_info.name, init_response.server_info.version
        );

        // Get available tools from remote server
        // TODO: Implement proper session management for GitHub MCP server
        // let tools = client.list_tools().await.unwrap_or_default();
        // info!("Remote server provides {} tools", tools.len());
        info!("Skipping tool listing for now due to session management");

        self.remote_client = Some(client);
        Ok(())
    }

    async fn setup_local_server(&mut self) -> Result<()> {
        info!("Setting up local STDIO MCP server");

        let transport = Box::new(StdioTransport::new()?);
        let mut server = McpServer::new("mcp-remote-proxy".to_string(), "0.1.0".to_string())
            .with_transport(transport);

        // Get tools from remote client and add them to local server
        // TODO: Implement proper session management for GitHub MCP server
        // if let Some(ref mut client) = self.remote_client {
        //     let tools = client.list_tools().await.unwrap_or_default();
        //     // Tools are now shared types, so no conversion needed
        //     server = server.with_tools(tools);
        // }

        // For now, we'll add a simple handler that returns an error
        // In a full implementation, we'd need a proper async handler system
        server.add_request_handler("tools/call", |request| {
            Ok(mcp_types::JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(mcp_types::JsonRpcError {
                    code: -32603,
                    message: "Tool forwarding not yet implemented".to_string(),
                    data: None,
                }),
            })
        });

        self.local_server = Some(server);
        Ok(())
    }

    async fn run_proxy_loop(&mut self) -> Result<()> {
        let server = self.local_server.take()
            .ok_or_else(|| ProxyError::Protocol("Local server not initialized".into()))?;

        let (_shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        // Spawn server task
        let server_handle = {
            let mut server = server;
            tokio::spawn(async move {
                if let Err(e) = server.run().await {
                    error!("Local server error: {}", e);
                }
                info!("Local server stopped");
            })
        };

        // Spawn remote client message handler
        let client_handle = {
            let client = self.remote_client.take();
            tokio::spawn(async move {
                if let Some(mut client) = client {
                    loop {
                        match client.receive_message().await {
                            Ok(()) => {
                                debug!("Processed remote message");
                            }
                            Err(e) => {
                                warn!("Remote client message error: {}", e);
                                break;
                            }
                        }
                    }
                }
                info!("Remote client handler stopped");
            })
        };

        // Wait for shutdown signal or task completion
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("Received shutdown signal");
            }
            _ = server_handle => {
                info!("Server task completed");
            }
            _ = client_handle => {
                info!("Client task completed");
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl+C, shutting down");
            }
        }

        self.running = false;
        info!("MCP proxy stopped");
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if !self.running {
            return Ok(());
        }

        info!("Stopping MCP proxy");

        if let Some(ref mut client) = self.remote_client {
            client.close().await?;
        }

        if let Some(ref mut server) = self.local_server {
            server.close().await?;
        }

        self.running = false;
        info!("MCP proxy stopped successfully");
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn connected_transport_type(&self) -> Option<&TransportType> {
        self.connected_transport_type.as_ref()
    }
}
