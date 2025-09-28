use crate::error::Result;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use std::collections::HashMap;
use std::time::Duration;

pub mod http;
pub mod stdio;
pub mod tcp;

pub use http::HttpTransport;
pub use stdio::StdioTransport;
pub use tcp::TcpTransport;

#[async_trait]
pub trait McpClientTransport: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn send_request(&mut self, request: &str) -> Result<String>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn is_connected(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub endpoint: String,
    pub timeout: Duration,
    pub retry_attempts: u32,
    pub retry_delay: Duration,
    pub headers: HashMap<String, String>,
    pub auth_token: Option<String>,
    pub user_agent: Option<String>,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:8080".to_string(),
            timeout: Duration::from_secs(30),
            retry_attempts: 3,
            retry_delay: Duration::from_millis(1000),
            headers: HashMap::new(),
            auth_token: None,
            user_agent: Some("mcp-remote-client/0.1.0".to_string()),
        }
    }
}

impl TransportConfig {
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers.extend(headers);
        self
    }

    pub fn with_auth_token(mut self, token: String) -> Self {
        self.auth_token = Some(token);
        self
    }

    pub fn with_bearer_token(mut self, token: String) -> Self {
        self.headers.insert("Authorization".to_string(), format!("Bearer {}", token));
        self
    }

    pub fn with_basic_auth(mut self, username: String, password: String) -> Self {
        let credentials = general_purpose::STANDARD.encode(format!("{}:{}", username, password));
        self.headers.insert("Authorization".to_string(), format!("Basic {}", credentials));
        self
    }

    pub fn with_api_key(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }
}

pub async fn create_transport(
    transport_type: mcp_types::TransportType,
    config: TransportConfig,
) -> Result<Box<dyn McpClientTransport>> {
    match transport_type {
        mcp_types::TransportType::Http => {
            Ok(Box::new(HttpTransport::new(config)))
        }
        mcp_types::TransportType::Stdio => {
            Ok(Box::new(StdioTransport::new(config)))
        }
        mcp_types::TransportType::Tcp => {
            Ok(Box::new(TcpTransport::new(config)))
        }
    }
}
