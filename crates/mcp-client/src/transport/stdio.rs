use crate::error::{ClientError, Result};
use crate::transport::{McpClientTransport, TransportConfig};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

pub struct StdioTransport {
    config: TransportConfig,
    child: Mutex<Option<Child>>,
    connected: Mutex<bool>,
}

impl StdioTransport {
    pub fn new(config: TransportConfig) -> Self {
        Self {
            config,
            child: Mutex::new(None),
            connected: Mutex::new(false),
        }
    }

    async fn start_subprocess(&self) -> Result<Child> {
        // Parse the endpoint as a command
        // For stdio transport, endpoint should be like "command arg1 arg2"
        let parts: Vec<&str> = self.config.endpoint.split_whitespace().collect();
        if parts.is_empty() {
            return Err(ClientError::Connection("Empty command".to_string()));
        }

        let mut cmd = Command::new(parts[0]);
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }

        let child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ClientError::Connection(format!("Failed to start subprocess: {}", e)))?;

        info!("Started MCP server subprocess: {}", self.config.endpoint);
        Ok(child)
    }
}

#[async_trait]
impl McpClientTransport for StdioTransport {
    async fn connect(&mut self) -> Result<()> {
        info!("Connecting to MCP server via STDIO: {}", self.config.endpoint);

        let child = self.start_subprocess().await?;
        *self.child.lock().await = Some(child);
        *self.connected.lock().await = true;

        info!("Successfully connected to MCP server via STDIO");
        Ok(())
    }

    async fn send_request(&mut self, request: &str) -> Result<String> {
        if !self.is_connected().await {
            return Err(ClientError::Connection("Not connected".to_string()));
        }

        let mut child_guard = self.child.lock().await;
        let child = child_guard.as_mut()
            .ok_or_else(|| ClientError::Connection("No active subprocess".to_string()))?;

        let stdin = child.stdin.as_mut()
            .ok_or_else(|| ClientError::Connection("No stdin available".to_string()))?;

        let stdout = child.stdout.as_mut()
            .ok_or_else(|| ClientError::Connection("No stdout available".to_string()))?;

        debug!("Sending request: {}", request);

        // Send the request
        stdin.write_all(request.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        // Read the response
        let mut reader = BufReader::new(stdout);
        let mut response = String::new();

        match tokio::time::timeout(self.config.timeout, reader.read_line(&mut response)).await {
            Ok(Ok(0)) => {
                error!("Subprocess closed stdout");
                *self.connected.lock().await = false;
                Err(ClientError::Connection("Subprocess closed".to_string()))
            }
            Ok(Ok(_)) => {
                let response = response.trim().to_string();
                debug!("Received response: {}", response);
                Ok(response)
            }
            Ok(Err(e)) => {
                error!("IO error reading response: {}", e);
                Err(ClientError::Io(e))
            }
            Err(_) => {
                error!("Timeout waiting for response");
                Err(ClientError::Timeout)
            }
        }
    }

    async fn disconnect(&mut self) -> Result<()> {
        *self.connected.lock().await = false;

        let mut child_guard = self.child.lock().await;
        if let Some(mut child) = child_guard.take() {
            // Close stdin to signal the subprocess to exit
            drop(child.stdin.take());

            // Wait for the subprocess to exit or kill it after a timeout
            match tokio::time::timeout(std::time::Duration::from_secs(5), child.wait()).await {
                Ok(Ok(status)) => {
                    info!("Subprocess exited with status: {}", status);
                }
                Ok(Err(e)) => {
                    error!("Error waiting for subprocess: {}", e);
                }
                Err(_) => {
                    warn!("Subprocess did not exit within timeout, killing it");
                    let _ = child.kill().await;
                }
            }
        }

        info!("Disconnected from MCP server");
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        *self.connected.lock().await
    }
}
