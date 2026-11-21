//! Serve command implementation.
//!
//! Run the multiplexing MCP server that serves all configured remote servers.

use anyhow::{Context, Result};
use mcp_client::{
    McpRemoteClient, OAuthDiscovery, OAuthMetadata,
    TokenProvider, ClientToken, StaticTokenProvider,
};
use mcp_config::ConfigManager;
use mcp_proxy::stdio_proxy::StdioProxyBuilder;
use mcp_types::{ConnectConfig, McpClient, OAuthConfig};
use rmcp::model::{Implementation, InitializeResult, ProtocolVersion, ServerCapabilities};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

const INITIALIZED_METHOD: &str = "notifications/initialized";

// Versions served to local clients, newest first. Capped at what the
// remotes are spoken to with, since messages pass through untranslated.
const SUPPORTED_VERSIONS: [ProtocolVersion; 3] = [
    ProtocolVersion::V_2025_06_18,
    ProtocolVersion::V_2025_03_26,
    ProtocolVersion::V_2024_11_05,
];

/// Pick the protocol version to answer a client's initialize with: the
/// requested one when supported, else our newest, as the MCP spec requires.
fn negotiate_version(request: &serde_json::Value) -> ProtocolVersion {
    let requested = serde_json::from_value::<ProtocolVersion>(
        request["params"]["protocolVersion"].clone(),
    );

    match requested {
        Ok(version) if SUPPORTED_VERSIONS.contains(&version) => version,
        _ => SUPPORTED_VERSIONS[0].clone(),
    }
}

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
    /// Initialize results of remotes with a live session; absent means
    /// the next use reconnects and initializes.
    sessions: Arc<Mutex<HashMap<String, InitializeResult>>>,
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

            let endpoint = &server_config.remote.url;

            // Handle OAuth authentication if configured
            let token_provider: Option<Arc<dyn TokenProvider>> =
                if let Some(ref oauth_config) = server_config.remote.oauth {
                    match Self::setup_oauth_provider(&name, endpoint, oauth_config).await {
                        Ok(provider) => {
                            info!("OAuth provider configured for server: {}", name);
                            Some(provider)
                        }
                        Err(e) => {
                            warn!("OAuth setup failed for server '{}': {}", name, e);
                            warn!("Continuing without OAuth - server may reject requests");
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
                auth_token: None,
                token_provider,
                user_agent: Some(format!("mcp-connect/{}", env!("CARGO_PKG_VERSION"))),
            };

            let fallbacks = vec![]; // No fallbacks for multiplexing (each server is HTTP only)
            let client = McpRemoteClient::new_with_config(transport_config, fallbacks);

            clients.insert(name, Arc::new(Mutex::new(client)));
        }

        info!("Initialized {} client(s)", clients.len());

        Ok(Self {
            clients,
            routing_config,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Set up OAuth provider for a server.
    async fn setup_oauth_provider(
        server_name: &str,
        endpoint: &str,
        oauth_config: &OAuthConfig,
    ) -> Result<Arc<dyn TokenProvider>> {
        // Try cached token first (with auto-refresh if expired)
        if let Some((token, refresh, client_id, secret)) = OAuthDiscovery::load_cached_token(endpoint) {
            if !OAuthDiscovery::is_token_expired(endpoint) {
                info!("Using cached OAuth token for: {}", server_name);
                return Ok(Arc::new(StaticTokenProvider::new(token)));
            }
            // Try refresh
            if let Some(ref r) = refresh {
                let discovery = OAuthDiscovery::with_credentials(Some(client_id.clone()), secret.clone(), None)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                if let Ok(new_token) = discovery.refresh_token(endpoint, r, &client_id, secret.as_deref()).await {
                    info!("Token refreshed for: {}", server_name);
                    return Ok(Arc::new(StaticTokenProvider::new(new_token.access_token)));
                }
            }
        }

        // Full OAuth flow
        let token = Self::perform_oauth_flow(server_name, endpoint, oauth_config).await?;
        Ok(Arc::new(StaticTokenProvider::new(token.access_token)))
    }

    /// Ensure a client is connected and initialized. The only place that
    /// connects: connect() opens a new transport, which drops the session.
    async fn ensure_client_connected(&self, server_name: &str, client: &Arc<Mutex<McpRemoteClient>>) -> Result<bool> {
        if self.sessions.lock().await.contains_key(server_name) {
            return Ok(true);
        }

        // Try to connect
        let mut client_guard = client.lock().await;
        match client_guard.connect().await {
            Ok(()) => {
                // Initialize the client
                match client_guard.initialize().await {
                    Ok(init_result) => {
                        drop(client_guard);
                        let mut sessions = self.sessions.lock().await;
                        sessions.insert(server_name.to_string(), init_result);
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
        self.sessions.lock().await.remove(server_name);
    }

    /// Perform full OAuth flow for a server that requires authentication.
    async fn perform_oauth_flow(server_name: &str, endpoint: &str, oauth_config: &OAuthConfig) -> Result<ClientToken> {
        info!("Starting OAuth flow for server: {}", server_name);

        // Extract port from redirect URL
        let redirect_url = url::Url::parse(&oauth_config.redirect_url)
            .context("Invalid redirect URL in OAuth config")?;
        let port = redirect_url.port().unwrap_or(8085);

        // Create discovery with explicit credentials
        let discovery = OAuthDiscovery::with_credentials(
            Some(oauth_config.client_id.clone()),
            oauth_config.client_secret.clone(),
            Some(port),
        ).map_err(|e| anyhow::anyhow!("{}", e))?;

        // Build metadata from config
        let metadata = OAuthMetadata {
            authorization_endpoint: oauth_config.auth_url.clone(),
            token_endpoint: oauth_config.token_url.clone(),
            revocation_endpoint: None,
            registration_endpoint: None,
            response_types_supported: vec!["code".to_string()],
            grant_types_supported: vec!["authorization_code".to_string(), "refresh_token".to_string()],
            code_challenge_methods_supported: if oauth_config.use_pkce { vec!["S256".to_string()] } else { vec![] },
            scopes_supported: oauth_config.scopes.clone(),
        };

        // Use OAuthDiscovery to handle the browser flow
        let token = discovery.authenticate(&metadata, Some(oauth_config.scopes.clone())).await
            .map_err(|e| anyhow::anyhow!("OAuth authentication failed: {}", e))?;

        // Save token to cache - prefer token's client_id (from dynamic registration) over config
        let client_id = token.client_id.as_deref().unwrap_or(&oauth_config.client_id);
        let client_secret = token.client_secret.as_deref().or(oauth_config.client_secret.as_deref());
        if let Err(e) = OAuthDiscovery::save_token_to_cache(
            endpoint,
            &token.access_token,
            token.refresh_token.as_deref(),
            token.expires_at,
            client_id,
            client_secret,
        ) {
            warn!("Failed to cache OAuth token: {}", e);
        }

        Ok(token)
    }

    /// Route a request to the appropriate server based on namespace.
    async fn route_request(&self, request: &str) -> Result<Option<String>> {
        // Parse the JSON-RPC request
        let request_json: serde_json::Value =
            serde_json::from_str(request).context("Failed to parse request")?;

        // Extract request ID for response
        let request_id = request_json.get("id").cloned();

        // JSON-RPC notifications get no reply; anything written back would
        // be read by the client as a malformed response.
        if request_id.is_none() {
            // Each remote already got its own from client.initialize()
            if request_json["method"] == INITIALIZED_METHOD {
                return Ok(None);
            }
            if let Err(e) = self.forward_to_any(request).await {
                warn!("Failed to forward notification: {}", e);
            }
            return Ok(None);
        }

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

        if !self.ensure_client_connected(server_name, client).await? {
            anyhow::bail!("Server '{}' unavailable", server_name);
        }
        let mut client_guard = client.lock().await;

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

        if !self.ensure_client_connected(server_name, client).await? {
            anyhow::bail!("Server '{}' unavailable", server_name);
        }
        let mut client_guard = client.lock().await;

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

        // Reuse sessions opened at startup; only remotes without one initialize here
        let mut init_results = Vec::new();
        let mut server_infos = Vec::new();

        for (server_name, client) in &self.clients {
            if !self.ensure_client_connected(server_name, client).await? {
                continue;
            }
            if let Some(init_result) = self.sessions.lock().await.get(server_name) {
                init_results.push((server_name.clone(), init_result.clone()));
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
        // Logging is deprecated only after the 2025-06-18 spec we negotiate.
        #[allow(deprecated)]
        let merged_capabilities = ServerCapabilities::builder()
            .enable_logging() // Most servers support logging
            .enable_tools() // Most servers support tools
            .enable_resources() // Most servers support resources
            .build();

        // Create merged server info
        let merged_server_info = Implementation::new("mcp-connect", "0.1.0").with_title(format!(
            "MCP Connect Multiplexer ({} server(s))",
            init_results.len()
        ));

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

        // Build the merged initialize result
        let mut merged_result = InitializeResult::new(merged_capabilities)
            .with_protocol_version(negotiate_version(&request))
            .with_server_info(merged_server_info);
        merged_result.instructions = merged_instructions;

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
            if !self.ensure_client_connected(name, client).await? {
                continue;
            }

            info!("Forwarding request to server: {}", name);
            return client
                .lock()
                .await
                .send_request(request)
                .await
                .map_err(|e| anyhow::anyhow!("Request failed: {}", e));
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
        // Open every session up front; failures retry on first use
        for (name, client) in &self.clients {
            self.ensure_client_connected(name, client)
                .await
                .map_err(|e| mcp_proxy::error::ProxyError::Strategy(e.to_string()))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_strategy() -> MultiplexingStrategy {
        MultiplexingStrategy {
            clients: HashMap::new(),
            routing_config: Default::default(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[tokio::test]
    async fn test_notification_gets_no_reply() {
        let reply = mock_strategy()
            .route_request(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .await
            .unwrap();
        assert_eq!(reply, None);
    }

    type Log = Arc<Mutex<Vec<serde_json::Value>>>;

    async fn mock_reply(body: &[u8], log: &Log) -> String {
        let msg: serde_json::Value = serde_json::from_slice(body).unwrap();
        log.lock().await.push(msg.clone());

        let Some(id) = msg.get("id") else {
            return "HTTP/1.1 202 Accepted\r\ncontent-length: 0\r\n\r\n".to_string();
        };

        let result = match msg["method"].as_str() {
            Some("initialize") => serde_json::json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "serverInfo": {"name": "mock", "version": "1"}
            }),
            Some("tools/list") => serde_json::json!({"tools": []}),
            _ => serde_json::json!({}),
        };
        let out = serde_json::json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string();

        format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nmcp-session-id: s1\r\ncontent-length: {}\r\n\r\n{}",
            out.len(),
            out
        )
    }

    // Streamable HTTP remote that logs every JSON-RPC message it receives.
    async fn mock_remote() -> (String, Log) {
        use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/mcp", listener.local_addr().unwrap());
        let log: Log = Default::default();
        let log_task = log.clone();

        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let log = log_task.clone();

                tokio::spawn(async move {
                    let mut stream = BufReader::new(stream);
                    loop {
                        let mut line = String::new();
                        if stream.read_line(&mut line).await.unwrap_or(0) == 0 {
                            return;
                        }

                        let mut len = 0;
                        loop {
                            let mut header = String::new();
                            stream.read_line(&mut header).await.unwrap();
                            if header == "\r\n" {
                                break;
                            }
                            let header = header.to_ascii_lowercase();
                            if let Some(value) = header.strip_prefix("content-length:") {
                                len = value.trim().parse().unwrap();
                            }
                        }
                        let mut body = vec![0; len];
                        stream.read_exact(&mut body).await.unwrap();

                        let reply = if line.starts_with("POST") {
                            mock_reply(&body, &log).await
                        } else {
                            "HTTP/1.1 405 Method Not Allowed\r\ncontent-length: 0\r\n\r\n".to_string()
                        };
                        stream.get_mut().write_all(reply.as_bytes()).await.unwrap();
                    }
                });
            }
        });

        (url, log)
    }

    #[tokio::test]
    async fn test_remote_initialized_once() {
        use mcp_proxy::strategy::ProxyStrategy;

        let (url, log) = mock_remote().await;
        let config = mcp_client::transport::TransportConfig {
            endpoint: url,
            ..Default::default()
        };
        let client = McpRemoteClient::new_with_config(config, vec![]);
        let mut strategy = mock_strategy();
        strategy.clients.insert("mock".to_string(), Arc::new(Mutex::new(client)));

        strategy.initialize().await.unwrap();
        for request in [
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"mock/echo","arguments":{}}}"#,
        ] {
            strategy.route_request(request).await.unwrap();
        }

        let methods: Vec<String> = log
            .lock()
            .await
            .iter()
            .map(|m| m["method"].as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(
            methods,
            ["initialize", "notifications/initialized", "tools/list", "tools/call"]
        );
    }

    fn init_request(version: &str) -> serde_json::Value {
        serde_json::json!({"params": {"protocolVersion": version}})
    }

    #[test]
    fn test_negotiate_supported_version() {
        let version = negotiate_version(&init_request("2025-03-26"));
        assert_eq!(version, ProtocolVersion::V_2025_03_26);
    }

    #[test]
    fn test_negotiate_unsupported_version() {
        let version = negotiate_version(&init_request("2099-01-01"));
        assert_eq!(version, ProtocolVersion::V_2025_06_18);
    }

    #[test]
    fn test_negotiate_missing_version() {
        let version = negotiate_version(&serde_json::json!({}));
        assert_eq!(version, ProtocolVersion::V_2025_06_18);
    }
}
