//! MCP Client - Official rmcp SDK Integration
//!
//! This crate provides a client interface for connecting to remote MCP servers
//! using the official rmcp Rust SDK. It wraps rmcp's transport capabilities
//! while maintaining compatibility with the existing codebase.

pub mod client;
pub mod error;
pub mod transport;
pub mod types;

// Re-export core client functionality
pub use client::{McpClient, RmcpClient};
pub use error::{ClientError, Result};
pub use transport::{Transport, TransportStrategy, RmcpTransportWrapper};
pub use types::*;

// Re-export rmcp types for convenience
pub use rmcp::service::{Service, ServiceExt};
pub use rmcp::ErrorData as McpError;

// Transport strategy enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    HttpFirst,
    SseFirst,
    HttpOnly,
    SseOnly,
}

impl Default for Strategy {
    fn default() -> Self {
        Strategy::HttpFirst
    }
}

// Client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub server_url: String,
    pub strategy: Strategy,
    pub headers: std::collections::HashMap<String, String>,
    pub timeout: Option<std::time::Duration>,
    pub allow_http: bool,
}

impl ClientConfig {
    pub fn new(server_url: impl Into<String>) -> Self {
        Self {
            server_url: server_url.into(),
            strategy: Strategy::default(),
            headers: std::collections::HashMap::new(),
            timeout: Some(std::time::Duration::from_secs(30)),
            allow_http: false,
        }
    }

    pub fn with_strategy(mut self, strategy: Strategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn allow_http(mut self) -> Self {
        self.allow_http = true;
        self
    }
}

// Convenience functions for creating clients
pub async fn create_client(config: ClientConfig) -> Result<RmcpClient> {
    RmcpClient::new(config).await
}

pub async fn create_http_client(url: impl Into<String>) -> Result<RmcpClient> {
    let config = ClientConfig::new(url).with_strategy(Strategy::HttpOnly);
    RmcpClient::new(config).await
}

pub async fn create_sse_client(url: impl Into<String>) -> Result<RmcpClient> {
    let config = ClientConfig::new(url).with_strategy(Strategy::SseOnly);
    RmcpClient::new(config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config() {
        let config = ClientConfig::new("http://localhost:8080")
            .with_strategy(Strategy::HttpFirst)
            .with_header("Authorization", "Bearer token")
            .with_timeout(std::time::Duration::from_secs(60))
            .allow_http();

        assert_eq!(config.server_url, "http://localhost:8080");
        assert_eq!(config.strategy, Strategy::HttpFirst);
        assert_eq!(config.headers.get("Authorization"), Some(&"Bearer token".to_string()));
        assert_eq!(config.timeout, Some(std::time::Duration::from_secs(60)));
        assert!(config.allow_http);
    }

    #[test]
    fn test_default_strategy() {
        assert_eq!(Strategy::default(), Strategy::HttpFirst);
    }

    #[test]
    fn test_client_config() {
        let config = ClientConfig::new("http://localhost:8080")
            .with_strategy(Strategy::HttpFirst)
            .with_header("Authorization", "Bearer token")
            .with_timeout(std::time::Duration::from_secs(60))
            .allow_http();

        assert_eq!(config.server_url, "http://localhost:8080");
        assert_eq!(config.strategy, Strategy::HttpFirst);
        assert_eq!(config.headers.get("Authorization"), Some(&"Bearer token".to_string()));
        assert_eq!(config.timeout, Some(std::time::Duration::from_secs(60)));
        assert!(config.allow_http);
    }
}
