use crate::error::{ServerError, Result};
use crate::transport::Transport;
use crate::types::JsonRpcMessage;
use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

pub struct StdioTransport {
    stdin_receiver: mpsc::UnboundedReceiver<String>,
    connected: bool,
}

impl StdioTransport {
    pub fn new() -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();

        // Spawn a task to read from stdin
        tokio::spawn(async move {
            let stdin = tokio::io::stdin();
            let reader = BufReader::new(stdin);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }

                if tx.send(line).is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            stdin_receiver: rx,
            connected: true,
        })
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send(&mut self, message: JsonRpcMessage) -> Result<()> {
        if !self.connected {
            return Err(ServerError::ConnectionClosed);
        }

        let json_str = serde_json::to_string(&message)?;

        // Write to stdout with newline
        let mut stdout = tokio::io::stdout();
        stdout.write_all(json_str.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;

        Ok(())
    }

    async fn receive(&mut self) -> Result<JsonRpcMessage> {
        if !self.connected {
            return Err(ServerError::ConnectionClosed);
        }

        let line = self.stdin_receiver.recv().await
            .ok_or_else(|| ServerError::Transport("Stdin closed".into()))?;

        let message: JsonRpcMessage = serde_json::from_str(&line)?;
        Ok(message)
    }

    async fn close(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new().expect("Failed to create STDIO transport")
    }
}
