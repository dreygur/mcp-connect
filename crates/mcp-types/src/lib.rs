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
use std::collections::HashMap;
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

/// Central configuration file structure for mcp-connect.
///
/// This structure represents the `.mcp-connect.json` file that contains
/// all remote MCP server configurations.
///
/// # Examples
///
/// ```rust
/// use mcp_types::ConnectConfig;
/// use std::collections::HashMap;
///
/// let config = ConnectConfig {
///     schema: Some("https://static.modelcontextprotocol.io/schemas/2025-10-17/mcp-connect-config.schema.json".to_string()),
///     version: "1.0".to_string(),
///     env_file: Some(".env".to_string()),
///     servers: HashMap::new(),
///     routing: None,
///     registries: None,
///     default_registry: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectConfig {
    /// JSON schema URL for validation
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Config file format version
    pub version: String,

    /// Path to .env file for environment variable substitution
    #[serde(rename = "envFile", skip_serializing_if = "Option::is_none")]
    pub env_file: Option<String>,

    /// Map of server name to server configuration
    pub servers: HashMap<String, ServerConfig>,

    /// Routing configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing: Option<RoutingConfig>,

    /// Custom registries configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registries: Option<HashMap<String, RegistryConfig>>,

    /// Default registry to use (name from registries map, or "default" for official registry)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_registry: Option<String>,
}

impl Default for ConnectConfig {
    fn default() -> Self {
        Self {
            schema: Some("https://static.modelcontextprotocol.io/schemas/2025-10-17/mcp-connect-config.schema.json".to_string()),
            version: "1.0".to_string(),
            env_file: Some(".env".to_string()),
            servers: HashMap::new(),
            routing: Some(RoutingConfig::default()),
            registries: None,
            default_registry: None,
        }
    }
}

/// Configuration for a single remote MCP server.
///
/// Contains all the information needed to connect to and communicate
/// with a remote MCP server.
///
/// # Examples
///
/// ```rust
/// use mcp_types::{ServerConfig, RemoteConfig, RemoteTransportType, KeyValue};
///
/// let config = ServerConfig {
///     name: Some("io.github.modelcontextprotocol/github-mcp-server".to_string()),
///     description: Some("GitHub repository management".to_string()),
///     version: Some("latest".to_string()),
///     remote: RemoteConfig {
///         transport_type: RemoteTransportType::StreamableHttp,
///         url: "https://api.github.com/mcp".to_string(),
///         headers: Some(vec![KeyValue {
///             key: "Authorization".to_string(),
///             value: "Bearer ${GITHUB_TOKEN}".to_string(),
///         }]),
///         oauth: None,
///     },
///     timeout: Some(30),
///     retry_attempts: Some(3),
///     fallbacks: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Registry name (e.g., "io.github.modelcontextprotocol/github-mcp-server")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Version from registry (typically "latest")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Remote transport configuration
    pub remote: RemoteConfig,

    /// Connection timeout in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,

    /// Number of retry attempts
    #[serde(rename = "retryAttempts", skip_serializing_if = "Option::is_none")]
    pub retry_attempts: Option<u32>,

    /// Fallback transport types if primary fails
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallbacks: Option<Vec<String>>,
}

/// Remote transport configuration matching MCP registry schema.
///
/// Defines how to connect to a remote MCP server using HTTP-based transports.
///
/// # Examples
///
/// ```rust
/// use mcp_types::{RemoteConfig, RemoteTransportType, KeyValue};
///
/// let config = RemoteConfig {
///     transport_type: RemoteTransportType::StreamableHttp,
///     url: "https://api.example.com/mcp".to_string(),
///     headers: Some(vec![KeyValue {
///         key: "Authorization".to_string(),
///         value: "Bearer ${API_TOKEN}".to_string(),
///     }]),
///     oauth: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    /// Transport type (streamable-http or sse)
    #[serde(rename = "type")]
    pub transport_type: RemoteTransportType,

    /// Remote server URL
    pub url: String,

    /// HTTP headers for authentication and customization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<KeyValue>>,

    /// OAuth configuration for servers requiring OAuth authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth: Option<OAuthConfig>,
}

/// OAuth configuration for connecting to OAuth-protected MCP servers.
///
/// Supports OAuth 2.0 Authorization Code flow with PKCE.
///
/// # Examples
///
/// ```rust
/// use mcp_types::OAuthConfig;
///
/// let oauth = OAuthConfig {
///     client_id: "my-client-id".to_string(),
///     client_secret: None, // Public client with PKCE
///     auth_url: "https://auth.example.com/authorize".to_string(),
///     token_url: "https://auth.example.com/token".to_string(),
///     redirect_url: "http://localhost:8085/callback".to_string(),
///     scopes: vec!["mcp".to_string(), "profile".to_string()],
///     use_pkce: true,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// OAuth client ID
    #[serde(rename = "clientId")]
    pub client_id: String,

    /// OAuth client secret (optional for public clients using PKCE)
    #[serde(rename = "clientSecret", skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,

    /// Authorization endpoint URL
    #[serde(rename = "authUrl")]
    pub auth_url: String,

    /// Token endpoint URL
    #[serde(rename = "tokenUrl")]
    pub token_url: String,

    /// Redirect URL for OAuth callback
    #[serde(rename = "redirectUrl")]
    pub redirect_url: String,

    /// OAuth scopes to request
    #[serde(default)]
    pub scopes: Vec<String>,

    /// Use PKCE (Proof Key for Code Exchange) - recommended for security
    #[serde(rename = "usePkce", default = "default_use_pkce")]
    pub use_pkce: bool,
}

fn default_use_pkce() -> bool {
    true
}

/// Remote transport types supported by the MCP registry.
///
/// These transport types are defined in the official MCP server schema
/// and used for connecting to remote servers over HTTP-based protocols.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum RemoteTransportType {
    /// Streamable HTTP transport for bidirectional communication
    StreamableHttp,
    /// Server-Sent Events transport
    Sse,
}

/// Key-value pair for HTTP headers.
///
/// Used in remote configurations to specify headers like authentication tokens.
///
/// # Examples
///
/// ```rust
/// use mcp_types::KeyValue;
///
/// let auth_header = KeyValue {
///     key: "Authorization".to_string(),
///     value: "Bearer ${TOKEN}".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValue {
    /// Header name (e.g., "Authorization")
    pub key: String,
    /// Header value with optional env var substitution (e.g., "Bearer ${TOKEN}")
    pub value: String,
}

/// Routing configuration for multiplexing multiple servers.
///
/// Controls how tool/resource names are mapped to specific servers.
///
/// # Examples
///
/// ```rust
/// use mcp_types::{RoutingConfig, RoutingMethod};
///
/// let config = RoutingConfig {
///     method: RoutingMethod::NamespacePrefix,
///     separator: "/".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Routing method to use
    pub method: RoutingMethod,
    /// Separator character for namespace prefixing
    pub separator: String,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            method: RoutingMethod::NamespacePrefix,
            separator: "/".to_string(),
        }
    }
}

/// Routing methods for multiplexing servers.
///
/// Defines how tools/resources from multiple servers are distinguished.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum RoutingMethod {
    /// Prefix tool names with server name (e.g., "github/search_code")
    NamespacePrefix,
}

/// Configuration for a custom MCP registry.
///
/// Allows users to configure additional registries beyond the official one.
///
/// # Examples
///
/// ```rust
/// use mcp_types::RegistryConfig;
///
/// let config = RegistryConfig {
///     base_url: "https://my-registry.example.com".to_string(),
///     api_version: Some("v1".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Base URL of the registry (e.g., "https://registry.example.com")
    #[serde(rename = "baseUrl")]
    pub base_url: String,

    /// API version path (e.g., "v1", "v0.1"). If None, uses default version.
    #[serde(rename = "apiVersion", skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
}
