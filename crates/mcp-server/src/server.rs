use crate::error::{ServerError, Result};
use crate::transport::Transport;
use crate::types::*;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type RequestHandler = Arc<dyn Fn(JsonRpcRequest) -> Result<JsonRpcResponse> + Send + Sync>;

pub struct McpServer {
    transport: Option<Box<dyn Transport>>,
    server_info: ServerInfo,
    capabilities: ServerCapabilities,
    request_handlers: std::collections::HashMap<String, RequestHandler>,
    tools: Vec<Tool>,
}

impl McpServer {
    pub fn new(name: String, version: String) -> Self {
        let mut server = Self {
            transport: None,
            server_info: ServerInfo { name, version },
            capabilities: ServerCapabilities::default(),
            request_handlers: std::collections::HashMap::new(),
            tools: Vec::new(),
        };

        // Register default handlers
        server.register_default_handlers();
        server
    }

    pub fn with_transport(mut self, transport: Box<dyn Transport>) -> Self {
        self.transport = Some(transport);
        self
    }

    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = tools;
        self.capabilities.tools = Some(ToolsCapability {
            list_changed: false,
        });
        self
    }

    pub fn add_request_handler<F>(&mut self, method: &str, handler: F)
    where
        F: Fn(JsonRpcRequest) -> Result<JsonRpcResponse> + Send + Sync + 'static,
    {
        self.request_handlers.insert(method.to_string(), Arc::new(handler));
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            let is_connected = {
                let transport = self.transport.as_ref()
                    .ok_or_else(|| ServerError::Transport("No transport configured".into()))?;
                transport.is_connected()
            };

            if !is_connected {
                break;
            }

            let message = {
                let transport = self.transport.as_mut()
                    .ok_or_else(|| ServerError::Transport("No transport configured".into()))?;
                transport.receive().await
            };

            match message {
                Ok(message) => {
                    if let Err(e) = self.handle_message(message).await {
                        tracing::error!("Error handling message: {}", e);
                    }
                }
                Err(ServerError::ConnectionClosed) => {
                    tracing::info!("Connection closed");
                    break;
                }
                Err(e) => {
                    tracing::error!("Transport error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_message(&mut self, message: JsonRpcMessage) -> Result<()> {
        // Try to parse as request first
        if let Ok(request) = serde_json::from_value::<JsonRpcRequest>(message.clone()) {
            let response = self.handle_request(request).await?;
            self.send_response(response).await?;
            return Ok(());
        }

        // Try to parse as notification
        if let Ok(notification) = serde_json::from_value::<JsonRpcNotification>(message.clone()) {
            self.handle_notification(notification).await?;
            return Ok(());
        }

        // Try to parse as response (client responding to server request)
        if let Ok(_response) = serde_json::from_value::<JsonRpcResponse>(message) {
            // Handle client responses if needed
            tracing::debug!("Received response from client");
            return Ok(());
        }

        Err(ServerError::Protocol("Invalid message format".into()))
    }

    async fn handle_request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        if let Some(handler) = self.request_handlers.get(&request.method) {
            return handler(request);
        }

        // Return method not found error
        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: "Method not found".to_string(),
                data: None,
            }),
        })
    }

    async fn handle_notification(&mut self, notification: JsonRpcNotification) -> Result<()> {
        match notification.method.as_str() {
            "notifications/initialized" => {
                tracing::info!("Client initialized");
            }
            _ => {
                tracing::debug!("Received notification: {}", notification.method);
            }
        }
        Ok(())
    }

    async fn send_response(&mut self, response: JsonRpcResponse) -> Result<()> {
        let transport = self.transport.as_mut()
            .ok_or_else(|| ServerError::Transport("No transport configured".into()))?;

        let message = serde_json::to_value(&response)?;
        transport.send(message).await
    }

    pub async fn send_notification(&mut self, method: &str, params: Option<serde_json::Value>) -> Result<()> {
        let transport = self.transport.as_mut()
            .ok_or_else(|| ServerError::Transport("No transport configured".into()))?;

        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        };

        let message = serde_json::to_value(&notification)?;
        transport.send(message).await
    }

    fn register_default_handlers(&mut self) {
        // Initialize handler
        let server_info = self.server_info.clone();
        let capabilities = self.capabilities.clone();
        self.add_request_handler("initialize", move |request| {
            let _init_request: InitializeRequest =
                serde_json::from_value(request.params.unwrap_or_default())?;

            let response = InitializeResponse {
                protocol_version: "2024-11-05".to_string(),
                capabilities: capabilities.clone(),
                server_info: server_info.clone(),
            };

            Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(serde_json::to_value(response)?),
                error: None,
            })
        });

        // Tools list handler
        let tools = Arc::new(Mutex::new(self.tools.clone()));
        self.add_request_handler("tools/list", move |request| {
            let tools = tools.clone();
            tokio::task::block_in_place(|| {
                let tools = futures::executor::block_on(tools.lock());

                #[derive(serde::Serialize)]
                struct ToolsListResponse {
                    tools: Vec<Tool>,
                }

                let response = ToolsListResponse {
                    tools: tools.clone(),
                };

                Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(serde_json::to_value(response)?),
                    error: None,
                })
            })
        });

        // Ping handler
        self.add_request_handler("ping", |request| {
            Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(serde_json::json!({})),
                error: None,
            })
        });
    }

    pub fn add_tool(&mut self, tool: Tool) {
        self.tools.push(tool);
        self.capabilities.tools = Some(ToolsCapability {
            list_changed: false,
        });
    }

    pub async fn close(&mut self) -> Result<()> {
        if let Some(ref mut transport) = self.transport {
            transport.close().await?;
        }
        self.transport = None;
        Ok(())
    }
}
