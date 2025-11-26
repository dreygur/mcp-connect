use crate::error::{ProxyError, Result};
use async_trait::async_trait;
use mcp_client::McpRemoteClient;
use mcp_types::McpClient;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

#[async_trait]
pub trait ProxyStrategy: Send + Sync {
    async fn handle_request(&self, request: &str) -> Result<Option<String>>;
    async fn initialize(&self) -> Result<()>;
    async fn shutdown(&self) -> Result<()>;
}

pub struct ForwardingStrategy {
    client: Arc<Mutex<McpRemoteClient>>,
    initialized: Arc<Mutex<bool>>,
}

impl ForwardingStrategy {
    pub fn new(client: McpRemoteClient) -> Self {
        Self {
            client: Arc::new(Mutex::new(client)),
            initialized: Arc::new(Mutex::new(false)),
        }
    }

    fn is_notification(message: &str) -> bool {
        if let Ok(parsed) = serde_json::from_str::<Value>(message) {
            parsed.get("id").is_none() && parsed.get("method").is_some()
        } else {
            false
        }
    }

    fn extract_method(message: &str) -> Option<String> {
        serde_json::from_str::<Value>(message)
            .ok()
            .and_then(|v| v.get("method").cloned())
            .and_then(|m| m.as_str().map(|s| s.to_string()))
    }

    async fn handle_initialize(&self, request: &str) -> Result<Option<String>> {
        debug!("Handling initialize request");

        // Connect and initialize with remote
        let mut client = self.client.lock().await;
        client.connect().await?;

        // Forward the initialize request to remote
        let response = client.send_request(request).await?;

        // Mark as initialized
        *self.initialized.lock().await = true;
        info!("Proxy client initialized via forwarded request");

        Ok(Some(response))
    }
}

#[async_trait]
impl ProxyStrategy for ForwardingStrategy {
    async fn handle_request(&self, request: &str) -> Result<Option<String>> {
        debug!("Forwarding request: {}", request);

        // Check if it's a notification (no response expected)
        if Self::is_notification(request) {
            debug!("Received notification, forwarding without expecting response");
            // Forward notification to remote but don't wait for response
            let initialized = *self.initialized.lock().await;
            if initialized {
                let mut client = self.client.lock().await;
                if let Err(e) = client.send_request(request).await {
                    warn!("Failed to forward notification: {}", e);
                }
            }
            return Ok(None);
        }

        let method = Self::extract_method(request);
        debug!("Extracted method: {:?}", method);

        // Handle initialize specially - connect to remote and forward
        if method.as_deref() == Some("initialize") {
            return self.handle_initialize(request).await;
        }

        // For other requests, ensure we're initialized first
        let initialized = *self.initialized.lock().await;
        if !initialized {
            // Return error - client should send initialize first
            if let Ok(parsed) = serde_json::from_str::<Value>(request) {
                if let Some(id) = parsed.get("id") {
                    let error_response = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32002,
                            "message": "Server not initialized. Send initialize request first."
                        }
                    });
                    return Ok(Some(error_response.to_string()));
                }
            }
            return Err(ProxyError::NotInitialized);
        }

        let mut client = self.client.lock().await;
        match client.send_request(request).await {
            Ok(response) => {
                debug!("Received response: {}", response);
                Ok(Some(response))
            }
            Err(e) => {
                error!("Failed to forward request: {}", e);

                // Create an error response in JSON-RPC format
                if let Ok(parsed) = serde_json::from_str::<Value>(request) {
                    if let Some(id) = parsed.get("id") {
                        let error_response = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {
                                "code": -32603,
                                "message": format!("Proxy forwarding error: {}", e)
                            }
                        });
                        return Ok(Some(error_response.to_string()));
                    }
                }

                Err(ProxyError::ForwardingFailed(e.to_string()))
            }
        }
    }

    async fn initialize(&self) -> Result<()> {
        // Don't initialize here - wait for client's initialize request
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        let mut client = self.client.lock().await;
        client.disconnect().await?;
        *self.initialized.lock().await = false;
        info!("Proxy strategy shut down");
        Ok(())
    }
}

pub struct LoadBalancingStrategy {
    clients: Vec<Arc<Mutex<McpRemoteClient>>>,
    current_client: Arc<Mutex<usize>>,
    initialized: Arc<Mutex<Vec<bool>>>,
}

impl LoadBalancingStrategy {
    pub fn new(clients: Vec<McpRemoteClient>) -> Self {
        let client_count = clients.len();
        Self {
            clients: clients.into_iter().map(|c| Arc::new(Mutex::new(c))).collect(),
            current_client: Arc::new(Mutex::new(0)),
            initialized: Arc::new(Mutex::new(vec![false; client_count])),
        }
    }

    async fn get_next_client(&self) -> Result<Arc<Mutex<McpRemoteClient>>> {
        let mut current = self.current_client.lock().await;
        let client = self.clients.get(*current)
            .ok_or_else(|| ProxyError::Strategy("No clients available".to_string()))?
            .clone();

        *current = (*current + 1) % self.clients.len();
        Ok(client)
    }

    async fn ensure_client_initialized(&self, client_index: usize) -> Result<()> {
        let mut initialized = self.initialized.lock().await;
        if !initialized[client_index] {
            let client = &self.clients[client_index];
            let mut client_guard = client.lock().await;
            client_guard.connect().await?;
            let _init_result = client_guard.initialize().await?;
            initialized[client_index] = true;
            info!("Load balancing client {} initialized", client_index);
        }
        Ok(())
    }
}

#[async_trait]
impl ProxyStrategy for LoadBalancingStrategy {
    async fn handle_request(&self, request: &str) -> Result<Option<String>> {
        debug!("Load balancing request: {}", request);

        if ForwardingStrategy::is_notification(request) {
            debug!("Received notification, forwarding to all clients");
            // Forward notification to all initialized clients
            let initialized = self.initialized.lock().await;
            for (i, client) in self.clients.iter().enumerate() {
                if initialized[i] {
                    let mut client_guard = client.lock().await;
                    if let Err(e) = client_guard.send_request(request).await {
                        warn!("Failed to forward notification to client {}: {}", i, e);
                    }
                }
            }
            return Ok(None);
        }

        // Try each client until one succeeds
        for _i in 0..self.clients.len() {
            let client = self.get_next_client().await?;
            let client_index = {
                let current = self.current_client.lock().await;
                (*current + self.clients.len() - 1) % self.clients.len()
            };

            match self.ensure_client_initialized(client_index).await {
                Ok(()) => {
                    let mut client_guard = client.lock().await;
                    match client_guard.send_request(request).await {
                        Ok(response) => {
                            debug!("Client {} handled request successfully", client_index);
                            return Ok(Some(response));
                        }
                        Err(e) => {
                            warn!("Client {} failed: {}", client_index, e);
                            // Mark client as not initialized to force reconnection
                            self.initialized.lock().await[client_index] = false;
                            continue;
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to initialize client {}: {}", client_index, e);
                    continue;
                }
            }
        }

        // All clients failed
        if let Ok(parsed) = serde_json::from_str::<Value>(request) {
            if let Some(id) = parsed.get("id") {
                let error_response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": "All load-balanced clients failed"
                    }
                });
                return Ok(Some(error_response.to_string()));
            }
        }

        Err(ProxyError::ForwardingFailed("All clients failed".to_string()))
    }

    async fn initialize(&self) -> Result<()> {
        // Initialize first client immediately, others on-demand
        if !self.clients.is_empty() {
            self.ensure_client_initialized(0).await?;
        }
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        for (i, client) in self.clients.iter().enumerate() {
            let mut client_guard = client.lock().await;
            if let Err(e) = client_guard.disconnect().await {
                warn!("Error disconnecting client {}: {}", i, e);
            }
        }

        *self.initialized.lock().await = vec![false; self.clients.len()];
        info!("Load balancing strategy shut down");
        Ok(())
    }
}
