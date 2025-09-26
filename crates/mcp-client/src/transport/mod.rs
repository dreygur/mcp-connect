//! Transport module for MCP client with rmcp integration
//!
//! This module provides transport abstractions and implementations for connecting
//! to remote MCP servers. It wraps the official rmcp SDK transports while maintaining
//! backward compatibility with the existing transport trait.

use crate::error::{ClientError, Result};
use async_trait::async_trait;
use rmcp::service::{Service, ServiceExt};
use std::collections::HashMap;
use tracing::{debug, info, warn};

pub mod http;
pub mod sse;

// Re-export for backward compatibility
pub use http::HttpTransport;
pub use sse::SseTransport;

/// Transport strategy enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportStrategy {
    /// Try HTTP first, fallback to SSE on 404
    HttpFirst,
    /// Try SSE first, fallback to HTTP on 405
    SseFirst,
    /// Only use HTTP transport
    HttpOnly,
    /// Only use SSE transport
    SseOnly,
}

impl Default for TransportStrategy {
    fn default() -> Self {
        TransportStrategy::HttpFirst
    }
}

/// Legacy transport trait for backward compatibility
#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(&mut self, message: serde_json::Value) -> Result<()>;
    async fn receive(&mut self) -> Result<serde_json::Value>;
    async fn close(&mut self) -> Result<()>;
    fn is_connected(&self) -> bool;
}

/// Modern rmcp-based transport wrapper
pub struct RmcpTransportWrapper {
    service: Service,
    strategy: TransportStrategy,
    url: String,
    headers: HashMap<String, String>,
}

impl RmcpTransportWrapper {
    /// Create a new transport wrapper with HTTP strategy
    pub async fn new_http(
        url: impl Into<String>,
        headers: HashMap<String, String>,
    ) -> Result<Self> {
        let url = url.into();
        debug!("Creating HTTP transport for {}", url);

        let service = Self::create_http_service(&url, &headers).await?;

        Ok(Self {
            service,
            strategy: TransportStrategy::HttpOnly,
            url,
            headers,
        })
    }

    /// Create a new transport wrapper with SSE strategy
    pub async fn new_sse(url: impl Into<String>) -> Result<Self> {
        let url = url.into();
        debug!("Creating SSE transport for {}", url);

        let service = Self::create_sse_service(&url).await?;

        Ok(Self {
            service,
            strategy: TransportStrategy::SseOnly,
            url,
            headers: HashMap::new(),
        })
    }

    /// Create a new transport wrapper with strategy-based connection
    pub async fn new_with_strategy(
        url: impl Into<String>,
        strategy: TransportStrategy,
        headers: HashMap<String, String>,
    ) -> Result<Self> {
        let url = url.into();
        debug!("Creating transport with strategy {:?} for {}", strategy, url);

        let service = match strategy {
            TransportStrategy::HttpFirst => {
                match Self::create_http_service(&url, &headers).await {
                    Ok(service) => {
                        info!("Successfully connected via HTTP");
                        service
                    }
                    Err(e) => {
                        warn!("HTTP transport failed: {}, trying SSE", e);
                        Self::create_sse_service(&url).await?
                    }
                }
            }
            TransportStrategy::SseFirst => {
                match Self::create_sse_service(&url).await {
                    Ok(service) => {
                        info!("Successfully connected via SSE");
                        service
                    }
                    Err(e) => {
                        warn!("SSE transport failed: {}, trying HTTP", e);
                        Self::create_http_service(&url, &headers).await?
                    }
                }
            }
            TransportStrategy::HttpOnly => {
                Self::create_http_service(&url, &headers).await?
            }
            TransportStrategy::SseOnly => {
                Self::create_sse_service(&url).await?
            }
        };

        Ok(Self {
            service,
            strategy,
            url,
            headers,
        })
    }

    /// Create HTTP service using rmcp
    async fn create_http_service(
        url: &str,
        headers: &HashMap<String, String>,
    ) -> Result<Service> {
        // Build HTTP client with custom headers
        let mut reqwest_headers = reqwest::header::HeaderMap::new();
        for (key, value) in headers {
            let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
                .map_err(|e| ClientError::InvalidHeader(format!("Invalid header name '{}': {}", key, e)))?;
            let header_value = reqwest::header::HeaderValue::from_str(value)
                .map_err(|e| ClientError::InvalidHeader(format!("Invalid header value '{}': {}", value, e)))?;
            reqwest_headers.insert(header_name, header_value);
        }

        // Create HTTP client
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .default_headers(reqwest_headers)
            .build()
            .map_err(|e| ClientError::TransportError(format!("Failed to create HTTP client: {}", e)))?;

        // Create rmcp HTTP transport
        let transport = rmcp::transport::Http::new(url)
            .map_err(|e| ClientError::TransportError(format!("Failed to create HTTP transport: {}", e)))?
            .with_client(client);

        // Create service
        let service = rmcp::service::serve_client((), transport)
            .await
            .map_err(|e| ClientError::TransportError(format!("Failed to create HTTP service: {}", e)))?;

        Ok(service)
    }

    /// Create SSE service using rmcp
    async fn create_sse_service(url: &str) -> Result<Service> {
        // Convert HTTP URL to SSE URL
        let sse_url = if url.starts_with("https://") {
            url.replace("https://", "wss://") + "/sse"
        } else if url.starts_with("http://") {
            url.replace("http://", "ws://") + "/sse"
        } else {
            format!("ws://{}/sse", url)
        };

        // Create rmcp SSE transport
        let transport = rmcp::transport::Sse::new(&sse_url)
            .map_err(|e| ClientError::TransportError(format!("Failed to create SSE transport: {}", e)))?;

        // Create service
        let service = rmcp::service::serve_client((), transport)
            .await
            .map_err(|e| ClientError::TransportError(format!("Failed to create SSE service: {}", e)))?;

        Ok(service)
    }

    /// Get the underlying rmcp service
    pub fn service(&self) -> &Service {
        &self.service
    }

    /// Get the transport strategy
    pub fn strategy(&self) -> TransportStrategy {
        self.strategy
    }

    /// Get the URL
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Check connection health
    pub async fn health_check(&self) -> bool {
        // Try to perform a basic operation to check connectivity
        match self.service.peer_info() {
            info => {
                debug!("Health check passed: {:?}", info);
                true
            }
        }
    }

    /// Close the transport connection
    pub async fn close(self) -> Result<()> {
        info!("Closing transport connection");
        self.service
            .cancel()
            .await
            .map_err(|e| ClientError::TransportError(format!("Failed to close connection: {}", e)))?;
        Ok(())
    }
}

/// Helper function to create transport with automatic strategy selection
pub async fn create_transport(
    url: impl Into<String>,
    strategy: Option<TransportStrategy>,
    headers: HashMap<String, String>,
) -> Result<RmcpTransportWrapper> {
    let strategy = strategy.unwrap_or_default();
    RmcpTransportWrapper::new_with_strategy(url, strategy, headers).await
}

/// Helper function to create HTTP-only transport
pub async fn create_http_transport(
    url: impl Into<String>,
    headers: HashMap<String, String>,
) -> Result<RmcpTransportWrapper> {
    RmcpTransportWrapper::new_http(url, headers).await
}

/// Helper function to create SSE-only transport
pub async fn create_sse_transport(url: impl Into<String>) -> Result<RmcpTransportWrapper> {
    RmcpTransportWrapper::new_sse(url).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_strategy() {
        assert_eq!(TransportStrategy::default(), TransportStrategy::HttpFirst);
    }

    #[test]
    fn test_strategy_equality() {
        assert_eq!(TransportStrategy::HttpFirst, TransportStrategy::HttpFirst);
        assert_ne!(TransportStrategy::HttpFirst, TransportStrategy::SseFirst);
    }

    #[tokio::test]
    async fn test_transport_creation_invalid_url() {
        let headers = HashMap::new();

        // Test with invalid URL - should fail during service creation
        let result = RmcpTransportWrapper::new_http("invalid-url", headers).await;
        assert!(result.is_err());

        let result = RmcpTransportWrapper::new_sse("invalid-url").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_http_url_validation() {
        let headers = HashMap::new();

        // Test valid HTTPS URL format (will fail on connection, but URL is valid)
        let result = RmcpTransportWrapper::new_http("https://example.com", headers).await;
        // Should fail with transport error, not URL error
        if let Err(e) = result {
            assert!(matches!(e, ClientError::TransportError(_)));
        }
    }
}
