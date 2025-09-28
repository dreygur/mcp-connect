use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("Server error: {0}")]
    Server(#[from] mcp_server::ServerError),

    #[error("Client error: {0}")]
    Client(#[from] mcp_client::ClientError),

    #[error("MCP error: {0}")]
    Mcp(#[from] mcp_types::McpError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Proxy not initialized")]
    NotInitialized,

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Forwarding failed: {0}")]
    ForwardingFailed(String),

    #[error("Strategy error: {0}")]
    Strategy(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Protocol error: {0}")]
    Protocol(String),
}

pub type Result<T> = std::result::Result<T, ProxyError>;
