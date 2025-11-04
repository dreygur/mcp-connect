//! Serve command implementation.
//!
//! Run the multiplexing MCP server that serves all configured remote servers.

use anyhow::{Context, Result};
use mcp_client::McpRemoteClient;
use mcp_config::ConfigManager;
use mcp_proxy::stdio_proxy::StdioProxyBuilder;
use mcp_types::{ConnectConfig, McpClient};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

/// Run the multiplexing server.
pub async fn serve(debug: bool) -> Result<()> {
    let mut manager = ConfigManager::new()?;

    if !manager.exists() {
        anyhow::bail!(
            "No .mcp-connect.json found.\n\
            Run 'mcp-connect init' to create configuration."
        );
    }

    let config = manager.load()?;

    if config.servers.is_empty() {
        warn!("No servers configured in .mcp-connect.json");
        println!("⚠️  No servers configured.");
        println!("Add servers with: mcp-connect config add <name> <registry-path>");
        anyhow::bail!("Cannot start with no servers configured");
    }

    info!("Starting mcp-connect multiplexing server");
    info!("Configured servers: {}", config.servers.len());

    // Create the multiplexing strategy
    let strategy = Arc::new(MultiplexingStrategy::new(config).await?);

    // Build and run the proxy
    let proxy = StdioProxyBuilder::new()
        .with_strategy(strategy)
        .with_debug_mode(debug)
        .build()?;

    info!("Multiplexing server ready, listening on STDIO");
    proxy.run().await?;

    Ok(())
}

/// Multiplexing strategy that routes requests to multiple servers.
struct MultiplexingStrategy {
    clients: HashMap<String, Arc<Mutex<McpRemoteClient>>>,
    routing_config: mcp_types::RoutingConfig,
}

impl MultiplexingStrategy {
    /// Create a new multiplexing strategy from configuration.
    async fn new(config: ConnectConfig) -> Result<Self> {
        let mut clients = HashMap::new();
        let routing_config = config.routing.unwrap_or_default();

        // Create a client for each configured server
        for (name, server_config) in config.servers {
            info!("Initializing client for server: {}", name);

            // Build transport config from server config
            let mut headers = HashMap::new();
            if let Some(server_headers) = &server_config.remote.headers {
                for header in server_headers {
                    headers.insert(header.key.clone(), header.value.clone());
                }
            }

            let transport_config = mcp_client::transport::TransportConfig {
                endpoint: server_config.remote.url.clone(),
                timeout: std::time::Duration::from_secs(server_config.timeout.unwrap_or(30)),
                retry_attempts: server_config.retry_attempts.unwrap_or(3),
                retry_delay: std::time::Duration::from_millis(1000),
                headers,
                auth_token: None,
                user_agent: Some("mcp-connect/0.1.0".to_string()),
            };

            let fallbacks = vec![]; // No fallbacks for multiplexing (each server is HTTP only)
            let client = McpRemoteClient::new_with_config(transport_config, fallbacks);

            clients.insert(name, Arc::new(Mutex::new(client)));
        }

        info!("Initialized {} client(s)", clients.len());

        Ok(Self {
            clients,
            routing_config,
        })
    }

    /// Route a request to the appropriate server based on namespace.
    async fn route_request(&self, request: &str) -> Result<Option<String>> {
        // Parse the JSON-RPC request
        let request_json: serde_json::Value = serde_json::from_str(request)
            .context("Failed to parse request")?;

        // Check if this is a tools/list, resources/list, or prompts/list request
        if let Some(method) = request_json["method"].as_str() {
            match method {
                "tools/list" => return self.aggregate_tools().await.map(Some),
                "resources/list" => return self.aggregate_resources().await.map(Some),
                "tools/call" => return self.route_tool_call(request_json).await.map(Some),
                "resources/read" => return self.route_resource_read(request_json).await.map(Some),
                "initialize" => return self.handle_initialize(request_json).await.map(Some),
                _ => {
                    // For other methods, forward to first available server
                    return self.forward_to_any(request).await.map(Some);
                }
            }
        }

        // Default: forward to first server
        self.forward_to_any(request).await.map(Some)
    }

    /// Aggregate tools from all servers with namespace prefixes.
    async fn aggregate_tools(&self) -> Result<String> {
        let mut all_tools = Vec::new();

        for (server_name, client) in &self.clients {
            let mut client_guard = client.lock().await;

            // Ensure client is connected
            if let Err(e) = client_guard.connect().await {
                warn!("Failed to connect to server '{}': {}", server_name, e);
                continue;
            }

            // Get tools from this server
            match client_guard.list_tools().await {
                Ok(tools_result) => {
                    if let Some(tools_array) = tools_result["tools"].as_array() {
                        for tool in tools_array {
                            let mut prefixed_tool = tool.clone();

                            // Add namespace prefix to tool name
                            if let Some(tool_obj) = prefixed_tool.as_object_mut() {
                                if let Some(name) = tool_obj.get("name").and_then(|n| n.as_str()) {
                                    let prefixed_name = format!(
                                        "{}{}{}",
                                        server_name,
                                        self.routing_config.separator,
                                        name
                                    );
                                    tool_obj.insert("name".to_string(), serde_json::json!(prefixed_name));
                                }
                            }

                            all_tools.push(prefixed_tool);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to list tools from '{}': {}", server_name, e);
                }
            }
        }

        // Build response
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": all_tools
            }
        });

        Ok(serde_json::to_string(&response)?)
    }

    /// Aggregate resources from all servers with namespace prefixes.
    async fn aggregate_resources(&self) -> Result<String> {
        // Similar to aggregate_tools but for resources
        let mut all_resources = Vec::new();

        for (server_name, client) in &self.clients {
            let mut client_guard = client.lock().await;

            if let Err(e) = client_guard.connect().await {
                warn!("Failed to connect to server '{}': {}", server_name, e);
                continue;
            }

            match client_guard.list_resources().await {
                Ok(resources_result) => {
                    if let Some(resources_array) = resources_result["resources"].as_array() {
                        for resource in resources_array {
                            let mut prefixed_resource = resource.clone();

                            if let Some(resource_obj) = prefixed_resource.as_object_mut() {
                                if let Some(uri) = resource_obj.get("uri").and_then(|u| u.as_str()) {
                                    let prefixed_uri = format!(
                                        "{}{}{}",
                                        server_name,
                                        self.routing_config.separator,
                                        uri
                                    );
                                    resource_obj.insert("uri".to_string(), serde_json::json!(prefixed_uri));
                                }
                            }

                            all_resources.push(prefixed_resource);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to list resources from '{}': {}", server_name, e);
                }
            }
        }

        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "resources": all_resources
            }
        });

        Ok(serde_json::to_string(&response)?)
    }


    /// Route a tool call to the appropriate server.
    async fn route_tool_call(&self, request: serde_json::Value) -> Result<String> {
        let tool_name = request["params"]["name"]
            .as_str()
            .context("Missing tool name")?;

        let (server_name, actual_tool_name) = self.split_namespace(tool_name)?;

        // Get the client for this server
        let client = self.clients
            .get(server_name)
            .context(format!("Server '{}' not found", server_name))?;

        let mut client_guard = client.lock().await;
        client_guard.connect().await?;

        // Modify request to use actual tool name (without prefix)
        let mut modified_request = request.clone();
        modified_request["params"]["name"] = serde_json::json!(actual_tool_name);

        let request_str = serde_json::to_string(&modified_request)?;
        let response = client_guard.send_request(&request_str).await?;

        Ok(response)
    }

    /// Route a resource read to the appropriate server.
    async fn route_resource_read(&self, request: serde_json::Value) -> Result<String> {
        let uri = request["params"]["uri"]
            .as_str()
            .context("Missing resource URI")?;

        let (server_name, actual_uri) = self.split_namespace(uri)?;

        let client = self.clients
            .get(server_name)
            .context(format!("Server '{}' not found", server_name))?;

        let mut client_guard = client.lock().await;
        client_guard.connect().await?;

        let mut modified_request = request.clone();
        modified_request["params"]["uri"] = serde_json::json!(actual_uri);

        let request_str = serde_json::to_string(&modified_request)?;
        let response = client_guard.send_request(&request_str).await?;

        Ok(response)
    }

    /// Handle initialize request - forward to all servers and merge capabilities.
    async fn handle_initialize(&self, request: serde_json::Value) -> Result<String> {
        // For now, just forward to the first server
        // TODO: Properly aggregate capabilities from all servers
        self.forward_to_any(&serde_json::to_string(&request)?).await
    }

    /// Forward request to any available server (used for initialize, ping, etc.).
    async fn forward_to_any(&self, request: &str) -> Result<String> {
        // Try first available server
        for (name, client) in &self.clients {
            let mut client_guard = client.lock().await;

            match client_guard.connect().await {
                Ok(_) => {
                    info!("Forwarding request to server: {}", name);
                    return client_guard.send_request(request).await
                        .map_err(|e| anyhow::anyhow!("Request failed: {}", e));
                }
                Err(e) => {
                    warn!("Failed to connect to '{}': {}", name, e);
                    continue;
                }
            }
        }

        anyhow::bail!("No servers available")
    }

    /// Split a namespaced identifier into (server_name, actual_name).
    fn split_namespace<'a>(&self, identifier: &'a str) -> Result<(&'a str, &'a str)> {
        let parts: Vec<&str> = identifier.splitn(2, &self.routing_config.separator).collect();

        if parts.len() != 2 {
            anyhow::bail!(
                "Invalid identifier format: '{}'. Expected: server_name{}identifier",
                identifier,
                self.routing_config.separator
            );
        }

        Ok((parts[0], parts[1]))
    }
}

// Implement the ProxyStrategy trait for MultiplexingStrategy
#[async_trait::async_trait]
impl mcp_proxy::strategy::ProxyStrategy for MultiplexingStrategy {
    async fn handle_request(&self, request: &str) -> mcp_proxy::error::Result<Option<String>> {
        self.route_request(request)
            .await
            .map_err(|e| mcp_proxy::error::ProxyError::Strategy(e.to_string()))
    }

    async fn initialize(&self) -> mcp_proxy::error::Result<()> {
        // Initialize all clients
        for (name, client) in &self.clients {
            let mut client_guard = client.lock().await;
            if let Err(e) = client_guard.connect().await {
                warn!("Failed to initialize client '{}': {}", name, e);
            } else {
                if let Err(e) = client_guard.initialize().await {
                    warn!("Failed to initialize MCP protocol for '{}': {}", name, e);
                } else {
                    info!("Initialized client: {}", name);
                }
            }
        }

        Ok(())
    }

    async fn shutdown(&self) -> mcp_proxy::error::Result<()> {
        for (name, client) in &self.clients {
            let mut client_guard = client.lock().await;
            if let Err(e) = client_guard.disconnect().await {
                warn!("Error disconnecting '{}': {}", name, e);
            } else {
                info!("Disconnected client: {}", name);
            }
        }

        Ok(())
    }
}
