use crate::error::{ClientError, Result};
use crate::transport::Transport;
use crate::types::*;
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

pub struct McpClient {
    transport: Option<Box<dyn Transport>>,
    client_info: ClientInfo,
    server_info: Option<ServerInfo>,
    server_capabilities: Option<ServerCapabilities>,
    pending_requests: HashMap<serde_json::Value, oneshot::Sender<JsonRpcResponse>>,
    notification_receiver: Option<mpsc::UnboundedReceiver<JsonRpcNotification>>,
}

impl McpClient {
    pub fn new(name: String, version: String) -> Self {
        Self {
            transport: None,
            client_info: ClientInfo { name, version },
            server_info: None,
            server_capabilities: None,
            pending_requests: HashMap::new(),
            notification_receiver: None,
        }
    }

    pub async fn connect(&mut self, transport: Box<dyn Transport>) -> Result<InitializeResponse> {
        self.transport = Some(transport);

        // Send initialize request
        let init_request = InitializeRequest {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: self.client_info.clone(),
        };

        let response = self.send_request("initialize", Some(serde_json::to_value(init_request)?)).await?;

        let init_response: InitializeResponse = serde_json::from_value(
            response.result.ok_or_else(|| ClientError::Protocol("No result in initialize response".into()))?
        )?;

        self.server_info = Some(init_response.server_info.clone());
        self.server_capabilities = Some(init_response.capabilities.clone());

        // Send initialized notification
        self.send_notification("notifications/initialized", None).await?;

        Ok(init_response)
    }

    pub async fn send_request(&mut self, method: &str, params: Option<serde_json::Value>) -> Result<JsonRpcResponse> {
        let id = serde_json::Value::String(Uuid::new_v4().to_string());

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: id.clone(),
            method: method.to_string(),
            params,
        };

        let message = serde_json::to_value(&request)?;

        // Send the request
        {
            let transport = self.transport.as_mut()
                .ok_or_else(|| ClientError::Transport("Not connected".into()))?;
            transport.send(message).await?;
        }

        // For simplicity, receive response immediately (blocking approach)
        let response_message = {
            let transport = self.transport.as_mut()
                .ok_or_else(|| ClientError::Transport("Not connected".into()))?;
            transport.receive().await?
        };

        if let Ok(response) = serde_json::from_value::<JsonRpcResponse>(response_message.clone()) {
            if response.id == id {
                return Ok(response);
            }
        }

        // Handle other message types
        self.handle_incoming_message(response_message).await?;

        Err(ClientError::Protocol("Did not receive expected response".into()))
    }

    pub async fn send_notification(&mut self, method: &str, params: Option<serde_json::Value>) -> Result<()> {
        let transport = self.transport.as_mut()
            .ok_or_else(|| ClientError::Transport("Not connected".into()))?;

        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        };

        let message = serde_json::to_value(&notification)?;
        transport.send(message).await?;

        Ok(())
    }

    pub async fn receive_message(&mut self) -> Result<()> {
        let message = {
            let transport = self.transport.as_mut()
                .ok_or_else(|| ClientError::Transport("Not connected".into()))?;
            transport.receive().await?
        };

        self.handle_incoming_message(message).await
    }

    async fn handle_incoming_message(&mut self, message: JsonRpcMessage) -> Result<()> {
        // Try to parse as response first
        if let Ok(response) = serde_json::from_value::<JsonRpcResponse>(message.clone()) {
            if let Some(tx) = self.pending_requests.remove(&response.id) {
                let _ = tx.send(response);
            }
            return Ok(());
        }

        // Try to parse as notification
        if let Ok(notification) = serde_json::from_value::<JsonRpcNotification>(message.clone()) {
            tracing::debug!("Received notification: {}", notification.method);
            return Ok(());
        }

        // Try to parse as request (server calling client)
        if let Ok(request) = serde_json::from_value::<JsonRpcRequest>(message) {
            self.handle_server_request(request).await?;
            return Ok(());
        }

        Err(ClientError::Protocol("Invalid message format".into()))
    }

    async fn handle_server_request(&mut self, request: JsonRpcRequest) -> Result<()> {
        let response = match request.method.as_str() {
            "ping" => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(serde_json::json!({})),
                error: None,
            },
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: "Method not found".to_string(),
                    data: None,
                }),
            },
        };

        let transport = self.transport.as_mut()
            .ok_or_else(|| ClientError::Transport("Not connected".into()))?;

        let message = serde_json::to_value(&response)?;
        transport.send(message).await?;

        Ok(())
    }

    pub async fn list_tools(&mut self) -> Result<Vec<Tool>> {
        let response = self.send_request("tools/list", None).await?;

        let result = response.result
            .ok_or_else(|| ClientError::Protocol("No result in tools/list response".into()))?;

        #[derive(serde::Deserialize)]
        struct ToolsListResponse {
            tools: Vec<Tool>,
        }

        let tools_response: ToolsListResponse = serde_json::from_value(result)?;
        Ok(tools_response.tools)
    }

    pub async fn call_tool(&mut self, name: &str, arguments: Option<HashMap<String, serde_json::Value>>) -> Result<CallToolResponse> {
        let request = CallToolRequest {
            name: name.to_string(),
            arguments,
        };

        let response = self.send_request("tools/call", Some(serde_json::to_value(request)?)).await?;

        let result = response.result
            .ok_or_else(|| ClientError::Protocol("No result in tools/call response".into()))?;

        let tool_response: CallToolResponse = serde_json::from_value(result)?;
        Ok(tool_response)
    }

    pub fn server_info(&self) -> Option<&ServerInfo> {
        self.server_info.as_ref()
    }

    pub fn server_capabilities(&self) -> Option<&ServerCapabilities> {
        self.server_capabilities.as_ref()
    }

    pub async fn close(&mut self) -> Result<()> {
        if let Some(ref mut transport) = self.transport {
            transport.close().await?;
        }
        self.transport = None;
        self.pending_requests.clear();
        Ok(())
    }
}
