use crate::auth::TokenProvider;
use crate::error::{ClientError, Result};
use crate::transport::{McpClientTransport, TransportConfig};
use async_trait::async_trait;
use reqwest::{Client, header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, CONTENT_TYPE}};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Mask sensitive token for logging (show first 8 chars + "...")
fn mask_token(token: &str) -> String {
    if token.len() <= 12 {
        "***".to_string()
    } else {
        format!("{}...", &token[..8])
    }
}

pub struct HttpTransport {
    client: Client,
    config: TransportConfig,
    connected: Arc<Mutex<bool>>,
    session_id: Arc<Mutex<Option<String>>>,
    token_provider: Option<Arc<dyn TokenProvider>>,
}

impl HttpTransport {
    pub fn new(config: TransportConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| ClientError::Connection(format!("Failed to create HTTP client: {}", e)))?;

        let token_provider = config.token_provider.clone();

        Ok(Self {
            client,
            config,
            connected: Arc::new(Mutex::new(false)),
            session_id: Arc::new(Mutex::new(None)),
            token_provider,
        })
    }

    async fn send_http_request(&self, payload: &str) -> Result<String> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json, text/event-stream"));

        // Add custom headers from config (mask sensitive values in logs)
        for (key, value) in &self.config.headers {
            let log_value = if key.to_lowercase() == "authorization" {
                mask_token(value)
            } else {
                value.clone()
            };
            debug!("Adding header: {}={}", key, log_value);
            headers.insert(
                HeaderName::from_str(key).map_err(|e| ClientError::Protocol(format!("Invalid header name '{}': {}", key, e)))?,
                HeaderValue::from_str(value).map_err(|e| ClientError::Protocol(format!("Invalid header value for '{}': {}", key, e)))?
            );
        }

        // Add authentication token - prefer dynamic provider over static token
        if let Some(ref provider) = self.token_provider {
            match provider.get_token().await {
                Ok(token) => {
                    let auth_value = if token.starts_with("Bearer ") {
                        token
                    } else {
                        format!("Bearer {}", token)
                    };
                    debug!("Adding dynamic auth token: {}", mask_token(&auth_value));
                    headers.insert("Authorization", HeaderValue::from_str(&auth_value)
                        .map_err(|e| ClientError::Protocol(format!("Invalid auth token: {}", e)))?);
                }
                Err(e) => {
                    warn!("Failed to get token from provider: {}", e);
                    // Fall through to static token if available
                    if let Some(auth_token) = &self.config.auth_token {
                        debug!("Falling back to static auth token: {}", mask_token(auth_token));
                        headers.insert("Authorization", HeaderValue::from_str(auth_token)
                            .map_err(|e| ClientError::Protocol(format!("Invalid auth token: {}", e)))?);
                    }
                }
            }
        } else if let Some(auth_token) = &self.config.auth_token {
            debug!("Adding static auth token: {}", mask_token(auth_token));
            headers.insert("Authorization", HeaderValue::from_str(auth_token)
                .map_err(|e| ClientError::Protocol(format!("Invalid auth token: {}", e)))?);
        }

        // Add custom user agent if present
        if let Some(user_agent) = &self.config.user_agent {
            headers.insert("User-Agent", HeaderValue::from_str(user_agent)
                .map_err(|e| ClientError::Protocol(format!("Invalid user agent: {}", e)))?);
        }

        // Add session ID if available
        if let Some(session_id) = self.session_id.lock().await.as_ref() {
            headers.insert("Mcp-Session-Id", HeaderValue::from_str(session_id)
                .map_err(|e| ClientError::Protocol(format!("Invalid session ID: {}", e)))?);
        }

        debug!("Sending HTTP request to {}: {}", self.config.endpoint, payload);

        let response = self.client
            .post(&self.config.endpoint)
            .headers(headers)
            .body(payload.to_string())
            .send()
            .await?;

        // Extract session ID from response headers if present
        if let Some(session_id) = response.headers().get("Mcp-Session-Id") {
            if let Ok(session_str) = session_id.to_str() {
                *self.session_id.lock().await = Some(session_str.to_string());
                debug!("Updated session ID: {}", session_str);
            }
        }

        if response.status() == 202 {
            // HTTP 202 Accepted with no body - this is the expected response for MCP over HTTP
            debug!("Received HTTP 202 Accepted");
            return Ok("{}".to_string()); // Return empty JSON object
        }

        if !response.status().is_success() {
            return Err(ClientError::Protocol(format!(
                "HTTP error: {} {}",
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }

        let status = response.status();
        let response_text = response.text().await?;
        debug!("Received HTTP response (status {}): {}", status, response_text);

        // For MCP over HTTP, we might get various response formats
        if response_text.trim().is_empty() {
            debug!("Empty response body, returning empty JSON object");
            return Ok("{}".to_string());
        }

        // Check if this is a Server-Sent Events (SSE) response
        if response_text.starts_with("event:") || response_text.contains("data:") {
            debug!("Detected SSE response, parsing data field");
            return self.parse_sse_response(&response_text);
        }

        Ok(response_text)
    }

    fn parse_sse_response(&self, sse_text: &str) -> Result<String> {
        // Parse SSE format to extract JSON data
        // Format: event: message\ndata: {...}\n\n
        let mut json_data = String::new();

        for line in sse_text.lines() {
            let line = line.trim();
            if line.starts_with("data:") {
                let data_part = &line[5..].trim(); // Remove "data:" prefix
                json_data.push_str(data_part);
            }
        }

        if json_data.is_empty() {
            debug!("No data field found in SSE response, returning empty JSON");
            return Ok("{}".to_string());
        }

        debug!("Extracted JSON from SSE: {}", json_data);
        Ok(json_data)
    }

    async fn test_connection(&self) -> Result<()> {
        // For MCP over HTTP, we skip the connection test and let the first actual request determine connectivity
        // This is because many MCP servers only accept POST requests and return errors for GET
        info!("HTTP transport ready (connection will be verified on first request)");
        Ok(())
    }
}

#[async_trait]
impl McpClientTransport for HttpTransport {
    async fn connect(&mut self) -> Result<()> {
        info!("Connecting to MCP server via HTTP: {}", self.config.endpoint);

        for attempt in 1..=self.config.retry_attempts {
            match self.test_connection().await {
                Ok(()) => {
                    *self.connected.lock().await = true;
                    info!("Successfully connected to MCP server");
                    return Ok(());
                }
                Err(e) => {
                    warn!("Connection attempt {} failed: {}", attempt, e);
                    if attempt < self.config.retry_attempts {
                        tokio::time::sleep(self.config.retry_delay).await;
                    }
                }
            }
        }

        Err(ClientError::Connection(format!(
            "Failed to connect after {} attempts",
            self.config.retry_attempts
        )))
    }

    async fn send_request(&mut self, request: &str) -> Result<String> {
        if !self.is_connected().await {
            return Err(ClientError::Connection("Not connected".to_string()));
        }

        for attempt in 1..=self.config.retry_attempts {
            match self.send_http_request(request).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    error!("Request attempt {} failed: {}", attempt, e);
                    if attempt < self.config.retry_attempts {
                        tokio::time::sleep(self.config.retry_delay).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(ClientError::Protocol("All retry attempts failed".to_string()))
    }

    async fn disconnect(&mut self) -> Result<()> {
        *self.connected.lock().await = false;
        *self.session_id.lock().await = None;
        info!("Disconnected from MCP server");
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        *self.connected.lock().await
    }
}
