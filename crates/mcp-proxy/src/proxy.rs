use crate::error::{ProxyError, Result};
use crate::strategy::ProxyStrategy;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

pub struct McpProxy {
    strategy: Arc<dyn ProxyStrategy>,
    running: Arc<Mutex<bool>>,
}

impl McpProxy {
    pub fn new(strategy: Arc<dyn ProxyStrategy>) -> Self {
        Self {
            strategy,
            running: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting MCP proxy");

        *self.running.lock().await = true;
        self.strategy.initialize().await?;

        info!("MCP proxy started successfully");
        Ok(())
    }

    pub async fn handle_message(&self, message: &str) -> Result<Option<String>> {
        if !*self.running.lock().await {
            return Err(ProxyError::NotInitialized);
        }

        debug!("Proxy handling message: {}", message);

        match self.strategy.handle_request(message).await {
            Ok(response) => {
                if let Some(ref resp) = response {
                    debug!("Proxy returning response: {}", resp);
                } else {
                    debug!("Proxy handling notification (no response)");
                }
                Ok(response)
            }
            Err(e) => {
                error!("Proxy error handling message: {}", e);
                Err(e)
            }
        }
    }

    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down MCP proxy");

        *self.running.lock().await = false;
        self.strategy.shutdown().await?;

        info!("MCP proxy shut down successfully");
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        *self.running.lock().await
    }
}
