use crate::error::{Result, ServerError};
use mcp_types::{LogLevel, LogMessage, McpServer};
use rmcp::model::{
    Implementation, InitializeResult, ServerCapabilities, InitializeRequestParam, ProtocolVersion,
};
use serde_json::{json, Value};
use std::io::{self, Write};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as AsyncBufReader};
use tokio::sync::mpsc;

pub struct McpStdioServer {
    debug_mode: bool,
    initialized: bool,
    client_info: Option<Implementation>,
    stdin: AsyncBufReader<tokio::io::Stdin>,
    stdout: tokio::io::Stdout,
    log_sender: Option<mpsc::UnboundedSender<LogMessage>>,
}

impl McpStdioServer {
    pub fn new(debug_mode: bool) -> Self {
        let stdin = AsyncBufReader::new(tokio::io::stdin());
        let stdout = tokio::io::stdout();

        Self {
            debug_mode,
            initialized: false,
            client_info: None,
            stdin,
            stdout,
            log_sender: None,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        self.setup_logging().await?;

        self.log_info("MCP STDIO Server starting").await;

        let mut line = String::new();
        loop {
            line.clear();

            match self.stdin.read_line(&mut line).await {
                Ok(0) => {
                    self.log_info("EOF reached, shutting down").await;
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    match self.handle_message(trimmed).await {
                        Ok(Some(response)) => {
                            self.send_response(&response).await?;
                        }
                        Ok(None) => {
                            // No response needed (notification)
                        }
                        Err(e) => {
                            self.log_error(&format!("Error handling message: {}", e)).await;
                            // Send error response if possible
                            if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                                if let Some(id) = parsed.get("id") {
                                    let error_response = json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "error": {
                                            "code": -32603,
                                            "message": format!("Internal error: {}", e)
                                        }
                                    });
                                    let _ = self.send_response(&error_response.to_string()).await;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    self.log_error(&format!("Failed to read from stdin: {}", e)).await;
                    return Err(ServerError::Io(e));
                }
            }
        }

        Ok(())
    }

    async fn setup_logging(&mut self) -> Result<()> {
        if !self.debug_mode {
            let (tx, mut rx) = mpsc::unbounded_channel();
            self.log_sender = Some(tx);

            // Spawn a task to handle log notifications
            tokio::spawn(async move {
                while let Some(log_msg) = rx.recv().await {
                    // Send notification to stderr (not stdout to avoid interfering with MCP protocol)
                    eprintln!("{}: {}", log_msg.level, log_msg.message);
                }
            });
        }
        Ok(())
    }

    async fn send_response(&mut self, response: &str) -> Result<()> {
        self.stdout.write_all(response.as_bytes()).await?;
        self.stdout.write_all(b"\n").await?;
        self.stdout.flush().await?;
        Ok(())
    }

    async fn log_message(&self, level: LogLevel, message: &str) {
        if self.debug_mode {
            // In debug mode, write to stdout as MCP notifications
            let notification = json!({
                "jsonrpc": "2.0",
                "method": "notifications/message",
                "params": {
                    "level": level,
                    "logger": "mcp-server",
                    "data": message
                }
            });

            // We can't use self.send_response here due to borrowing issues
            // So we write directly to stdout
            if let Ok(json_str) = serde_json::to_string(&notification) {
                print!("{}\n", json_str);
                let _ = io::stdout().flush();
            }
        } else if let Some(sender) = &self.log_sender {
            let log_msg = LogMessage {
                level,
                message: message.to_string(),
                timestamp: None, // No timestamp as per requirements
            };
            let _ = sender.send(log_msg);
        }
    }

    async fn log_debug(&self, message: &str) {
        self.log_message(LogLevel::Debug, message).await;
    }

    async fn log_info(&self, message: &str) {
        self.log_message(LogLevel::Info, message).await;
    }

    async fn log_warn(&self, message: &str) {
        self.log_message(LogLevel::Warn, message).await;
    }

    async fn log_error(&self, message: &str) {
        self.log_message(LogLevel::Error, message).await;
    }

    fn handle_initialize_request(&mut self, params: Value, id: Value) -> Result<String> {
        let init_params: InitializeRequestParam = serde_json::from_value(params)
            .map_err(|e| ServerError::InvalidMessage(e.to_string()))?;

        self.client_info = Some(init_params.client_info);
        self.initialized = true;

        let server_info = Implementation {
            name: "mcp-stdio-server".to_string(),
            version: "0.1.0".to_string(),
            title: None,
            icons: None,
            website_url: None,
        };

        let capabilities = ServerCapabilities::builder()
            .enable_logging()
            .enable_tools()
            .enable_resources()
            .build();

        let result = InitializeResult {
            protocol_version: ProtocolVersion::default(),
            capabilities,
            server_info,
            instructions: Some("MCP STDIO Server ready".to_string()),
        };

        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        });

        Ok(response.to_string())
    }

    fn handle_ping_request(&self, id: Value) -> Result<String> {
        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        });
        Ok(response.to_string())
    }

    fn handle_list_tools_request(&self, id: Value) -> Result<String> {
        // Return empty tools list for now
        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": []
            }
        });
        Ok(response.to_string())
    }

    fn handle_list_resources_request(&self, id: Value) -> Result<String> {
        // Return empty resources list for now
        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "resources": []
            }
        });
        Ok(response.to_string())
    }
}

#[async_trait::async_trait]
impl McpServer for McpStdioServer {
    async fn start(&mut self) -> mcp_types::Result<()> {
        self.run().await.map_err(|e| mcp_types::McpError::Protocol(e.to_string()))
    }

    async fn handle_message(&mut self, message: &str) -> mcp_types::Result<Option<String>> {
        self.log_debug(&format!("Received message: {}", message)).await;

        let parsed: Value = serde_json::from_str(message)
            .map_err(|e| mcp_types::McpError::Serialization(e))?;

        // Check if it's a notification (no id field)
        if parsed.get("id").is_none() {
            self.log_debug("Received notification, no response needed").await;
            return Ok(None);
        }

        let id = parsed["id"].clone();
        let method = parsed.get("method")
            .and_then(|m| m.as_str())
            .ok_or_else(|| mcp_types::McpError::Protocol("Missing method field".to_string()))?;

        let response = match method {
            "initialize" => {
                let params = parsed.get("params").cloned().unwrap_or(Value::Null);
                self.handle_initialize_request(params, id)
                    .map_err(|e| mcp_types::McpError::Protocol(e.to_string()))?
            }
            "ping" => {
                self.handle_ping_request(id)
                    .map_err(|e| mcp_types::McpError::Protocol(e.to_string()))?
            }
            "tools/list" => {
                self.handle_list_tools_request(id)
                    .map_err(|e| mcp_types::McpError::Protocol(e.to_string()))?
            }
            "resources/list" => {
                self.handle_list_resources_request(id)
                    .map_err(|e| mcp_types::McpError::Protocol(e.to_string()))?
            }
            _ => {
                let error_response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {}", method)
                    }
                });
                error_response.to_string()
            }
        };

        self.log_debug(&format!("Sending response: {}", response)).await;
        Ok(Some(response))
    }

    async fn shutdown(&mut self) -> mcp_types::Result<()> {
        self.log_info("Server shutting down").await;
        Ok(())
    }
}
