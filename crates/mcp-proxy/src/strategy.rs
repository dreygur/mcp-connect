//! Transport strategy for connecting to remote MCP servers

use crate::error::{ProxyError, Result};
use mcp_client::{HttpTransport, SseTransport, Transport};

#[derive(Debug, Clone)]
pub enum TransportStrategy {
    HttpFirst,
    SseFirst,
    HttpOnly,
    SseOnly,
}

#[derive(Debug, Clone)]
pub enum TransportType {
    Http,
    Sse,
}

impl Default for TransportStrategy {
    fn default() -> Self {
        TransportStrategy::HttpFirst
    }
}

pub async fn create_remote_transport(
    server_url: &str,
    strategy: TransportStrategy,
) -> Result<(Box<dyn Transport>, TransportType)> {
    match strategy {
        TransportStrategy::HttpFirst => {
            // Try HTTP first
            match HttpTransport::new(server_url) {
                Ok(mut transport) => {
                    if transport.connect().await.is_ok() {
                        return Ok((Box::new(transport), TransportType::Http));
                    }
                }
                Err(_) => {}
            }

            // Fallback to SSE
            match SseTransport::new(server_url) {
                Ok(mut transport) => {
                    transport.connect().await?;
                    Ok((Box::new(transport), TransportType::Sse))
                }
                Err(e) => Err(ProxyError::ConnectionFailed(format!(
                    "Failed to connect via both HTTP and SSE: {}",
                    e
                ))),
            }
        }
        TransportStrategy::SseFirst => {
            // Try SSE first
            match SseTransport::new(server_url) {
                Ok(mut transport) => {
                    if transport.connect().await.is_ok() {
                        return Ok((Box::new(transport), TransportType::Sse));
                    }
                }
                Err(_) => {}
            }

            // Fallback to HTTP
            match HttpTransport::new(server_url) {
                Ok(mut transport) => {
                    transport.connect().await?;
                    Ok((Box::new(transport), TransportType::Http))
                }
                Err(e) => Err(ProxyError::ConnectionFailed(format!(
                    "Failed to connect via both SSE and HTTP: {}",
                    e
                ))),
            }
        }
        TransportStrategy::HttpOnly => {
            let mut transport = HttpTransport::new(server_url)?;
            transport.connect().await?;
            Ok((Box::new(transport), TransportType::Http))
        }
        TransportStrategy::SseOnly => {
            let mut transport = SseTransport::new(server_url)?;
            transport.connect().await?;
            Ok((Box::new(transport), TransportType::Sse))
        }
    }
}
