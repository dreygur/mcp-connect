use crate::error::{ProxyError, Result};
use crate::strategy::{create_remote_transport, TransportStrategy, TransportType};
use mcp_client::McpClient;
use mcp_server::{StdioTransport, Transport};
use mcp_types::{JsonRpcMessage, JsonRpcRequest, JsonRpcResponse, JsonRpcNotification};
use tracing::{debug, error, info, warn};

pub struct McpProxy {
    remote_client: Option<McpClient>,
    stdio_transport: Option<StdioTransport>,
    server_url: String,
    transport_strategy: TransportStrategy,
    headers: Vec<String>,
    connected_transport_type: Option<TransportType>,
    running: bool,
}

impl McpProxy {
    pub fn new(server_url: String) -> Self {
        Self {
            remote_client: None,
            stdio_transport: None,
            server_url,
            transport_strategy: TransportStrategy::default(),
            headers: Vec::new(),
            connected_transport_type: None,
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

        // Create STDIO transport
        self.setup_stdio_transport().await?;

        self.running = true;
        info!("MCP proxy started successfully");

        // Start the bidirectional message forwarding loop
        self.run_message_forwarding_loop().await
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

        self.remote_client = Some(client);
        Ok(())
    }

    async fn setup_stdio_transport(&mut self) -> Result<()> {
        info!("Setting up STDIO transport");
        let transport = StdioTransport::new()?;
        self.stdio_transport = Some(transport);
        Ok(())
    }

    async fn run_message_forwarding_loop(&mut self) -> Result<()> {
        let mut stdio_transport = self.stdio_transport.take()
            .ok_or_else(|| ProxyError::Protocol("STDIO transport not initialized".into()))?;

        let mut remote_client = self.remote_client.take()
            .ok_or_else(|| ProxyError::Protocol("Remote client not initialized".into()))?;

        loop {
            tokio::select! {
                // Handle messages from STDIO (local client)
                stdio_result = stdio_transport.receive() => {
                    match stdio_result {
                        Ok(message) => {
                            debug!("Received STDIO message");
                            if let Err(e) = self.handle_stdio_message(&mut stdio_transport, &mut remote_client, message).await {
                                error!("Error handling STDIO message: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            info!("STDIO connection closed: {}", e);
                            break;
                        }
                    }
                }
                // Handle Ctrl+C
                _ = tokio::signal::ctrl_c() => {
                    info!("Received Ctrl+C, shutting down");
                    break;
                }
            }
        }

        self.running = false;
        info!("MCP proxy stopped");
        Ok(())
    }

    async fn handle_stdio_message(
        &self,
        stdio_transport: &mut StdioTransport,
        remote_client: &mut McpClient,
        message: JsonRpcMessage,
    ) -> Result<()> {
        // Try to parse as request first
        if let Ok(request) = serde_json::from_value::<JsonRpcRequest>(message.clone()) {
            debug!("Forwarding request: {}", request.method);

            match request.method.as_str() {
                "initialize" => {
                    // Handle initialize locally AND forward to remote server for session setup
                    let response = self.handle_initialize_request(request.clone()).await?;
                    stdio_transport.send(serde_json::to_value(&response)?).await?;

                    // Also forward initialize to remote server to establish session
                    // but don't send response back (we already sent our local response)
                    if let Err(e) = remote_client.send_request(&request.method, request.params).await {
                        warn!("Failed to forward initialize to remote server: {}", e);
                    }
                }
                "ping" => {
                    // Handle ping locally - simple pong response
                    let response = self.handle_ping_request(request).await?;
                    stdio_transport.send(serde_json::to_value(&response)?).await?;
                }
                "tools/list" => {
                    // Always forward tools/list to remote server
                    debug!("Forwarding tools/list request to remote server");
                    match remote_client.send_request(&request.method, request.params).await {
                        Ok(response) => {
                            debug!("Received response from remote server for tools/list");
                            stdio_transport.send(serde_json::to_value(&response)?).await?;
                        }
                        Err(e) => {
                            error!("Failed to get tools/list from remote server: {}", e);
                            // Send error response
                            let error_response = JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: request.id,
                                result: None,
                                error: Some(mcp_types::JsonRpcError {
                                    code: -32603,
                                    message: format!("Remote server error: {}", e),
                                    data: None,
                                }),
                            };
                            stdio_transport.send(serde_json::to_value(&error_response)?).await?;
                        }
                    }
                }
                _ => {
                    // Forward other requests to remote server
                    let response = remote_client.send_request(&request.method, request.params).await?;
                    stdio_transport.send(serde_json::to_value(&response)?).await?;
                }
            }
            return Ok(());
        }

        // Try to parse as notification
        if let Ok(notification) = serde_json::from_value::<JsonRpcNotification>(message.clone()) {
            debug!("Forwarding notification: {}", notification.method);

            match notification.method.as_str() {
                "notifications/initialized" => {
                    // Handle locally AND forward to remote server
                    info!("Client initialized");
                    remote_client.send_notification(&notification.method, notification.params).await?;
                }
                _ => {
                    // Forward to remote server
                    remote_client.send_notification(&notification.method, notification.params).await?;
                }
            }
            return Ok(());
        }

        warn!("Received unknown message type");
        Ok(())
    }

    async fn handle_initialize_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        use mcp_types::*;

        let response = InitializeResponse {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: false,
                }),
                resources: None,
                prompts: None,
                logging: None,
                completion: None,
            },
            server_info: ServerInfo {
                name: "mcp-remote-proxy".to_string(),
                version: "0.1.0".to_string(),
            },
        };

        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::to_value(response)?),
            error: None,
        })
    }

    async fn handle_ping_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // Simple pong response
        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::json!({})), // Empty object as pong
            error: None,
        })
    }

    pub async fn stop(&mut self) -> Result<()> {
        if !self.running {
            return Ok(());
        }

        info!("Stopping MCP proxy");

        if let Some(ref mut client) = self.remote_client {
            client.close().await?;
        }

        if let Some(ref mut transport) = self.stdio_transport {
            transport.close().await?;
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
