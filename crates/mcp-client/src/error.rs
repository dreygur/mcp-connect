use std::fmt;

pub type Result<T> = std::result::Result<T, ClientError>;

#[derive(Debug)]
pub enum ClientError {
    Transport(String),
    Serialization(serde_json::Error),
    Http(reqwest::Error),
    Protocol(String),
    Timeout,
    ConnectionClosed,
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::Transport(msg) => write!(f, "Transport error: {}", msg),
            ClientError::Serialization(err) => write!(f, "Serialization error: {}", err),
            ClientError::Http(err) => write!(f, "HTTP error: {}", err),
            ClientError::Protocol(msg) => write!(f, "Protocol error: {}", msg),
            ClientError::Timeout => write!(f, "Operation timed out"),
            ClientError::ConnectionClosed => write!(f, "Connection closed"),
        }
    }
}

impl std::error::Error for ClientError {}

impl From<serde_json::Error> for ClientError {
    fn from(err: serde_json::Error) -> Self {
        ClientError::Serialization(err)
    }
}

impl From<reqwest::Error> for ClientError {
    fn from(err: reqwest::Error) -> Self {
        ClientError::Http(err)
    }
}
