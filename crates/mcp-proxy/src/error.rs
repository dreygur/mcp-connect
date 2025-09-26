use std::fmt;

pub type Result<T> = std::result::Result<T, ProxyError>;

#[derive(Debug)]
pub enum ProxyError {
    Client(mcp_client::ClientError),
    Server(mcp_server::ServerError),
    Transport(String),
    Protocol(String),
    ConnectionFailed(String),
    Timeout,
    Serialization(serde_json::Error),
}

impl fmt::Display for ProxyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyError::Client(err) => write!(f, "Client error: {}", err),
            ProxyError::Server(err) => write!(f, "Server error: {}", err),
            ProxyError::Transport(msg) => write!(f, "Transport error: {}", msg),
            ProxyError::Protocol(msg) => write!(f, "Protocol error: {}", msg),
            ProxyError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            ProxyError::Timeout => write!(f, "Operation timed out"),
            ProxyError::Serialization(err) => write!(f, "Serialization error: {}", err),
        }
    }
}

impl std::error::Error for ProxyError {}

impl From<mcp_client::ClientError> for ProxyError {
    fn from(err: mcp_client::ClientError) -> Self {
        ProxyError::Client(err)
    }
}

impl From<mcp_server::ServerError> for ProxyError {
    fn from(err: mcp_server::ServerError) -> Self {
        ProxyError::Server(err)
    }
}

impl From<serde_json::Error> for ProxyError {
    fn from(err: serde_json::Error) -> Self {
        ProxyError::Serialization(err)
    }
}
