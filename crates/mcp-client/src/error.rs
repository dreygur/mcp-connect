//! Error types for the MCP client with rmcp integration
//!
//! This module defines error types that wrap rmcp errors while maintaining
//! compatibility with the existing codebase.

use std::fmt;

/// Result type alias for client operations
pub type Result<T> = std::result::Result<T, ClientError>;

/// Client error types
#[derive(Debug)]
pub enum ClientError {
    /// Invalid URL provided
    InvalidUrl(String),

    /// Security-related errors (e.g., HTTP not allowed)
    SecurityError(String),

    /// Invalid header name or value
    InvalidHeader(String),

    /// Transport-level errors (connection, network, etc.)
    TransportError(String),

    /// MCP protocol errors
    ProtocolError(String),

    /// Authentication/authorization errors
    AuthError(String),

    /// Timeout errors
    Timeout(String),

    /// Configuration errors
    ConfigError(String),

    /// JSON serialization/deserialization errors
    SerializationError(String),

    /// rmcp SDK errors
    RmcpError(rmcp::RmcpError),

    /// Generic errors for backward compatibility
    Generic(String),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::InvalidUrl(msg) => write!(f, "Invalid URL: {}", msg),
            ClientError::SecurityError(msg) => write!(f, "Security error: {}", msg),
            ClientError::InvalidHeader(msg) => write!(f, "Invalid header: {}", msg),
            ClientError::TransportError(msg) => write!(f, "Transport error: {}", msg),
            ClientError::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            ClientError::AuthError(msg) => write!(f, "Authentication error: {}", msg),
            ClientError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            ClientError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            ClientError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            ClientError::RmcpError(err) => write!(f, "rmcp error: {}", err),
            ClientError::Generic(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ClientError::RmcpError(err) => Some(err),
            _ => None,
        }
    }
}

// Conversions from common error types
impl From<rmcp::RmcpError> for ClientError {
    fn from(err: rmcp::RmcpError) -> Self {
        ClientError::RmcpError(err)
    }
}

impl From<rmcp::ErrorData> for ClientError {
    fn from(err: rmcp::ErrorData) -> Self {
        ClientError::ProtocolError(format!("MCP error {:?}: {}", err.code, err.message))
    }
}

impl From<serde_json::Error> for ClientError {
    fn from(err: serde_json::Error) -> Self {
        ClientError::SerializationError(format!("JSON error: {}", err))
    }
}

impl From<url::ParseError> for ClientError {
    fn from(err: url::ParseError) -> Self {
        ClientError::InvalidUrl(format!("URL parse error: {}", err))
    }
}

impl From<reqwest::Error> for ClientError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            ClientError::Timeout(format!("Request timeout: {}", err))
        } else if err.is_connect() {
            ClientError::TransportError(format!("Connection error: {}", err))
        } else if err.is_request() {
            ClientError::TransportError(format!("Request error: {}", err))
        } else {
            ClientError::TransportError(format!("HTTP error: {}", err))
        }
    }
}

impl From<anyhow::Error> for ClientError {
    fn from(err: anyhow::Error) -> Self {
        ClientError::Generic(err.to_string())
    }
}

impl From<std::io::Error> for ClientError {
    fn from(err: std::io::Error) -> Self {
        ClientError::TransportError(format!("IO error: {}", err))
    }
}

// Backward compatibility aliases
pub type TransportError = ClientError;
pub type Protocol = ClientError;
pub type Transport = ClientError;

// Helper functions for creating specific error types
impl ClientError {
    pub fn invalid_url(msg: impl Into<String>) -> Self {
        ClientError::InvalidUrl(msg.into())
    }

    pub fn security_error(msg: impl Into<String>) -> Self {
        ClientError::SecurityError(msg.into())
    }

    pub fn transport_error(msg: impl Into<String>) -> Self {
        ClientError::TransportError(msg.into())
    }

    pub fn protocol_error(msg: impl Into<String>) -> Self {
        ClientError::ProtocolError(msg.into())
    }

    pub fn auth_error(msg: impl Into<String>) -> Self {
        ClientError::AuthError(msg.into())
    }

    pub fn timeout(msg: impl Into<String>) -> Self {
        ClientError::Timeout(msg.into())
    }

    pub fn config_error(msg: impl Into<String>) -> Self {
        ClientError::ConfigError(msg.into())
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            ClientError::TransportError(_) => true,
            ClientError::Timeout(_) => true,
            ClientError::RmcpError(_) => true, // Some rmcp errors might be retryable
            _ => false,
        }
    }

    /// Check if the error is a transport-related error
    pub fn is_transport_error(&self) -> bool {
        matches!(
            self,
            ClientError::TransportError(_) | ClientError::Timeout(_)
        )
    }

    /// Check if the error is a protocol-related error
    pub fn is_protocol_error(&self) -> bool {
        matches!(self, ClientError::ProtocolError(_))
    }

    /// Check if the error is security-related
    pub fn is_security_error(&self) -> bool {
        matches!(self, ClientError::SecurityError(_) | ClientError::AuthError(_))
    }

    /// Get the error message as a string
    pub fn message(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = ClientError::invalid_url("bad url");
        assert!(matches!(err, ClientError::InvalidUrl(_)));
        assert_eq!(err.message(), "Invalid URL: bad url");

        let err = ClientError::transport_error("connection failed");
        assert!(matches!(err, ClientError::TransportError(_)));
        assert!(err.is_transport_error());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_error_conversions() {
        // Test JSON error conversion
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json");
        assert!(json_err.is_err());
        let client_err: ClientError = json_err.unwrap_err().into();
        assert!(matches!(client_err, ClientError::SerializationError(_)));

        // Test URL error conversion
        let url_err = url::Url::parse("invalid url");
        assert!(url_err.is_err());
        let client_err: ClientError = url_err.unwrap_err().into();
        assert!(matches!(client_err, ClientError::InvalidUrl(_)));
    }

    #[test]
    fn test_error_classification() {
        let transport_err = ClientError::transport_error("test");
        assert!(transport_err.is_transport_error());
        assert!(transport_err.is_retryable());
        assert!(!transport_err.is_protocol_error());

        let protocol_err = ClientError::protocol_error("test");
        assert!(protocol_err.is_protocol_error());
        assert!(!protocol_err.is_retryable());
        assert!(!protocol_err.is_transport_error());

        let security_err = ClientError::security_error("test");
        assert!(security_err.is_security_error());
        assert!(!security_err.is_retryable());
    }
}
