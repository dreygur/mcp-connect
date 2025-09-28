use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Transport error: {0}")]
    Transport(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Timeout")]
    Timeout,

    #[error("MCP error: {0}")]
    Mcp(#[from] mcp_types::McpError),
}

pub type Result<T> = std::result::Result<T, ClientError>;
