use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum McpError {
    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Timeout error")]
    Timeout,

    #[error("Authentication error: {0}")]
    Auth(String),
}

pub type Result<T> = std::result::Result<T, McpError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMessage {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warn")]
    Warn,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub server_debug: bool,
    pub client_endpoint: String,
    pub fallback_transports: Vec<TransportType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportType {
    #[serde(rename = "stdio")]
    Stdio,
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "tcp")]
    Tcp,
}

#[async_trait::async_trait]
pub trait McpTransport: Send + Sync {
    async fn send_message(&mut self, message: &str) -> Result<()>;
    async fn receive_message(&mut self) -> Result<String>;
    async fn close(&mut self) -> Result<()>;
}

#[async_trait::async_trait]
pub trait McpServer: Send + Sync {
    async fn start(&mut self) -> Result<()>;
    async fn handle_message(&mut self, message: &str) -> Result<Option<String>>;
    async fn shutdown(&mut self) -> Result<()>;
}

#[async_trait::async_trait]
pub trait McpClient: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn send_request(&mut self, request: &str) -> Result<String>;
    async fn disconnect(&mut self) -> Result<()>;
}
