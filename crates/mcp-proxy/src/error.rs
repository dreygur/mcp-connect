use std::fmt;

pub type Result<T> = std::result::Result<T, ProxyError>;

#[derive(Debug)]
pub enum ProxyError {
    Transport(String),
    Protocol(String),
    ConnectionFailed(String),
    Timeout,
    Serialization(serde_json::Error),
    RmcpError(rmcp::ErrorData),
}

impl fmt::Display for ProxyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyError::Transport(msg) => write!(f, "Transport error: {}", msg),
            ProxyError::Protocol(msg) => write!(f, "Protocol error: {}", msg),
            ProxyError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            ProxyError::Timeout => write!(f, "Operation timed out"),
            ProxyError::Serialization(err) => write!(f, "Serialization error: {}", err),
            ProxyError::RmcpError(err) => write!(f, "RMCP error: {}", err),
        }
    }
}

impl std::error::Error for ProxyError {}

impl From<serde_json::Error> for ProxyError {
    fn from(err: serde_json::Error) -> Self {
        ProxyError::Serialization(err)
    }
}

impl From<rmcp::ErrorData> for ProxyError {
    fn from(err: rmcp::ErrorData) -> Self {
        ProxyError::RmcpError(err)
    }
}
