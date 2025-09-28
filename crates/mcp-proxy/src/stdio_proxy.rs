use crate::error::{ProxyError, Result};
use crate::proxy::McpProxy;
use crate::strategy::ProxyStrategy;
use mcp_types::McpServer;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{error, info};

pub struct StdioMcpProxy {
    proxy: McpProxy,
    debug_mode: bool,
}

impl StdioMcpProxy {
    pub fn new(strategy: Arc<dyn ProxyStrategy>, debug_mode: bool) -> Self {
        Self {
            proxy: McpProxy::new(strategy),
            debug_mode,
        }
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting STDIO MCP Proxy");

        // Start the proxy
        self.proxy.start().await?;

        // Set up STDIO handling
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        info!("STDIO MCP Proxy ready, listening for messages");

        loop {
            line.clear();

            match reader.read_line(&mut line).await {
                Ok(0) => {
                    info!("EOF reached, shutting down proxy");
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    self.log_debug(&format!("Received: {}", trimmed));

                    match self.proxy.handle_message(trimmed).await {
                        Ok(Some(response)) => {
                            self.log_debug(&format!("Sending: {}", response));

                            if let Err(e) = stdout.write_all(response.as_bytes()).await {
                                error!("Failed to write response to stdout: {}", e);
                                break;
                            }
                            if let Err(e) = stdout.write_all(b"\n").await {
                                error!("Failed to write newline to stdout: {}", e);
                                break;
                            }
                            if let Err(e) = stdout.flush().await {
                                error!("Failed to flush stdout: {}", e);
                                break;
                            }
                        }
                        Ok(None) => {
                            self.log_debug("No response needed (notification)");
                        }
                        Err(e) => {
                            error!("Error handling message: {}", e);

                            // Try to send an error response if we can parse the request ID
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
                                if let Some(id) = parsed.get("id") {
                                    let error_response = serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "error": {
                                            "code": -32603,
                                            "message": format!("Proxy error: {}", e)
                                        }
                                    });

                                    let error_str = error_response.to_string();
                                    let _ = stdout.write_all(error_str.as_bytes()).await;
                                    let _ = stdout.write_all(b"\n").await;
                                    let _ = stdout.flush().await;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read from stdin: {}", e);
                    break;
                }
            }
        }

        // Shutdown the proxy
        self.proxy.shutdown().await?;
        info!("STDIO MCP Proxy shut down");
        Ok(())
    }

    fn log_debug(&self, message: &str) {
        if self.debug_mode {
            // In debug mode, write to stderr to avoid interfering with stdout protocol
            eprintln!("DEBUG: {}", message);
        }
    }
}

/// A combined server-proxy that acts as an MCP server but forwards requests to remote servers
pub struct CombinedStdioProxy {
    stdio_proxy: StdioMcpProxy,
}

impl CombinedStdioProxy {
    pub fn new(strategy: Arc<dyn ProxyStrategy>, debug_mode: bool) -> Self {
        Self {
            stdio_proxy: StdioMcpProxy::new(strategy, debug_mode),
        }
    }

    pub async fn run_as_server(&self) -> Result<()> {
        self.stdio_proxy.run().await
    }
}

#[async_trait::async_trait]
impl McpServer for CombinedStdioProxy {
    async fn start(&mut self) -> mcp_types::Result<()> {
        self.stdio_proxy.proxy.start().await
            .map_err(|e| mcp_types::McpError::Protocol(e.to_string()))
    }

    async fn handle_message(&mut self, message: &str) -> mcp_types::Result<Option<String>> {
        self.stdio_proxy.proxy.handle_message(message).await
            .map_err(|e| mcp_types::McpError::Protocol(e.to_string()))
    }

    async fn shutdown(&mut self) -> mcp_types::Result<()> {
        self.stdio_proxy.proxy.shutdown().await
            .map_err(|e| mcp_types::McpError::Protocol(e.to_string()))
    }
}

/// Builder for creating STDIO proxies with different configurations
pub struct StdioProxyBuilder {
    strategy: Option<Arc<dyn ProxyStrategy>>,
    debug_mode: bool,
}

impl StdioProxyBuilder {
    pub fn new() -> Self {
        Self {
            strategy: None,
            debug_mode: false,
        }
    }

    pub fn with_strategy(mut self, strategy: Arc<dyn ProxyStrategy>) -> Self {
        self.strategy = Some(strategy);
        self
    }

    pub fn with_debug_mode(mut self, debug: bool) -> Self {
        self.debug_mode = debug;
        self
    }

    pub fn build(self) -> Result<StdioMcpProxy> {
        let strategy = self.strategy
            .ok_or_else(|| ProxyError::Strategy("No strategy provided".to_string()))?;

        Ok(StdioMcpProxy::new(strategy, self.debug_mode))
    }

    pub fn build_combined(self) -> Result<CombinedStdioProxy> {
        let strategy = self.strategy
            .ok_or_else(|| ProxyError::Strategy("No strategy provided".to_string()))?;

        Ok(CombinedStdioProxy::new(strategy, self.debug_mode))
    }
}

impl Default for StdioProxyBuilder {
    fn default() -> Self {
        Self::new()
    }
}
