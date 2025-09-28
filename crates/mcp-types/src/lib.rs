//! # MCP Types
//!
//! Core types, traits, and error definitions for the Model Context Protocol (MCP) remote proxy system.
//!
//! This crate provides the foundational building blocks used across all MCP components:
//! - Common error types and result handling
//! - Configuration structures
//! - Transport and protocol traits
//! - Logging utilities
//!
//! ## Features
//!
//! - **Unified Error Handling**: Comprehensive error types for all MCP operations
//! - **Transport Abstraction**: Generic traits for different transport mechanisms
//! - **Configuration**: Structured configuration for proxy components
//! - **Async Support**: All traits are designed for async/await usage
//!
//! ## Usage
//!
//! ```rust
//! use mcp_types::{McpError, Result, TransportType, LogLevel};
//!
//! // Handle MCP operations with unified error types
//! fn example_operation() -> Result<()> {
//!     // Your MCP operation here
//!     Ok(())
//! }
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Comprehensive error type for all MCP operations.
///
/// This enum covers all possible error conditions that can occur during
/// MCP proxy operations, from transport-level issues to authentication failures.
///
/// # Examples
///
/// ```rust
/// use mcp_types::{McpError, Result};
///
/// fn might_fail() -> Result<String> {
///     Err(McpError::Connection("Server unreachable".to_string()))
/// }
/// ```
#[derive(Error, Debug)]
pub enum McpError {
    /// Transport-level errors (network, protocol issues)
    #[error("Transport error: {0}")]
    Transport(String),

    /// MCP protocol-specific errors
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// JSON serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O operation errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Connection establishment or maintenance errors
    #[error("Connection error: {0}")]
    Connection(String),

    /// Operation timeout errors
    #[error("Timeout error")]
    Timeout,

    /// Authentication and authorization errors
    #[error("Authentication error: {0}")]
    Auth(String),
}

/// Convenient Result type alias for MCP operations.
///
/// This type alias simplifies error handling throughout the MCP codebase
/// by providing a standard Result type that uses [`McpError`] as the error type.
pub type Result<T> = std::result::Result<T, McpError>;

/// Structured log message for MCP operations.
///
/// Used for consistent logging across all MCP components, supporting
/// both debug and production logging modes.
///
/// # Examples
///
/// ```rust
/// use mcp_types::{LogMessage, LogLevel};
///
/// let log = LogMessage {
///     level: LogLevel::Info,
///     message: "Connection established".to_string(),
///     timestamp: Some("2024-01-01T12:00:00Z".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMessage {
    /// Log level (debug, info, warn, error)
    pub level: LogLevel,
    /// The actual log message content
    pub message: String,
    /// Optional timestamp (ISO 8601 format recommended)
    pub timestamp: Option<String>,
}

/// Log level enumeration for structured logging.
///
/// Supports standard log levels with serde serialization for
/// JSON-RPC message integration.
///
/// # Examples
///
/// ```rust
/// use mcp_types::LogLevel;
///
/// let level = LogLevel::Info;
/// println!("Current level: {}", level); // Prints "info"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    /// Debug-level messages for detailed troubleshooting
    #[serde(rename = "debug")]
    Debug,
    /// Informational messages for normal operations
    #[serde(rename = "info")]
    Info,
    /// Warning messages for potentially problematic situations
    #[serde(rename = "warn")]
    Warn,
    /// Error messages for failure conditions
    #[serde(rename = "error")]
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

/// Configuration structure for MCP proxy operations.
///
/// Contains settings that control how the proxy behaves, including
/// debug modes, target endpoints, and fallback strategies.
///
/// # Examples
///
/// ```rust
/// use mcp_types::{ProxyConfig, TransportType};
///
/// let config = ProxyConfig {
///     server_debug: true,
///     client_endpoint: "https://api.example.com/mcp".to_string(),
///     fallback_transports: vec![TransportType::Stdio, TransportType::Tcp],
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Enable debug logging for the MCP server component
    pub server_debug: bool,
    /// Target endpoint URL for the remote MCP server
    pub client_endpoint: String,
    /// List of transport types to try if the primary connection fails
    pub fallback_transports: Vec<TransportType>,
}

/// Available transport mechanisms for MCP communication.
///
/// Each transport type provides a different method of connecting to
/// remote MCP servers, with different trade-offs in terms of performance,
/// compatibility, and deployment requirements.
///
/// # Examples
///
/// ```rust
/// use mcp_types::TransportType;
///
/// let primary = TransportType::Http;
/// let fallback = vec![TransportType::Stdio, TransportType::Tcp];
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportType {
    /// Standard I/O transport for subprocess-based MCP servers
    #[serde(rename = "stdio")]
    Stdio,
    /// HTTP transport for web-based MCP servers (most common)
    #[serde(rename = "http")]
    Http,
    /// Direct TCP socket transport for high-performance local connections
    #[serde(rename = "tcp")]
    Tcp,
}

/// Generic transport trait for MCP message communication.
///
/// This trait abstracts the underlying transport mechanism (HTTP, STDIO, TCP)
/// and provides a uniform interface for sending and receiving MCP messages.
///
/// # Examples
///
/// ```rust
/// use mcp_types::{McpTransport, Result};
/// # use async_trait::async_trait;
///
/// struct MyTransport;
///
/// #[async_trait]
/// impl McpTransport for MyTransport {
///     async fn send_message(&mut self, message: &str) -> Result<()> {
///         // Transport-specific message sending logic
///         Ok(())
///     }
///
///     async fn receive_message(&mut self) -> Result<String> {
///         // Transport-specific message receiving logic
///         Ok("response".to_string())
///     }
///
///     async fn close(&mut self) -> Result<()> {
///         // Cleanup logic
///         Ok(())
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait McpTransport: Send + Sync {
    /// Send a message through this transport.
    ///
    /// # Arguments
    /// * `message` - The JSON-RPC message to send
    ///
    /// # Errors
    /// Returns [`McpError`] if the message cannot be sent due to transport issues.
    async fn send_message(&mut self, message: &str) -> Result<()>;

    /// Receive a message from this transport.
    ///
    /// # Returns
    /// The received JSON-RPC message as a string.
    ///
    /// # Errors
    /// Returns [`McpError`] if no message can be received or transport fails.
    async fn receive_message(&mut self) -> Result<String>;

    /// Close the transport connection cleanly.
    ///
    /// # Errors
    /// Returns [`McpError`] if the connection cannot be closed properly.
    async fn close(&mut self) -> Result<()>;
}

/// MCP server trait for handling local client connections.
///
/// Represents the local side of the proxy that communicates with
/// MCP clients (like Claude Desktop) via STDIO.
///
/// # Examples
///
/// ```rust
/// use mcp_types::{McpServer, Result};
/// # use async_trait::async_trait;
///
/// struct MyServer;
///
/// #[async_trait]
/// impl McpServer for MyServer {
///     async fn start(&mut self) -> Result<()> {
///         // Server initialization logic
///         Ok(())
///     }
///
///     async fn handle_message(&mut self, message: &str) -> Result<Option<String>> {
///         // Process incoming message and optionally return response
///         Ok(Some("response".to_string()))
///     }
///
///     async fn shutdown(&mut self) -> Result<()> {
///         // Cleanup logic
///         Ok(())
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait McpServer: Send + Sync {
    /// Start the MCP server and begin accepting connections.
    ///
    /// # Errors
    /// Returns [`McpError`] if the server cannot be started.
    async fn start(&mut self) -> Result<()>;

    /// Handle an incoming MCP message from a client.
    ///
    /// # Arguments
    /// * `message` - The JSON-RPC message received from the client
    ///
    /// # Returns
    /// Optionally returns a response message. `None` indicates no response needed.
    ///
    /// # Errors
    /// Returns [`McpError`] if the message cannot be processed.
    async fn handle_message(&mut self, message: &str) -> Result<Option<String>>;

    /// Shut down the server cleanly.
    ///
    /// # Errors
    /// Returns [`McpError`] if shutdown cannot complete properly.
    async fn shutdown(&mut self) -> Result<()>;
}

/// MCP client trait for connecting to remote servers.
///
/// Represents the remote side of the proxy that communicates with
/// remote MCP servers using various transport mechanisms.
///
/// # Examples
///
/// ```rust
/// use mcp_types::{McpClient, Result};
/// # use async_trait::async_trait;
///
/// struct MyClient;
///
/// #[async_trait]
/// impl McpClient for MyClient {
///     async fn connect(&mut self) -> Result<()> {
///         // Connection establishment logic
///         Ok(())
///     }
///
///     async fn send_request(&mut self, request: &str) -> Result<String> {
///         // Send request and return response
///         Ok("response".to_string())
///     }
///
///     async fn disconnect(&mut self) -> Result<()> {
///         // Cleanup logic
///         Ok(())
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait McpClient: Send + Sync {
    /// Establish connection to the remote MCP server.
    ///
    /// # Errors
    /// Returns [`McpError`] if connection cannot be established.
    async fn connect(&mut self) -> Result<()>;

    /// Send a request to the remote server and wait for response.
    ///
    /// # Arguments
    /// * `request` - The JSON-RPC request to send
    ///
    /// # Returns
    /// The JSON-RPC response from the remote server.
    ///
    /// # Errors
    /// Returns [`McpError`] if the request fails or times out.
    async fn send_request(&mut self, request: &str) -> Result<String>;

    /// Disconnect from the remote server.
    ///
    /// # Errors
    /// Returns [`McpError`] if disconnection cannot complete cleanly.
    async fn disconnect(&mut self) -> Result<()>;
}
