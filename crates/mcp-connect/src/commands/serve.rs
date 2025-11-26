//! Serve command implementation.
//!
//! Run the multiplexing MCP server that serves all configured remote servers.

use anyhow::{Context, Result};
use mcp_client::{McpRemoteClient, OAuthClient, OAuthClientConfig};
use mcp_config::ConfigManager;
use mcp_proxy::stdio_proxy::StdioProxyBuilder;
use mcp_types::{ConnectConfig, McpClient, OAuthConfig};
use rmcp::model::{Implementation, InitializeResult, ProtocolVersion, ServerCapabilities};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

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
    /// Track which clients are connected and initialized
    connection_state: Arc<Mutex<HashMap<String, bool>>>,
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

            // Handle OAuth authentication if configured
            let auth_token = if let Some(ref oauth_config) = server_config.remote.oauth {
                match Self::perform_oauth_flow(&name, oauth_config).await {
                    Ok(token) => {
                        info!("OAuth authentication successful for server: {}", name);
                        Some(format!("Bearer {}", token))
                    }
                    Err(e) => {
                        warn!("OAuth authentication failed for server '{}': {}", name, e);
                        warn!("Continuing without OAuth token - server may reject requests");
                        None
                    }
                }
            } else {
                None
            };

            let transport_config = mcp_client::transport::TransportConfig {
                endpoint: server_config.remote.url.clone(),
                timeout: std::time::Duration::from_secs(server_config.timeout.unwrap_or(30)),
                retry_attempts: server_config.retry_attempts.unwrap_or(3),
                retry_delay: std::time::Duration::from_millis(1000),
                headers,
                auth_token,
                user_agent: Some("mcp-connect/0.1.0".to_string()),
            };

            let fallbacks = vec![]; // No fallbacks for multiplexing (each server is HTTP only)
            let client = McpRemoteClient::new_with_config(transport_config, fallbacks);

            clients.insert(name, Arc::new(Mutex::new(client)));
        }

        info!("Initialized {} client(s)", clients.len());

        // Initialize connection state tracking
        let connection_state = Arc::new(Mutex::new(HashMap::new()));

        Ok(Self {
            clients,
            routing_config,
            connection_state,
        })
    }

    /// Ensure a client is connected, using cached connection state
    async fn ensure_client_connected(&self, server_name: &str, client: &Arc<Mutex<McpRemoteClient>>) -> Result<bool> {
        // Check cached connection state first
        {
            let state = self.connection_state.lock().await;
            if state.get(server_name) == Some(&true) {
                return Ok(true);
            }
        }

        // Try to connect
        let mut client_guard = client.lock().await;
        match client_guard.connect().await {
            Ok(()) => {
                // Initialize the client
                match client_guard.initialize().await {
                    Ok(_) => {
                        drop(client_guard);
                        let mut state = self.connection_state.lock().await;
                        state.insert(server_name.to_string(), true);
                        info!("Connected and initialized client: {}", server_name);
                        Ok(true)
                    }
                    Err(e) => {
                        warn!("Failed to initialize client '{}': {}", server_name, e);
                        Ok(false)
                    }
                }
            }
            Err(e) => {
                warn!("Failed to connect to server '{}': {}", server_name, e);
                Ok(false)
            }
        }
    }

    /// Mark a client as disconnected (for reconnection on next use)
    async fn mark_disconnected(&self, server_name: &str) {
        let mut state = self.connection_state.lock().await;
        state.insert(server_name.to_string(), false);
    }

    /// Perform OAuth flow for a server that requires authentication.
    ///
    /// This starts a local HTTP server to receive the OAuth callback,
    /// opens the browser for user authentication, and exchanges the
    /// authorization code for an access token.
    async fn perform_oauth_flow(server_name: &str, oauth_config: &OAuthConfig) -> Result<String> {
        use tokio::net::TcpListener;

        info!("Starting OAuth flow for server: {}", server_name);

        // Convert from mcp_types::OAuthConfig to mcp_client::OAuthClientConfig
        let client_config = OAuthClientConfig {
            client_id: oauth_config.client_id.clone(),
            client_secret: oauth_config.client_secret.clone(),
            auth_url: oauth_config.auth_url.clone(),
            token_url: oauth_config.token_url.clone(),
            redirect_url: oauth_config.redirect_url.clone(),
            scopes: oauth_config.scopes.clone(),
            use_pkce: oauth_config.use_pkce,
        };

        let oauth_client = OAuthClient::new(client_config)
            .map_err(|e| anyhow::anyhow!("Failed to create OAuth client: {}", e))?;

        // Generate authorization URL
        let auth_url = oauth_client
            .generate_auth_url()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to generate auth URL: {}", e))?;

        // Extract port from redirect URL for the callback server
        let redirect_url = url::Url::parse(&oauth_config.redirect_url)
            .context("Invalid redirect URL in OAuth config")?;
        let port = redirect_url.port().unwrap_or(8085);
        let callback_path = redirect_url.path();

        // Start local server to receive callback
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
            .await
            .context("Failed to start OAuth callback server")?;

        info!("OAuth callback server listening on port {}", port);

        // Print instructions for user
        eprintln!("\n╔════════════════════════════════════════════════════════════════╗");
        eprintln!("║                    OAuth Authentication Required                ║");
        eprintln!("╠════════════════════════════════════════════════════════════════╣");
        eprintln!("║ Server: {:<54} ║", server_name);
        eprintln!("╠════════════════════════════════════════════════════════════════╣");
        eprintln!("║ Please open the following URL in your browser:                 ║");
        eprintln!("╚════════════════════════════════════════════════════════════════╝");
        eprintln!("\n{}\n", auth_url);
        eprintln!("Waiting for authorization callback...\n");

        // Try to open browser automatically
        if let Err(e) = open::that(&auth_url) {
            debug!("Could not open browser automatically: {}", e);
        }

        // Wait for callback with timeout
        let callback_result = tokio::time::timeout(
            std::time::Duration::from_secs(300), // 5 minute timeout
            Self::wait_for_oauth_callback(&listener, callback_path),
        )
        .await
        .context("OAuth authentication timed out")?
        .context("Failed to receive OAuth callback")?;

        let (code, state) = callback_result;

        // Exchange code for token
        let token = oauth_client
            .exchange_code(&code, &state)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to exchange authorization code: {}", e))?;

        eprintln!("✓ OAuth authentication successful!\n");

        Ok(token.access_token)
    }

    /// Wait for OAuth callback on the local server.
    async fn wait_for_oauth_callback(
        listener: &tokio::net::TcpListener,
        expected_path: &str,
    ) -> Result<(String, String)> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let (mut socket, _) = listener.accept().await?;
        let (reader, mut writer) = socket.split();
        let mut reader = BufReader::new(reader);

        // Read the HTTP request
        let mut request_line = String::new();
        reader.read_line(&mut request_line).await?;

        debug!("Received OAuth callback request: {}", request_line.trim());

        // Parse the request to extract code and state
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            anyhow::bail!("Invalid HTTP request");
        }

        let path_and_query = parts[1];

        // Parse query parameters
        let url = url::Url::parse(&format!("http://localhost{}", path_and_query))
            .context("Failed to parse callback URL")?;

        // Check if this is the expected callback path
        if !url.path().starts_with(expected_path.trim_end_matches('/')) {
            anyhow::bail!("Unexpected callback path: {}", url.path());
        }

        let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

        // Check for error response
        if let Some(error) = params.get("error") {
            let description = params
                .get("error_description")
                .map(|s| s.as_str())
                .unwrap_or("Unknown error");
            anyhow::bail!("OAuth error: {} - {}", error, description);
        }

        let code = params
            .get("code")
            .context("Missing authorization code in callback")?
            .clone();

        let state = params
            .get("state")
            .context("Missing state parameter in callback")?
            .clone();

        // Send success response to browser
        let response = "HTTP/1.1 200 OK\r\n\
            Content-Type: text/html\r\n\
            Connection: close\r\n\r\n\
            <!DOCTYPE html>\
            <html><head><title>Authentication Successful</title></head>\
            <body style=\"font-family: system-ui; text-align: center; padding: 50px;\">\
            <h1>✓ Authentication Successful</h1>\
            <p>You can close this window and return to the terminal.</p>\
            </body></html>";

        writer.write_all(response.as_bytes()).await?;
        writer.flush().await?;

        Ok((code, state))
    }

    /// Route a request to the appropriate server based on namespace.
    async fn route_request(&self, request: &str) -> Result<Option<String>> {
        // Parse the JSON-RPC request
        let request_json: serde_json::Value =
            serde_json::from_str(request).context("Failed to parse request")?;

        // Extract request ID for response
        let request_id = request_json.get("id").cloned();

        // Check if this is a tools/list, resources/list, or prompts/list request
        if let Some(method) = request_json["method"].as_str() {
            match method {
                "tools/list" => return self.aggregate_tools(request_id).await.map(Some),
                "resources/list" => return self.aggregate_resources(request_id).await.map(Some),
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
    async fn aggregate_tools(&self, request_id: Option<serde_json::Value>) -> Result<String> {
        let mut all_tools = Vec::new();

        for (server_name, client) in &self.clients {
            // Use connection state tracking instead of reconnecting every time
            if !self.ensure_client_connected(server_name, client).await? {
                continue;
            }

            let client_guard = client.lock().await;

            // Get tools from this server
            match client_guard.list_tools().await {
                Ok(tools_result) => {
                    if let Some(tools_array) = tools_result["tools"].as_array() {
                        // Pre-allocate capacity to avoid reallocations
                        all_tools.reserve(tools_array.len());

                        for tool in tools_array {
                            // Only clone if we need to modify the name
                            if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                                let mut prefixed_tool = tool.clone();
                                if let Some(tool_obj) = prefixed_tool.as_object_mut() {
                                    // Build prefixed name
                                    let prefixed_name = format!(
                                        "{}{}{}",
                                        server_name, self.routing_config.separator, name
                                    );
                                    tool_obj.insert("name".to_string(), serde_json::Value::String(prefixed_name));
                                }
                                all_tools.push(prefixed_tool);
                            } else {
                                // No name field, push as-is (clone required)
                                all_tools.push(tool.clone());
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to list tools from '{}': {}", server_name, e);
                    // Mark as disconnected for retry on next request
                    self.mark_disconnected(server_name).await;
                }
            }
        }

        // Build response with original request ID
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": request_id.unwrap_or(serde_json::Value::Null),
            "result": {
                "tools": all_tools
            }
        });

        Ok(serde_json::to_string(&response)?)
    }

    /// Aggregate resources from all servers with namespace prefixes.
    async fn aggregate_resources(&self, request_id: Option<serde_json::Value>) -> Result<String> {
        // Similar to aggregate_tools but for resources
        let mut all_resources = Vec::new();

        for (server_name, client) in &self.clients {
            // Use connection state tracking instead of reconnecting every time
            if !self.ensure_client_connected(server_name, client).await? {
                continue;
            }

            let client_guard = client.lock().await;

            match client_guard.list_resources().await {
                Ok(resources_result) => {
                    if let Some(resources_array) = resources_result["resources"].as_array() {
                        // Pre-allocate capacity to avoid reallocations
                        all_resources.reserve(resources_array.len());

                        for resource in resources_array {
                            // Only clone if we need to modify the URI
                            if let Some(uri) = resource.get("uri").and_then(|u| u.as_str()) {
                                let mut prefixed_resource = resource.clone();
                                if let Some(resource_obj) = prefixed_resource.as_object_mut() {
                                    let prefixed_uri = format!(
                                        "{}{}{}",
                                        server_name, self.routing_config.separator, uri
                                    );
                                    resource_obj.insert("uri".to_string(), serde_json::Value::String(prefixed_uri));
                                }
                                all_resources.push(prefixed_resource);
                            } else {
                                // No URI field, push as-is (clone required)
                                all_resources.push(resource.clone());
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to list resources from '{}': {}", server_name, e);
                    // Mark as disconnected for retry on next request
                    self.mark_disconnected(server_name).await;
                }
            }
        }

        // Build response with original request ID
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": request_id.unwrap_or(serde_json::Value::Null),
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
        let client = self
            .clients
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

        let client = self
            .clients
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
        let request_id = request
            .get("id")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        // Initialize all servers and collect their capabilities
        let mut init_results = Vec::new();
        let mut server_infos = Vec::new();

        for (server_name, client) in &self.clients {
            let mut client_guard = client.lock().await;

            // Connect and initialize each server
            if let Err(e) = client_guard.connect().await {
                warn!(
                    "Failed to connect to server '{}' during initialize: {}",
                    server_name, e
                );
                continue;
            }

            match client_guard.initialize().await {
                Ok(init_result) => {
                    info!("Initialized server '{}' with capabilities", server_name);
                    init_results.push((server_name.clone(), init_result));
                }
                Err(e) => {
                    warn!("Failed to initialize server '{}': {}", server_name, e);
                }
            }
        }

        if init_results.is_empty() {
            anyhow::bail!("No servers could be initialized");
        }

        // Extract server info from all results
        for (_server_name, init_result) in &init_results {
            server_infos.push(format!(
                "{} v{}",
                init_result.server_info.name, init_result.server_info.version
            ));
        }

        // Build merged capabilities
        // The builder pattern uses type-level state, so we can't conditionally build.
        // Since we're aggregating multiple servers, we'll enable all common capabilities
        // that are typically supported. This ensures the client knows what's available.
        // The actual capabilities are determined by what servers we can connect to.
        let merged_capabilities = ServerCapabilities::builder()
            .enable_logging() // Most servers support logging
            .enable_tools() // Most servers support tools
            .enable_resources() // Most servers support resources
            .build();

        // Create merged server info
        let merged_server_info = Implementation {
            name: "mcp-connect".to_string(),
            version: "0.1.0".to_string(),
            title: Some(format!(
                "MCP Connect Multiplexer ({} server(s))",
                init_results.len()
            )),
            icons: None,
            website_url: None,
        };

        // Combine instructions from all servers
        let mut instructions_parts = vec![format!(
            "MCP Connect multiplexing {} server(s): {}",
            init_results.len(),
            server_infos.join(", ")
        )];

        for (_, init_result) in &init_results {
            if let Some(instructions) = &init_result.instructions {
                if !instructions.trim().is_empty() {
                    instructions_parts.push(instructions.clone());
                }
            }
        }

        let merged_instructions = if instructions_parts.len() > 1 {
            Some(instructions_parts.join("\n\n"))
        } else {
            instructions_parts.first().cloned()
        };

        // Use the protocol version from the first server (they should all be the same)
        let protocol_version = init_results
            .first()
            .map(|(_, r)| r.protocol_version.clone())
            .unwrap_or_else(ProtocolVersion::default);

        // Build the merged initialize result
        let merged_result = InitializeResult {
            protocol_version,
            capabilities: merged_capabilities,
            server_info: merged_server_info,
            instructions: merged_instructions,
        };

        // Build response
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": merged_result
        });

        Ok(serde_json::to_string(&response)?)
    }

    /// Forward request to any available server (used for initialize, ping, etc.).
    async fn forward_to_any(&self, request: &str) -> Result<String> {
        // Try first available server
        for (name, client) in &self.clients {
            let mut client_guard = client.lock().await;

            match client_guard.connect().await {
                Ok(_) => {
                    info!("Forwarding request to server: {}", name);
                    return client_guard
                        .send_request(request)
                        .await
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
        let parts: Vec<&str> = identifier
            .splitn(2, &self.routing_config.separator)
            .collect();

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
