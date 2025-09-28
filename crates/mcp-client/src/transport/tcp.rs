use crate::error::{ClientError, Result};
use crate::transport::{McpClientTransport, TransportConfig};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

pub struct TcpTransport {
    config: TransportConfig,
    stream: Arc<Mutex<Option<TcpStream>>>,
    connected: Arc<Mutex<bool>>,
}

impl TcpTransport {
    pub fn new(config: TransportConfig) -> Self {
        Self {
            config,
            stream: Arc::new(Mutex::new(None)),
            connected: Arc::new(Mutex::new(false)),
        }
    }

    async fn parse_address(&self) -> Result<std::net::SocketAddr> {
        // Parse endpoint as "host:port" or just "port" (defaults to localhost)
        let addr = if self.config.endpoint.contains(':') {
            self.config.endpoint.clone()
        } else {
            format!("127.0.0.1:{}", self.config.endpoint)
        };

        addr.parse()
            .map_err(|e| ClientError::Connection(format!("Invalid address '{}': {}", addr, e)))
    }
}

#[async_trait]
impl McpClientTransport for TcpTransport {
    async fn connect(&mut self) -> Result<()> {
        info!("Connecting to MCP server via TCP: {}", self.config.endpoint);

        let addr = self.parse_address().await?;

        for attempt in 1..=self.config.retry_attempts {
            match TcpStream::connect(addr).await {
                Ok(stream) => {
                    *self.stream.lock().await = Some(stream);
                    *self.connected.lock().await = true;
                    info!("Successfully connected to MCP server via TCP");
                    return Ok(());
                }
                Err(e) => {
                    warn!("Connection attempt {} failed: {}", attempt, e);
                    if attempt < self.config.retry_attempts {
                        tokio::time::sleep(self.config.retry_delay).await;
                    }
                }
            }
        }

        Err(ClientError::Connection(format!(
            "Failed to connect to {} after {} attempts",
            addr, self.config.retry_attempts
        )))
    }

    async fn send_request(&mut self, request: &str) -> Result<String> {
        if !self.is_connected().await {
            return Err(ClientError::Connection("Not connected".to_string()));
        }

        let mut stream_guard = self.stream.lock().await;
        let stream = stream_guard.as_mut()
            .ok_or_else(|| ClientError::Connection("No active connection".to_string()))?;

        debug!("Sending request: {}", request);

        // Send the request
        stream.write_all(request.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        stream.flush().await?;

        // Read the response
        let mut reader = BufReader::new(stream);
        let mut response = String::new();

        match tokio::time::timeout(self.config.timeout, reader.read_line(&mut response)).await {
            Ok(Ok(0)) => {
                error!("Connection closed by server");
                *self.connected.lock().await = false;
                Err(ClientError::Connection("Connection closed".to_string()))
            }
            Ok(Ok(_)) => {
                let response = response.trim().to_string();
                debug!("Received response: {}", response);
                Ok(response)
            }
            Ok(Err(e)) => {
                error!("IO error reading response: {}", e);
                *self.connected.lock().await = false;
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

        let mut stream_guard = self.stream.lock().await;
        if let Some(mut stream) = stream_guard.take() {
            let _ = stream.shutdown().await;
        }

        info!("Disconnected from MCP server");
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        *self.connected.lock().await
    }
}
