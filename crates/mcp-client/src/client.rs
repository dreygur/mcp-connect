use crate::error::{ClientError, Result};
use crate::transport::{create_transport, McpClientTransport, TransportConfig};
use mcp_types::{McpClient, TransportType};
use rmcp::model::{
    ClientCapabilities, Implementation, InitializeRequestParam, InitializeResult, ProtocolVersion,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

pub struct McpRemoteClient {
    transports: Vec<(TransportType, TransportConfig)>,
    current_transport: Arc<Mutex<Option<Box<dyn McpClientTransport>>>>,
    current_transport_index: Arc<Mutex<usize>>,
    initialized: Arc<Mutex<bool>>,
    client_info: Implementation,
    capabilities: ClientCapabilities,
    request_id: Arc<Mutex<u64>>,
}

impl McpRemoteClient {
    pub fn new(primary_endpoint: String, fallback_transports: Vec<TransportType>) -> Self {
        let mut transports = vec![];

        // Primary transport (HTTP by default)
        let primary_config = TransportConfig {
            endpoint: primary_endpoint,
            ..Default::default()
        };
        transports.push((TransportType::Http, primary_config));

        // Add fallback transports
        for transport_type in fallback_transports {
            let config = match transport_type {
                TransportType::Stdio => TransportConfig {
                    endpoint: "mcp-server".to_string(), // Default server command
                    ..Default::default()
                },
                TransportType::Tcp => TransportConfig {
                    endpoint: "8080".to_string(), // Default port
                    ..Default::default()
                },
                TransportType::Http => continue, // Skip if already added as primary
            };
            transports.push((transport_type, config));
        }

        let client_info = Implementation {
            name: "mcp-remote-client".to_string(),
            version: "0.1.0".to_string(),
            title: None,
            icons: None,
            website_url: None,
        };

        let capabilities = ClientCapabilities::builder()
            .enable_experimental()
            .enable_roots()
            .enable_roots_list_changed()
            .build();

        Self {
            transports,
            current_transport: Arc::new(Mutex::new(None)),
            current_transport_index: Arc::new(Mutex::new(0)),
            initialized: Arc::new(Mutex::new(false)),
            client_info,
            capabilities,
            request_id: Arc::new(Mutex::new(1)),
        }
    }

    pub async fn with_custom_transports(transports: Vec<(TransportType, TransportConfig)>) -> Self {
        let client_info = Implementation {
            name: "mcp-remote-client".to_string(),
            version: "0.1.0".to_string(),
            title: None,
            icons: None,
            website_url: None,
        };

        let capabilities = ClientCapabilities::builder()
            .enable_experimental()
            .enable_roots()
            .enable_roots_list_changed()
            .build();

        Self {
            transports,
            current_transport: Arc::new(Mutex::new(None)),
            current_transport_index: Arc::new(Mutex::new(0)),
            initialized: Arc::new(Mutex::new(false)),
            client_info,
            capabilities,
            request_id: Arc::new(Mutex::new(1)),
        }
    }

    pub fn new_with_config(primary_config: TransportConfig, fallback_transports: Vec<TransportType>) -> Self {
        let mut transports = vec![];

        // Primary transport (HTTP by default)
        transports.push((TransportType::Http, primary_config));

        // Add fallback transports
        for transport_type in fallback_transports {
            let config = match transport_type {
                TransportType::Stdio => TransportConfig {
                    endpoint: "mcp-server".to_string(), // Default server command
                    ..Default::default()
                },
                TransportType::Tcp => TransportConfig {
                    endpoint: "8080".to_string(), // Default port
                    ..Default::default()
                },
                TransportType::Http => continue, // Skip if already added as primary
            };
            transports.push((transport_type, config));
        }

        let client_info = Implementation {
            name: "mcp-remote-client".to_string(),
            version: "0.1.0".to_string(),
            title: None,
            icons: None,
            website_url: None,
        };

        let capabilities = ClientCapabilities::builder()
            .enable_experimental()
            .enable_roots()
            .enable_roots_list_changed()
            .build();

        Self {
            transports,
            current_transport: Arc::new(Mutex::new(None)),
            current_transport_index: Arc::new(Mutex::new(0)),
            initialized: Arc::new(Mutex::new(false)),
            client_info,
            capabilities,
            request_id: Arc::new(Mutex::new(1)),
        }
    }

    async fn next_request_id(&self) -> u64 {
        let mut id = self.request_id.lock().await;
        *id += 1;
        *id
    }

    async fn try_connect_transport(&self, index: usize) -> Result<Box<dyn McpClientTransport>> {
        if index >= self.transports.len() {
            return Err(ClientError::Connection("No more transports to try".to_string()));
        }

        let (transport_type, config) = &self.transports[index];
        info!("Attempting to connect using {:?} transport", transport_type);

        let mut transport = create_transport(transport_type.clone(), config.clone()).await?;
        transport.connect().await?;

        Ok(transport)
    }

    async fn connect_with_fallbacks(&self) -> Result<()> {
        let current_index = *self.current_transport_index.lock().await;

        for i in current_index..self.transports.len() {
            match self.try_connect_transport(i).await {
                Ok(transport) => {
                    *self.current_transport.lock().await = Some(transport);
                    *self.current_transport_index.lock().await = i;
                    info!("Successfully connected using transport {}", i);
                    return Ok(());
                }
                Err(e) => {
                    warn!("Transport {} failed: {}", i, e);
                }
            }
        }

        Err(ClientError::Connection("All transports failed".to_string()))
    }

    async fn ensure_connected(&self) -> Result<()> {
        let transport_guard = self.current_transport.lock().await;
        if let Some(transport) = transport_guard.as_ref() {
            if transport.is_connected().await {
                return Ok(());
            }
        }
        drop(transport_guard);

        // Try to reconnect
        self.connect_with_fallbacks().await
    }

    async fn send_request_with_retry(&self, request: &str) -> Result<String> {
        const MAX_RETRY_ATTEMPTS: usize = 3;

        for attempt in 1..=MAX_RETRY_ATTEMPTS {
            self.ensure_connected().await?;

            let mut transport_guard = self.current_transport.lock().await;
            if let Some(transport) = transport_guard.as_mut() {
                match transport.send_request(request).await {
                    Ok(response) => return Ok(response),
                    Err(e) => {
                        error!("Request attempt {} failed: {}", attempt, e);
                        if attempt == MAX_RETRY_ATTEMPTS {
                            return Err(e);
                        }
                        // Mark transport as disconnected and try next transport
                        drop(transport_guard);
                        *self.current_transport.lock().await = None;

                        // Move to next transport for retry
                        let mut index_guard = self.current_transport_index.lock().await;
                        *index_guard = (*index_guard + 1) % self.transports.len();
                    }
                }
            } else {
                return Err(ClientError::Connection("No transport available".to_string()));
            }
        }

        Err(ClientError::Connection("All retry attempts failed".to_string()))
    }

    pub async fn initialize(&self) -> Result<InitializeResult> {
        let request_id = self.next_request_id().await;

        let request_params = InitializeRequestParam {
            protocol_version: ProtocolVersion::default(),
            capabilities: self.capabilities.clone(),
            client_info: self.client_info.clone(),
        };

        let json_request = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "initialize",
            "params": request_params
        });

        let request_str = json_request.to_string();
        info!("Sending initialization request: {}", request_str);
        let response = self.send_request_with_retry(&request_str).await?;
        info!("Received initialization response: {}", response);

        if response == "{}" || response.trim().is_empty() {
            // HTTP transport might return empty response for 202 Accepted
            warn!("Received empty response, assuming initialization succeeded");
            *self.initialized.lock().await = true;
            return Ok(InitializeResult {
                protocol_version: ProtocolVersion::default(),
                capabilities: Default::default(),
                server_info: Implementation {
                    name: "unknown".to_string(),
                    version: "unknown".to_string(),
                    title: None,
                    icons: None,
                    website_url: None,
                },
                instructions: None,
            });
        }

        let parsed: Value = serde_json::from_str(&response)
            .map_err(|e| ClientError::Json(e))?;

        if let Some(error) = parsed.get("error") {
            return Err(ClientError::Protocol(format!("Initialize error: {}", error)));
        }

        let result: InitializeResult = serde_json::from_value(
            parsed.get("result").unwrap_or(&Value::Null).clone()
        )?;

        *self.initialized.lock().await = true;
        info!("Successfully initialized MCP client");

        Ok(result)
    }

    pub async fn list_tools(&self) -> Result<Value> {
        if !*self.initialized.lock().await {
            return Err(ClientError::Protocol("Client not initialized".to_string()));
        }

        let request_id = self.next_request_id().await;
        let request = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "tools/list"
        });

        let response = self.send_request_with_retry(&request.to_string()).await?;

        if response == "{}" || response.trim().is_empty() {
            return Ok(json!({"tools": []}));
        }

        let parsed: Value = serde_json::from_str(&response)?;

        if let Some(error) = parsed.get("error") {
            return Err(ClientError::Protocol(format!("List tools error: {}", error)));
        }

        Ok(parsed.get("result").unwrap_or(&Value::Null).clone())
    }

    pub async fn list_resources(&self) -> Result<Value> {
        if !*self.initialized.lock().await {
            return Err(ClientError::Protocol("Client not initialized".to_string()));
        }

        let request_id = self.next_request_id().await;
        let request = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "resources/list"
        });

        let response = self.send_request_with_retry(&request.to_string()).await?;

        if response == "{}" || response.trim().is_empty() {
            return Ok(json!({"resources": []}));
        }

        let parsed: Value = serde_json::from_str(&response)?;

        if let Some(error) = parsed.get("error") {
            return Err(ClientError::Protocol(format!("List resources error: {}", error)));
        }

        Ok(parsed.get("result").unwrap_or(&Value::Null).clone())
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value> {
        if !*self.initialized.lock().await {
            return Err(ClientError::Protocol("Client not initialized".to_string()));
        }

        let request_id = self.next_request_id().await;
        let request = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": arguments
            }
        });

        let response = self.send_request_with_retry(&request.to_string()).await?;

        if response == "{}" || response.trim().is_empty() {
            return Err(ClientError::Protocol("Empty response for tool call".to_string()));
        }

        let parsed: Value = serde_json::from_str(&response)?;

        if let Some(error) = parsed.get("error") {
            return Err(ClientError::Protocol(format!("Tool call error: {}", error)));
        }

        Ok(parsed.get("result").unwrap_or(&Value::Null).clone())
    }

    pub async fn read_resource(&self, uri: &str) -> Result<Value> {
        if !*self.initialized.lock().await {
            return Err(ClientError::Protocol("Client not initialized".to_string()));
        }

        let request_id = self.next_request_id().await;
        let request = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "resources/read",
            "params": {
                "uri": uri
            }
        });

        let response = self.send_request_with_retry(&request.to_string()).await?;

        if response == "{}" || response.trim().is_empty() {
            return Err(ClientError::Protocol("Empty response for resource read".to_string()));
        }

        let parsed: Value = serde_json::from_str(&response)?;

        if let Some(error) = parsed.get("error") {
            return Err(ClientError::Protocol(format!("Resource read error: {}", error)));
        }

        Ok(parsed.get("result").unwrap_or(&Value::Null).clone())
    }
}

#[async_trait::async_trait]
impl McpClient for McpRemoteClient {
    async fn connect(&mut self) -> mcp_types::Result<()> {
        self.connect_with_fallbacks().await
            .map_err(|e| mcp_types::McpError::Connection(e.to_string()))
    }

    async fn send_request(&mut self, request: &str) -> mcp_types::Result<String> {
        self.send_request_with_retry(request).await
            .map_err(|e| mcp_types::McpError::Transport(e.to_string()))
    }

    async fn disconnect(&mut self) -> mcp_types::Result<()> {
        if let Some(mut transport) = self.current_transport.lock().await.take() {
            transport.disconnect().await
                .map_err(|e| mcp_types::McpError::Transport(e.to_string()))?;
        }
        *self.initialized.lock().await = false;
        Ok(())
    }
}
