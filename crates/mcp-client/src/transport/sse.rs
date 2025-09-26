use crate::error::{ClientError, Result};
use crate::transport::Transport;
use crate::types::JsonRpcMessage;
use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;
use tokio::sync::mpsc;
use url::Url;

pub struct SseTransport {
    client: Client,
    sse_endpoint: Url,
    post_endpoint: Option<Url>,
    response_receiver: Option<mpsc::UnboundedReceiver<JsonRpcMessage>>,
    connected: bool,
}

impl SseTransport {
    pub fn new(endpoint: &str) -> Result<Self> {
        let sse_endpoint = Url::parse(endpoint)
            .map_err(|e| ClientError::Transport(format!("Invalid URL: {}", e)))?;

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            sse_endpoint,
            post_endpoint: None,
            response_receiver: None,
            connected: false,
        })
    }

    pub async fn connect(&mut self) -> Result<()> {
        // Open SSE connection
        let response = self.client
            .get(self.sse_endpoint.clone())
            .header("Accept", "text/event-stream")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ClientError::Transport(format!(
                "SSE connection failed: {}",
                response.status()
            )));
        }

        self.start_sse_stream(response).await?;
        self.connected = true;
        Ok(())
    }

    async fn start_sse_stream(&mut self, response: reqwest::Response) -> Result<()> {
        use eventsource_stream::Eventsource;
        use futures::StreamExt;

        let (tx, rx) = mpsc::unbounded_channel();
        self.response_receiver = Some(rx);

        let tx_clone = tx.clone();
        let stream = response.bytes_stream().eventsource();

        tokio::spawn(async move {
            futures::pin_mut!(stream);

            while let Some(event) = stream.next().await {
                match event {
                    Ok(event) => {
                        match event.event.as_str() {
                            "endpoint" => {
                                // Server sends endpoint event with POST URL
                                if let Ok(endpoint_url) = serde_json::from_str::<serde_json::Value>(&event.data) {
                                    if let Some(uri) = endpoint_url.as_str() {
                                        // We should store this endpoint for POST requests
                                        // For now, we'll assume it's handled elsewhere
                                        tracing::debug!("Received endpoint: {}", uri);
                                    }
                                }
                            }
                            "message" => {
                                if let Ok(json_msg) = serde_json::from_str::<JsonRpcMessage>(&event.data) {
                                    if tx_clone.send(json_msg).is_err() {
                                        break;
                                    }
                                }
                            }
                            _ => {
                                tracing::debug!("Received unknown event: {}", event.event);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("SSE stream error: {:?}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }
}

#[async_trait]
impl Transport for SseTransport {
    async fn send(&mut self, message: JsonRpcMessage) -> Result<()> {
        if !self.connected {
            return Err(ClientError::ConnectionClosed);
        }

        let post_endpoint = self.post_endpoint.as_ref()
            .ok_or_else(|| ClientError::Transport("No POST endpoint available".into()))?;

        let response = self.client
            .post(post_endpoint.clone())
            .header("Content-Type", "application/json")
            .json(&message)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ClientError::Transport(format!(
                "POST request failed: {}",
                response.status()
            )));
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
        self.post_endpoint = None;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

impl SseTransport {
    pub fn set_post_endpoint(&mut self, endpoint: &str) -> Result<()> {
        let url = Url::parse(endpoint)
            .map_err(|e| ClientError::Transport(format!("Invalid POST endpoint: {}", e)))?;
        self.post_endpoint = Some(url);
        Ok(())
    }
}
