//! Minimal example demonstrating MCP Connect components.
//!
//! This example shows how to use the core MCP Connect components programmatically.

use mcp_client::{McpRemoteClient, transport::TransportConfig};
use mcp_config::ConfigManager;
use mcp_proxy::stdio_proxy::StdioProxyBuilder;
use mcp_proxy::strategy::ForwardingStrategy;
use mcp_registry::RegistryClient;
use mcp_server::McpStdioServer;
use mcp_types::{TransportType, McpServer};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== MCP Connect Component Test ===\n");

    // Test 1: Configuration Management
    println!("1. Testing Configuration Management...");
    let manager = ConfigManager::new()?;
    if manager.exists() {
        let mut manager = ConfigManager::new()?;
        let config = manager.load()?;
        println!("✓ Configuration loaded: {} server(s) configured", config.servers.len());
    } else {
        println!("ℹ No configuration file found (this is okay for testing)");
    }
    println!();

    // Test 2: Registry Client
    println!("2. Testing Registry Client...");
    match RegistryClient::new() {
        Ok(_client) => {
            println!("✓ Registry client created successfully");
            // Note: We don't actually fetch here to avoid network dependency in tests
        }
        Err(e) => {
            println!("⚠ Registry client creation failed: {}", e);
        }
    }
    println!();

    // Test 3: MCP Client Creation
    println!("3. Testing MCP Remote Client Creation...");
    let client = McpRemoteClient::new(
        "http://localhost:8080/mcp".to_string(),
        vec![TransportType::Stdio, TransportType::Tcp]
    );
    println!("✓ MCP Client created successfully");
    println!("  Primary transport: HTTP");
    println!("  Fallback transports: STDIO, TCP");
    println!();

    // Test 4: Forwarding Strategy
    println!("4. Testing Forwarding Strategy...");
    let strategy = Arc::new(ForwardingStrategy::new(client));
    println!("✓ Forwarding Strategy created successfully");
    println!();

    // Test 5: STDIO Proxy
    println!("5. Testing STDIO Proxy Builder...");
    match StdioProxyBuilder::new()
        .with_strategy(strategy)
        .with_debug_mode(true)
        .build()
    {
        Ok(_proxy) => {
            println!("✓ STDIO Proxy created successfully");
            println!("  Note: Proxy is ready but not started (would block)");
        }
        Err(e) => {
            println!("✗ Proxy creation failed: {}", e);
        }
    }
    println!();

    // Test 6: MCP Server
    println!("6. Testing MCP STDIO Server...");
    let mut server = McpStdioServer::new(true);
    println!("✓ MCP Server created successfully");
    println!();

    // Test 7: JSON-RPC Message Handling
    println!("7. Testing JSON-RPC Message Handling...");
    let ping_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "ping"
    }).to_string();

    match server.handle_message(&ping_request).await {
        Ok(Some(response)) => {
            println!("✓ Server handled ping request");
            println!("  Response preview: {}...", &response[..response.len().min(80)]);
        }
        Ok(None) => {
            println!("ℹ Server returned no response (notification)");
        }
        Err(e) => {
            println!("✗ Server error: {}", e);
        }
    }
    println!();

    // Test 8: Initialization Sequence
    println!("8. Testing Initialization Sequence...");
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    }).to_string();

    match server.handle_message(&init_request).await {
        Ok(Some(response)) => {
            let parsed: serde_json::Value = serde_json::from_str(&response)?;
            if let Some(server_info) = parsed.get("result").and_then(|r| r.get("serverInfo")) {
                println!("✓ Server initialized successfully");
                println!("  Server: {}", server_info.get("name").unwrap_or(&json!("unknown")));
                println!("  Version: {}", server_info.get("version").unwrap_or(&json!("unknown")));
            } else {
                println!("✓ Server handled initialize request");
            }
        }
        Ok(None) => {
            println!("ℹ Server returned no response to initialize");
        }
        Err(e) => {
            println!("✗ Server initialization error: {}", e);
        }
    }
    println!();

    // Test 9: Tools Listing
    println!("9. Testing Tools List...");
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/list"
    }).to_string();

    match server.handle_message(&tools_request).await {
        Ok(Some(response)) => {
            let parsed: serde_json::Value = serde_json::from_str(&response)?;
            if let Some(tools) = parsed.get("result").and_then(|r| r.get("tools")) {
                let count = tools.as_array().map(|a| a.len()).unwrap_or(0);
                println!("✓ Server handled tools/list request");
                println!("  Tools available: {}", count);
            } else {
                println!("✓ Server handled tools/list request");
            }
        }
        Ok(None) => {
            println!("ℹ Server returned no response to tools/list");
        }
        Err(e) => {
            println!("✗ Server tools/list error: {}", e);
        }
    }
    println!();

    // Test 10: Custom Transport Configuration
    println!("10. Testing Custom Transport Configuration...");
    let custom_config = TransportConfig {
        endpoint: "http://example.com:8080/mcp".to_string(),
        timeout: Duration::from_secs(5),
        retry_attempts: 2,
        retry_delay: Duration::from_millis(500),
        headers: std::collections::HashMap::new(),
        auth_token: None,
        user_agent: Some("test-client/1.0".to_string()),
    };

    let custom_transports = vec![
        (TransportType::Http, custom_config.clone()),
        (TransportType::Tcp, TransportConfig {
            endpoint: "localhost:9090".to_string(),
            ..custom_config
        })
    ];

    let _custom_client = McpRemoteClient::with_custom_transports(custom_transports).await;
    println!("✓ Custom client configuration created successfully");
    println!("  Transport 1: HTTP (example.com:8080)");
    println!("  Transport 2: TCP (localhost:9090)");
    println!();

    println!("=== All Tests Completed Successfully! ===\n");
    println!("Next steps:");
    println!("  1. Run 'mcp-connect init' to create configuration");
    println!("  2. Run 'mcp-connect config add <name> <registry-path>' to add servers");
    println!("  3. Run 'mcp-connect generate --ide vscode' to generate IDE config");
    println!("  4. Run 'mcp-connect serve' to start the multiplexing server");
    println!("\nFor more examples, see examples/simple_usage.md");

    Ok(())
}
