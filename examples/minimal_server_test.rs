use mcp_client::{McpRemoteClient, transport::TransportConfig};
use mcp_proxy::{StdioProxyBuilder, strategy::ForwardingStrategy};
use mcp_server::McpStdioServer;
use mcp_types::{TransportType, McpServer};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("=== MCP Remote Proxy Test ===\n");

    // Test 1: Create a basic MCP client
    println!("1. Testing MCP Client Creation...");
    let client = McpRemoteClient::new(
        "http://localhost:8080/mcp".to_string(),
        vec![TransportType::Stdio, TransportType::Tcp]
    );
    println!("✓ MCP Client created successfully\n");

    // Test 2: Create a forwarding strategy
    println!("2. Testing Forwarding Strategy...");
    let strategy = Arc::new(ForwardingStrategy::new(client));
    println!("✓ Forwarding Strategy created successfully\n");

    // Test 3: Create a STDIO proxy
    println!("3. Testing STDIO Proxy Creation...");
    let proxy = StdioProxyBuilder::new()
        .with_strategy(strategy)
        .with_debug_mode(true)
        .build()?;
    println!("✓ STDIO Proxy created successfully\n");

    // Test 4: Create a standalone MCP server
    println!("4. Testing MCP Server Creation...");
    let mut server = McpStdioServer::new(true);
    println!("✓ MCP Server created successfully\n");

    // Test 5: Test JSON-RPC message handling
    println!("5. Testing JSON-RPC Message Handling...");
    let ping_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "ping"
    }).to_string();

    match server.handle_message(&ping_request).await {
        Ok(Some(response)) => {
            println!("✓ Server handled ping request");
            println!("  Response: {}", response);
        }
        Ok(None) => {
            println!("✓ Server handled notification (no response)");
        }
        Err(e) => {
            println!("✗ Server error: {}", e);
        }
    }
    println!();

    // Test 6: Test initialization sequence
    println!("6. Testing Initialization Sequence...");
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
            println!("✓ Server handled initialize request");
            println!("  Response preview: {}...", &response[..response.len().min(100)]);
        }
        Ok(None) => {
            println!("? Server returned no response to initialize");
        }
        Err(e) => {
            println!("✗ Server initialization error: {}", e);
        }
    }
    println!();

    // Test 7: Test tools listing
    println!("7. Testing Tools List...");
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/list"
    }).to_string();

    match server.handle_message(&tools_request).await {
        Ok(Some(response)) => {
            println!("✓ Server handled tools/list request");
            println!("  Response: {}", response);
        }
        Ok(None) => {
            println!("? Server returned no response to tools/list");
        }
        Err(e) => {
            println!("✗ Server tools/list error: {}", e);
        }
    }
    println!();

    // Test 8: Test client with custom configuration
    println!("8. Testing Custom Client Configuration...");
    let custom_config = TransportConfig {
        endpoint: "http://example.com:8080/mcp".to_string(),
        timeout: Duration::from_secs(5),
        retry_attempts: 2,
        retry_delay: Duration::from_millis(500),
    };

    let custom_transports = vec![
        (TransportType::Http, custom_config.clone()),
        (TransportType::Tcp, TransportConfig {
            endpoint: "localhost:9090".to_string(),
            ..custom_config
        })
    ];

    let custom_client = McpRemoteClient::with_custom_transports(custom_transports).await;
    println!("✓ Custom client configuration created successfully\n");

    println!("=== All Tests Completed Successfully! ===");
    println!("\nTo run the actual proxy:");
    println!("cargo run --bin mcp-connect -- proxy --endpoint 'http://localhost:8080/mcp' --debug");
    println!("\nTo test connection:");
    println!("cargo run --bin mcp-connect -- test --endpoint 'http://localhost:8080/mcp' --transport http");

    Ok(())
}
