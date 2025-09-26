//! Transport module for MCP client with rmcp integration
//!
//! This module provides transport abstractions and implementations for connecting
//! to remote MCP servers. It wraps the official rmcp SDK transports while maintaining
//! backward compatibility with the existing transport trait.

use crate::error::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, info};

// Note: HTTP and SSE specific implementations have been moved to the main client
// This module now provides the common abstractions and strategy types

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

/// Simple wrapper that demonstrates rmcp integration patterns
///
/// In a full implementation, this would wrap rmcp's transport types
/// and provide the unified interface expected by the proxy layer.
pub struct RmcpTransportWrapper {
    strategy: TransportStrategy,
    url: String,
    headers: HashMap<String, String>,
    connected: bool,
}

impl RmcpTransportWrapper {
    /// Create a new transport wrapper with HTTP strategy
    pub async fn new_http(
        url: impl Into<String>,
        headers: HashMap<String, String>,
    ) -> Result<Self> {
        let url = url.into();
        debug!("Creating HTTP transport for {}", url);

        Ok(Self {
            strategy: TransportStrategy::HttpOnly,
            url,
            headers,
            connected: false,
        })
    }

    /// Create a new transport wrapper with SSE strategy
    pub async fn new_sse(url: impl Into<String>) -> Result<Self> {
        let url = url.into();
        debug!("Creating SSE transport for {}", url);

        Ok(Self {
            strategy: TransportStrategy::SseOnly,
            url,
            headers: HashMap::new(),
            connected: false,
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

        Ok(Self {
            strategy,
            url,
            headers,
            connected: false,
        })
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
        // In a full rmcp implementation, this would check the actual connection
        debug!("Health check: connected = {}", self.connected);
        self.connected
    }

    /// Close the transport connection
    pub async fn close(mut self) -> Result<()> {
        info!("Closing transport connection");
        self.connected = false;
        Ok(())
    }

    /// Connect the transport (placeholder for rmcp integration)
    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting transport to {}", self.url);
        // In a full rmcp implementation, this would:
        // 1. Create appropriate rmcp transport (HTTP or SSE)
        // 2. Establish the connection
        // 3. Store the service handle for later use
        self.connected = true;
        Ok(())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected
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
    async fn test_transport_creation() {
        let headers = HashMap::new();

        // Test HTTP transport creation
        let result = RmcpTransportWrapper::new_http("https://example.com", headers.clone()).await;
        assert!(result.is_ok());
        let transport = result.unwrap();
        assert_eq!(transport.strategy(), TransportStrategy::HttpOnly);
        assert_eq!(transport.url(), "https://example.com");
        assert!(!transport.is_connected());

        // Test SSE transport creation
        let result = RmcpTransportWrapper::new_sse("https://example.com").await;
        assert!(result.is_ok());
        let transport = result.unwrap();
        assert_eq!(transport.strategy(), TransportStrategy::SseOnly);

        // Test strategy-based creation
        let result = RmcpTransportWrapper::new_with_strategy(
            "https://example.com",
            TransportStrategy::HttpFirst,
            headers,
        ).await;
        assert!(result.is_ok());
        let transport = result.unwrap();
        assert_eq!(transport.strategy(), TransportStrategy::HttpFirst);
    }

    #[tokio::test]
    async fn test_transport_lifecycle() {
        let headers = HashMap::new();
        let mut transport = RmcpTransportWrapper::new_http("https://example.com", headers).await.unwrap();

        // Initially not connected
        assert!(!transport.is_connected());
        assert!(!transport.health_check().await);

        // Connect
        transport.connect().await.unwrap();
        assert!(transport.is_connected());
        assert!(transport.health_check().await);

        // Close
        transport.close().await.unwrap();
    }
}
