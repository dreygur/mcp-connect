use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Server not initialized")]
    NotInitialized,

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("MCP error: {0}")]
    Mcp(#[from] mcp_types::McpError),
}

pub type Result<T> = std::result::Result<T, ServerError>;
