use crate::error::{ClientError, Result};
use crate::transport::Transport;
use crate::types::JsonRpcMessage;
use async_trait::async_trait;
use reqwest::{Client, header::HeaderMap};
use tracing::info;
use std::time::Duration;
use tokio::sync::mpsc;
use url::Url;

pub struct HttpTransport {
    client: Client,
    endpoint: Url,
    headers: HeaderMap,
    response_receiver: Option<mpsc::UnboundedReceiver<JsonRpcMessage>>,
    connected: bool,
}

impl HttpTransport {
    pub fn new(endpoint: &str) -> Result<Self> {
        let endpoint = Url::parse(endpoint)
            .map_err(|e| ClientError::Transport(format!("Invalid URL: {}", e)))?;

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            endpoint,
            headers: HeaderMap::new(),
            response_receiver: None,
            connected: false,
        })
    }

    pub fn with_headers(mut self, headers: &[String]) -> Result<Self> {
        for header_str in headers {
            if let Some((key, value)) = header_str.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
                    .map_err(|e| ClientError::Transport(format!("Invalid header name '{}': {}", key, e)))?;
                let header_value = reqwest::header::HeaderValue::from_str(value)
                    .map_err(|e| ClientError::Transport(format!("Invalid header value '{}': {}", value, e)))?;

                self.headers.insert(header_name, header_value);
            } else {
                return Err(ClientError::Transport(format!("Invalid header format '{}'. Expected 'key:value'", header_str)));
            }
        }
        Ok(self)
    }

    pub async fn connect(&mut self) -> Result<()> {
        // For GitHub Copilot MCP server, we can't use GET requests
        // Just mark as connected and rely on first POST request to validate
        self.connected = true;
        Ok(())
    }
}

#[async_trait]
impl Transport for HttpTransport {
    async fn send(&mut self, message: JsonRpcMessage) -> Result<()> {
        if !self.connected {
            return Err(ClientError::ConnectionClosed);
        }

        let response = self.client
            .post(self.endpoint.clone())
            .headers(self.headers.clone())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&message)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ClientError::Transport(format!(
                "HTTP request failed: {}",
                response.status()
            )));
        }

        // Handle different response types based on Content-Type
        let content_type = response.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.starts_with("text/event-stream") {
            // SSE response - need to handle streaming
            self.handle_sse_response(response).await?;
        } else if content_type.starts_with("application/json") {
            // Direct JSON response
            let json_response: JsonRpcMessage = response.json().await?;
            let (tx, rx) = mpsc::unbounded_channel();
            tx.send(json_response)
                .map_err(|_| ClientError::Transport("Failed to queue response".into()))?;
            self.response_receiver = Some(rx);
        }

        Ok(())
    }

    async fn receive(&mut self) -> Result<JsonRpcMessage> {
        if let Some(ref mut receiver) = self.response_receiver {
            if let Some(message) = receiver.recv().await {
                return Ok(message);
            }
        }

        Err(ClientError::Transport("No messages available".into()))
    }

    async fn close(&mut self) -> Result<()> {
        self.connected = false;
        self.response_receiver = None;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

impl HttpTransport {
    async fn handle_sse_response(&mut self, response: reqwest::Response) -> Result<()> {
        use eventsource_stream::Eventsource;
        use futures::StreamExt;

        let (tx, rx) = mpsc::unbounded_channel();
        self.response_receiver = Some(rx);

        let stream = response.bytes_stream().eventsource();

        tokio::spawn(async move {
            futures::pin_mut!(stream);

            while let Some(event) = stream.next().await {
                match event {
                    Ok(event) => {
                        if event.event == "message" {
                            if let Ok(json_msg) = serde_json::from_str::<JsonRpcMessage>(&event.data) {
                                if tx.send(json_msg).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(())
    }
}
