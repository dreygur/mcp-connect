use std::fmt;
use serde_json;

pub type Result<T> = std::result::Result<T, ServerError>;

#[derive(Debug)]
pub enum ServerError {
    Transport(String),
    Serialization(serde_json::Error),
    Io(std::io::Error),
    Protocol(String),
    ConnectionClosed,
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerError::Transport(msg) => write!(f, "Transport error: {}", msg),
            ServerError::Serialization(err) => write!(f, "Serialization error: {}", err),
            ServerError::Io(err) => write!(f, "IO error: {}", err),
            ServerError::Protocol(msg) => write!(f, "Protocol error: {}", msg),
            ServerError::ConnectionClosed => write!(f, "Connection closed"),
        }
    }
}

impl std::error::Error for ServerError {}

impl From<serde_json::Error> for ServerError {
    fn from(err: serde_json::Error) -> Self {
        ServerError::Serialization(err)
    }
}

impl From<std::io::Error> for ServerError {
    fn from(err: std::io::Error) -> Self {
        ServerError::Io(err)
    }
}
