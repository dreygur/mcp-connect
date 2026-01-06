//! Proxy command handlers for MCP Connect.

use anyhow::Result;
use mcp_client::{McpRemoteClient, OAuthDiscovery, OAuthRequirement};
use mcp_proxy::{stdio_proxy::StdioProxyBuilder, strategy::{ForwardingStrategy, LoadBalancingStrategy}};
use mcp_types::{TransportType, McpClient, LogLevel};
use serde_json::json;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

use crate::transport;

fn send_notification(level: LogLevel, message: &str) {
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/message",
        "params": { "level": level, "logger": "mcp-proxy", "data": message }
    });
    if let Ok(json_str) = serde_json::to_string(&notification) {
        println!("{}", json_str);
        let _ = io::stdout().flush();
    }
}

/// Get OAuth token for endpoint - handles cache, refresh, and new auth
async fn get_oauth_token(
    endpoint: &str,
    oauth_client_id: Option<String>,
    oauth_client_secret: Option<String>,
    oauth_redirect_port: u16,
) -> Result<Option<String>> {
    // Check cached token
    if let Some((cached, refresh, client_id, secret)) = OAuthDiscovery::load_cached_token(endpoint) {
        if !OAuthDiscovery::is_token_expired(endpoint) {
            info!("Using cached OAuth token");
            return Ok(Some(cached));
        }
        // Try refresh
        if let Some(ref r) = refresh {
            info!("Token expired, attempting refresh...");
            let discovery = OAuthDiscovery::with_credentials(
                oauth_client_id.clone().or(Some(client_id.clone())),
                oauth_client_secret.clone().or(secret.clone()),
                Some(oauth_redirect_port),
            )?;
            if let Ok(new_token) = discovery.refresh_token(endpoint, r, &client_id, secret.as_deref()).await {
                info!("Token refreshed successfully");
                return Ok(Some(new_token.access_token));
            }
            warn!("Token refresh failed, will re-authenticate");
        }
    }

    // Check if OAuth required and authenticate
    let discovery = OAuthDiscovery::with_credentials(oauth_client_id.clone(), oauth_client_secret.clone(), Some(oauth_redirect_port))?;

    match discovery.check_oauth_required(endpoint).await {
        Ok(OAuthRequirement::NotRequired) => {
            info!("OAuth not required");
            Ok(None)
        }
        Ok(OAuthRequirement::Required(metadata)) => {
            info!("OAuth required, starting authentication...");
            send_notification(LogLevel::Info, "OAuth authentication required");

            let token = discovery.authenticate(&metadata, None).await
                .map_err(|e| anyhow::anyhow!("OAuth authentication failed: {}", e))?;

            // Use the actual client_id from token (from dynamic registration) or fallback
            let client_id = token.client_id.as_deref()
                .or(oauth_client_id.as_deref())
                .unwrap_or("dynamic");
            let client_secret = token.client_secret.as_deref()
                .or(oauth_client_secret.as_deref());
            OAuthDiscovery::save_token_to_cache(
                endpoint, &token.access_token, token.refresh_token.as_deref(),
                token.expires_at, client_id, client_secret,
            ).ok();

            send_notification(LogLevel::Info, "OAuth authentication successful");
            Ok(Some(token.access_token))
        }
        Ok(OAuthRequirement::RequiredButNoMetadata(reason)) => {
            warn!("OAuth may be required but metadata discovery failed: {}", reason);
            Ok(None)
        }
        Err(e) => {
            warn!("Failed to check OAuth requirement: {}", e);
            Ok(None)
        }
    }
}

pub async fn run_proxy(
    endpoint: String,
    fallbacks: Option<Vec<String>>,
    timeout: u64,
    retry_attempts: u32,
    retry_delay: u64,
    headers: Option<Vec<String>>,
    auth_token: Option<String>,
    api_key: Option<String>,
    user_agent: Option<String>,
    oauth_client_id: Option<String>,
    oauth_client_secret: Option<String>,
    oauth_redirect_port: u16,
    debug: bool,
) -> Result<()> {
    info!("Starting MCP Connect - endpoint: {}", endpoint);
    send_notification(LogLevel::Info, &format!("MCP Proxy starting: {}", endpoint));

    let fallback_transports = fallbacks
        .map(|f| transport::parse_fallback_transports(&f))
        .transpose()?
        .unwrap_or_else(|| vec![TransportType::Stdio, TransportType::Tcp]);

    let final_auth_token = if auth_token.is_some() {
        auth_token
    } else {
        get_oauth_token(&endpoint, oauth_client_id, oauth_client_secret, oauth_redirect_port).await?
    };

    let config = transport::build_transport_config(
        endpoint, timeout, retry_attempts, retry_delay, headers, final_auth_token, api_key, user_agent,
    )?;

    let client = McpRemoteClient::new_with_config(config, fallback_transports);
    let proxy = StdioProxyBuilder::new()
        .with_strategy(Arc::new(ForwardingStrategy::new(client)))
        .with_debug_mode(debug)
        .build()?;

    info!("Proxy ready, listening on STDIO");
    send_notification(LogLevel::Info, "MCP Proxy ready");
    proxy.run().await?;
    Ok(())
}

pub async fn run_load_balance(
    endpoints: Vec<String>,
    transport_str: String,
    timeout: u64,
    retry_attempts: u32,
    retry_delay: u64,
    headers: Option<Vec<String>>,
    auth_token: Option<String>,
    api_key: Option<String>,
    user_agent: Option<String>,
    debug: bool,
) -> Result<()> {
    info!("Starting MCP Load Balancing - endpoints: {:?}", endpoints);

    let transport_type = transport::parse_transport_type(&transport_str)?;
    let mut clients = Vec::new();

    for endpoint in &endpoints {
        let config = transport::build_transport_config(
            endpoint.clone(), timeout, retry_attempts, retry_delay,
            headers.clone(), auth_token.clone(), api_key.clone(), user_agent.clone(),
        )?;
        let client = McpRemoteClient::with_custom_transports(vec![(transport_type.clone(), config)]).await;
        clients.push(client);
    }

    if clients.is_empty() {
        return Err(anyhow::anyhow!("No clients configured"));
    }

    let proxy = StdioProxyBuilder::new()
        .with_strategy(Arc::new(LoadBalancingStrategy::new(clients)))
        .with_debug_mode(debug)
        .build()?;

    info!("Load balancing proxy ready");
    proxy.run().await?;
    Ok(())
}

pub async fn test_connection(
    endpoint: String,
    transport_str: String,
    timeout: u64,
    headers: Option<Vec<String>>,
    auth_token: Option<String>,
    api_key: Option<String>,
    user_agent: Option<String>,
) -> Result<()> {
    info!("Testing connection to: {}", endpoint);

    let transport_type = transport::parse_transport_type(&transport_str)?;
    let config = transport::build_transport_config(
        endpoint, timeout, 1, 100, headers, auth_token, api_key, user_agent,
    )?;

    let mut client = McpRemoteClient::with_custom_transports(vec![(transport_type, config)]).await;

    client.connect().await?;
    let init = client.initialize().await?;
    info!("Server: {} v{}", init.server_info.name, init.server_info.version);

    if let Ok(tools) = client.list_tools().await { info!("Tools: {}", tools); }
    if let Ok(res) = client.list_resources().await { info!("Resources: {}", res); }

    client.disconnect().await?;
    info!("Connection test successful!");
    Ok(())
}

pub async fn run_auth(
    endpoint: String,
    oauth_client_id: Option<String>,
    oauth_client_secret: Option<String>,
    oauth_redirect_port: u16,
    clear: bool,
) -> Result<()> {
    if clear {
        OAuthDiscovery::clear_cached_token(&endpoint)?;
        println!("Cleared cached token for {}", endpoint);
        return Ok(());
    }

    if let Some((token, _, _, _)) = OAuthDiscovery::load_cached_token(&endpoint) {
        println!("Found cached token: {}...", &token[..token.len().min(20)]);
        println!("To re-authenticate: mcp-connect auth --endpoint {} --clear", endpoint);
        return Ok(());
    }

    let discovery = OAuthDiscovery::with_credentials(oauth_client_id.clone(), oauth_client_secret.clone(), Some(oauth_redirect_port))?;

    match discovery.check_oauth_required(&endpoint).await {
        Ok(OAuthRequirement::NotRequired) => {
            println!("OAuth not required for this endpoint");
        }
        Ok(OAuthRequirement::Required(metadata)) => {
            println!("OAuth required, starting authentication...\n");
            let token = discovery.authenticate(&metadata, None).await
                .map_err(|e| anyhow::anyhow!("OAuth failed: {}", e))?;

            // Use the actual client_id from token (from dynamic registration) or fallback
            let client_id = token.client_id.as_deref()
                .or(oauth_client_id.as_deref())
                .unwrap_or("dynamic");
            let client_secret = token.client_secret.as_deref()
                .or(oauth_client_secret.as_deref());
            OAuthDiscovery::save_token_to_cache(
                &endpoint, &token.access_token, token.refresh_token.as_deref(),
                token.expires_at, client_id, client_secret,
            )?;

            println!("\n✓ Authentication successful!");
            println!("Use: claude mcp add myserver -- mcp-connect proxy --endpoint {}", endpoint);
        }
        Ok(OAuthRequirement::RequiredButNoMetadata(reason)) => {
            return Err(anyhow::anyhow!("OAuth required but discovery failed: {}", reason));
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to check OAuth: {}", e));
        }
    }
    Ok(())
}

pub async fn run_notification_demo(count: u32) -> Result<()> {
    for i in 1..=count {
        let (level, msg) = match i % 4 {
            1 => (LogLevel::Info, "info"),
            2 => (LogLevel::Warn, "warning"),
            3 => (LogLevel::Error, "error"),
            _ => (LogLevel::Debug, "debug"),
        };
        send_notification(level, &format!("Demo {} message {}", msg, i));
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    info!("Notification demo completed");
    Ok(())
}
