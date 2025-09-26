//! Direct JSON-RPC over STDIO proxy implementation
//!
//! This bypasses rmcp's serve_server function which has integration issues,
//! and implements direct MCP protocol handling over stdin/stdout.

use crate::error::{ProxyError, Result};
use crate::strategy::TransportStrategy;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, AsyncReadExt};
use tracing::{debug, info, warn, error};

/// Direct STDIO JSON-RPC proxy that handles MCP protocol manually
pub struct StdioProxy {
    server_url: String,
    transport_strategy: TransportStrategy,
    headers: Vec<String>,
}

impl StdioProxy {
    /// Create a new STDIO proxy
    pub fn new(server_url: String) -> Self {
        Self {
            server_url,
            transport_strategy: TransportStrategy::default(),
            headers: Vec::new(),
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
        info!("Starting direct STDIO MCP proxy for server: {}", self.server_url);

        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        info!("STDIO proxy ready - listening for MCP requests");

        loop {
            line.clear();
            debug!("Waiting for input...");

            // Try reading line with explicit handling
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    info!("End of input - shutting down proxy");
                    break;
                }
                Ok(bytes_read) => {
                    debug!("Read {} bytes from stdin: '{}'", bytes_read, line.trim());
                }
                Err(e) => {
                    return Err(ProxyError::Transport(format!("Failed to read from stdin: {}", e)));
                }
            }

            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }

            debug!("Received request: {}", line_trimmed);

            // Parse JSON-RPC request
            match serde_json::from_str::<Value>(line_trimmed) {
                Ok(request) => {
                    let response = self.handle_request(request).await;
                    let response_str = serde_json::to_string(&response)
                        .map_err(|e| ProxyError::Transport(format!("Failed to serialize response: {}", e)))?;

                    debug!("Sending response: {}", response_str);

                    stdout.write_all(response_str.as_bytes()).await
                        .map_err(|e| ProxyError::Transport(format!("Failed to write to stdout: {}", e)))?;
                    stdout.write_all(b"\n").await
                        .map_err(|e| ProxyError::Transport(format!("Failed to write newline: {}", e)))?;
                    stdout.flush().await
                        .map_err(|e| ProxyError::Transport(format!("Failed to flush stdout: {}", e)))?;
                }
                Err(e) => {
                    warn!("Failed to parse JSON-RPC request: {} - Input: {}", e, line_trimmed);

                    // Send JSON-RPC error response
                    let error_response = json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": {
                            "code": -32700,
                            "message": "Parse error",
                            "data": format!("Invalid JSON: {}", e)
                        }
                    });

                    if let Ok(error_str) = serde_json::to_string(&error_response) {
                        let _ = stdout.write_all(error_str.as_bytes()).await;
                        let _ = stdout.write_all(b"\n").await;
                        let _ = stdout.flush().await;
                    }
                }
            }
        }

        info!("STDIO proxy stopped");
        Ok(())
    }

    /// Handle a parsed JSON-RPC request
    async fn handle_request(&self, request: Value) -> Value {
        let method = request.get("method").and_then(|m| m.as_str());
        let id = request.get("id");
        let params = request.get("params");

        match method {
            Some("initialize") => self.handle_initialize(id, params).await,
            Some("ping") => self.handle_ping(id).await,
            Some("tools/list") => self.handle_list_tools(id, params).await,
            Some("tools/call") => self.handle_call_tool(id, params).await,
            Some("resources/list") => self.handle_list_resources(id, params).await,
            Some("prompts/list") => self.handle_list_prompts(id, params).await,
            Some("initialized") => {
                // Notification - no response needed
                info!("Client initialization complete");
                return json!(null);
            }
            Some(unknown_method) => {
                warn!("Unknown method: {}", unknown_method);
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": "Method not found",
                        "data": format!("Unknown method: {}", unknown_method)
                    }
                })
            }
            None => {
                warn!("Request missing method field");
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32600,
                        "message": "Invalid Request",
                        "data": "Missing method field"
                    }
                })
            }
        }
    }

    /// Handle initialize request
    async fn handle_initialize(&self, id: Option<&Value>, _params: Option<&Value>) -> Value {
        info!("Handling initialize request - TODO: forward to remote server {}", self.server_url);

        // For now, return proxy server info
        // TODO: Forward to actual remote server and return its capabilities
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {
                        "listChanged": false
                    },
                    "resources": {
                        "subscribe": false,
                        "listChanged": false
                    },
                    "prompts": {
                        "listChanged": false
                    }
                },
                "serverInfo": {
                    "name": "mcp-remote-proxy",
                    "version": "0.1.0",
                    "title": "MCP Remote Proxy",
                    "description": format!("Proxying to: {}", self.server_url)
                },
                "instructions": format!("Connected to remote server: {}", self.server_url)
            }
        })
    }

    /// Handle ping request
    async fn handle_ping(&self, id: Option<&Value>) -> Value {
        debug!("Handling ping request");
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        })
    }

    /// Handle tools/list request
    async fn handle_list_tools(&self, id: Option<&Value>, _params: Option<&Value>) -> Value {
        info!("Handling tools/list request - TODO: forward to remote server {}", self.server_url);

        // TODO: Forward to remote server and return its tools
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [],
                "_meta": {
                    "proxied_from": self.server_url.clone()
                }
            }
        })
    }

    /// Handle tools/call request
    async fn handle_call_tool(&self, id: Option<&Value>, params: Option<&Value>) -> Value {
        let tool_name = params
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown");

        info!("Handling tools/call request for '{}' - TODO: forward to remote server {}", tool_name, self.server_url);

        // TODO: Forward to remote server
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32603,
                "message": "Internal error",
                "data": "Remote tool execution not yet implemented"
            }
        })
    }

    /// Handle resources/list request
    async fn handle_list_resources(&self, id: Option<&Value>, _params: Option<&Value>) -> Value {
        info!("Handling resources/list request - TODO: forward to remote server {}", self.server_url);

        // TODO: Forward to remote server
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "resources": [],
                "_meta": {
                    "proxied_from": self.server_url.clone()
                }
            }
        })
    }

    /// Handle prompts/list request
    async fn handle_list_prompts(&self, id: Option<&Value>, _params: Option<&Value>) -> Value {
        info!("Handling prompts/list request - TODO: forward to remote server {}", self.server_url);

        // TODO: Forward to remote server
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "prompts": [],
                "_meta": {
                    "proxied_from": self.server_url.clone()
                }
            }
        })
    }
}
