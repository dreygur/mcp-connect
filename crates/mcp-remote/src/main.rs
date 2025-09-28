//! # MCP Remote Proxy CLI
//!
//! Command-line interface for the Model Context Protocol (MCP) remote proxy system.
//!
//! This application bridges local MCP clients with remote MCP servers, providing:
//! - Multiple transport support (HTTP, STDIO, TCP)
//! - Authentication handling (Bearer tokens, API keys, OAuth 2.1)
//! - Fallback mechanisms and load balancing
//! - Comprehensive logging and debugging
//!
//! ## Usage
//!
//! Basic proxy operation:
//! ```bash
//! mcp-remote proxy --endpoint "https://api.example.com/mcp" --auth-token "your-token"
//! ```
//!
//! With fallbacks:
//! ```bash
//! mcp-remote proxy --endpoint "https://api.example.com/mcp" --fallbacks "stdio,tcp"
//! ```
//!
//! Load balancing:
//! ```bash
//! mcp-remote load-balance --endpoints "server1,server2,server3" --transport "http"
//! ```
//!
//! ## Commands
//!
//! - `proxy`: Run as STDIO proxy (main mode)
//! - `test`: Test connection to remote server
//! - `load-balance`: Distribute requests across multiple servers
//! - `notification-demo`: Test MCP notification system

use anyhow::Result;
use clap::{Parser, Subcommand};
use mcp_client::{McpRemoteClient, transport::TransportConfig};
use mcp_proxy::{stdio_proxy::StdioProxyBuilder, strategy::{ForwardingStrategy, LoadBalancingStrategy}};
use mcp_types::{TransportType, McpClient, LogLevel};
use serde_json::json;
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

/// Command-line interface for MCP Remote Proxy.
///
/// This structure defines the main CLI interface using clap, providing
/// global options and subcommands for different proxy operations.
#[derive(Parser)]
#[command(name = "mcp-remote")]
#[command(about = "MCP Remote Proxy - Bridge local MCP clients to remote MCP servers")]
#[command(version = "0.1.0")]
struct Cli {
    /// The subcommand to execute
    #[command(subcommand)]
    command: Commands,

    /// Enable debug logging for detailed troubleshooting
    #[arg(long, global = true, help = "Enable debug logging")]
    debug: bool,

    /// Set the log level (trace, debug, info, warn, error)
    #[arg(long, global = true, help = "Set log level")]
    log_level: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run as a proxy server (STDIO mode)
    Proxy {
        #[arg(long, help = "Primary remote server endpoint")]
        endpoint: String,

        #[arg(long, help = "Fallback transport types", value_delimiter = ',')]
        fallbacks: Option<Vec<String>>,

        #[arg(long, help = "Connection timeout in seconds", default_value = "30")]
        timeout: u64,

        #[arg(long, help = "Retry attempts", default_value = "3")]
        retry_attempts: u32,

        #[arg(long, help = "Retry delay in milliseconds", default_value = "1000")]
        retry_delay: u64,

        #[arg(long, help = "HTTP headers in key:value format", value_delimiter = ',')]
        headers: Option<Vec<String>>,

        #[arg(long, help = "Authorization token (Bearer token)")]
        auth_token: Option<String>,

        #[arg(long, help = "API key header value")]
        api_key: Option<String>,

        #[arg(long, help = "Custom User-Agent header")]
        user_agent: Option<String>,
    },

    /// Run with load balancing across multiple endpoints
    LoadBalance {
        #[arg(long, help = "Remote server endpoints", value_delimiter = ',')]
        endpoints: Vec<String>,

        #[arg(long, help = "Transport type for all endpoints", default_value = "http")]
        transport: String,

        #[arg(long, help = "Connection timeout in seconds", default_value = "30")]
        timeout: u64,

        #[arg(long, help = "Retry attempts", default_value = "3")]
        retry_attempts: u32,

        #[arg(long, help = "Retry delay in milliseconds", default_value = "1000")]
        retry_delay: u64,

        #[arg(long, help = "HTTP headers in key:value format", value_delimiter = ',')]
        headers: Option<Vec<String>>,

        #[arg(long, help = "Authorization token (Bearer token)")]
        auth_token: Option<String>,

        #[arg(long, help = "API key header value")]
        api_key: Option<String>,

        #[arg(long, help = "Custom User-Agent header")]
        user_agent: Option<String>,
    },

    /// Test connection to a remote MCP server
    Test {
        #[arg(long, help = "Remote server endpoint")]
        endpoint: String,

        #[arg(long, help = "Transport type", default_value = "http")]
        transport: String,

        #[arg(long, help = "Connection timeout in seconds", default_value = "10")]
        timeout: u64,

        #[arg(long, help = "HTTP headers in key:value format", value_delimiter = ',')]
        headers: Option<Vec<String>>,

        #[arg(long, help = "Authorization token (Bearer token)")]
        auth_token: Option<String>,

        #[arg(long, help = "API key header value")]
        api_key: Option<String>,

        #[arg(long, help = "Custom User-Agent header")]
        user_agent: Option<String>,
    },

    /// Demo MCP server notifications
    NotificationDemo {
        #[arg(long, help = "Number of test notifications to send", default_value = "3")]
        count: u32,
    },
}

fn parse_transport_type(transport: &str) -> Result<TransportType> {
    match transport.to_lowercase().as_str() {
        "http" => Ok(TransportType::Http),
        "stdio" => Ok(TransportType::Stdio),
        "tcp" => Ok(TransportType::Tcp),
        _ => Err(anyhow::anyhow!("Unknown transport type: {}", transport)),
    }
}

fn parse_fallback_transports(fallbacks: &[String]) -> Result<Vec<TransportType>> {
    fallbacks.iter()
        .map(|s| parse_transport_type(s))
        .collect()
}

fn parse_headers(headers: Option<Vec<String>>) -> Result<HashMap<String, String>> {
    let mut header_map = HashMap::new();

    if let Some(headers) = headers {
        for header in headers {
            if let Some((key, value)) = header.split_once(':') {
                header_map.insert(key.trim().to_string(), value.trim().to_string());
            } else {
                return Err(anyhow::anyhow!("Invalid header format '{}'. Expected 'key:value'", header));
            }
        }
    }

    Ok(header_map)
}

fn build_transport_config(
    endpoint: String,
    timeout: u64,
    retry_attempts: u32,
    retry_delay: u64,
    headers: Option<Vec<String>>,
    auth_token: Option<String>,
    api_key: Option<String>,
    user_agent: Option<String>,
) -> Result<TransportConfig> {
    let mut config = TransportConfig {
        endpoint,
        timeout: Duration::from_secs(timeout),
        retry_attempts,
        retry_delay: Duration::from_millis(retry_delay),
        headers: parse_headers(headers)?,
        auth_token: None,
        user_agent,
    };

    // Handle authentication
    if let Some(token) = auth_token {
        config = config.with_bearer_token(token);
    } else if let Some(key) = api_key {
        config = config.with_api_key("X-API-Key".to_string(), key);
    }

    Ok(config)
}

// Simple function to send MCP notifications to STDOUT
fn send_mcp_notification(level: LogLevel, message: &str) {
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/message",
        "params": {
            "level": level,
            "logger": "mcp-proxy",
            "data": message
        }
    });

    if let Ok(json_str) = serde_json::to_string(&notification) {
        println!("{}", json_str);
        let _ = io::stdout().flush();
    }
}

// Custom writer that either writes to stderr (debug mode) or discards (non-debug mode)
struct ConditionalWriter {
    debug_mode: bool,
}

impl ConditionalWriter {
    fn new(debug_mode: bool) -> Self {
        Self { debug_mode }
    }
}

impl Write for ConditionalWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.debug_mode {
            // In debug mode, write to stderr so it doesn't interfere with STDIO MCP protocol
            io::stderr().write(buf)
        } else {
            // In non-debug mode, discard the output
            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.debug_mode {
            io::stderr().flush()
        } else {
            Ok(())
        }
    }
}

impl tracing_subscriber::fmt::MakeWriter<'_> for ConditionalWriter {
    type Writer = Self;

    fn make_writer(&self) -> Self::Writer {
        ConditionalWriter::new(self.debug_mode)
    }
}

fn setup_logging(debug: bool, log_level: Option<String>) -> Result<()> {
    let level = if debug {
        Level::DEBUG
    } else if let Some(level_str) = log_level {
        match level_str.to_lowercase().as_str() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => return Err(anyhow::anyhow!("Invalid log level: {}", level_str)),
        }
    } else {
        Level::INFO
    };

    let writer = ConditionalWriter::new(debug);

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_writer(writer)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

async fn run_notification_demo(count: u32) -> Result<()> {
    info!("Starting MCP Notification Demo");

    // Send a few different types of notifications
    for i in 1..=count {
        match i % 4 {
            1 => {
                send_mcp_notification(LogLevel::Info, &format!("Demo info message {}", i));
                info!("Sent info notification {}", i);
            }
            2 => {
                send_mcp_notification(LogLevel::Warn, &format!("Demo warning message {}", i));
                warn!("Sent warning notification {}", i);
            }
            3 => {
                send_mcp_notification(LogLevel::Error, &format!("Demo error message {}", i));
                error!("Sent error notification {}", i);
            }
            0 => {
                send_mcp_notification(LogLevel::Debug, &format!("Demo debug message {}", i));
                info!("Sent debug notification {}", i);
            }
            _ => unreachable!(),
        }

        // Small delay between notifications
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    info!("Notification demo completed");
    Ok(())
}

async fn run_proxy(
    endpoint: String,
    fallbacks: Option<Vec<String>>,
    timeout: u64,
    retry_attempts: u32,
    retry_delay: u64,
    headers: Option<Vec<String>>,
    auth_token: Option<String>,
    api_key: Option<String>,
    user_agent: Option<String>,
    debug: bool,
) -> Result<()> {
    info!("Starting MCP Remote Proxy");
    info!("Primary endpoint: {}", endpoint);

    // Send MCP notification that proxy is starting
    send_mcp_notification(LogLevel::Info, &format!("MCP Proxy starting with endpoint: {}", endpoint));

    let fallback_transports = if let Some(fallbacks) = fallbacks {
        parse_fallback_transports(&fallbacks)?
    } else {
        vec![TransportType::Stdio, TransportType::Tcp]
    };

    info!("Fallback transports: {:?}", fallback_transports);

    // Build primary transport config with headers
    let primary_config = build_transport_config(
        endpoint.clone(),
        timeout,
        retry_attempts,
        retry_delay,
        headers,
        auth_token,
        api_key,
        user_agent,
    )?;

    let client = McpRemoteClient::new_with_config(primary_config, fallback_transports);
    let strategy = Arc::new(ForwardingStrategy::new(client));

    let proxy = StdioProxyBuilder::new()
        .with_strategy(strategy)
        .with_debug_mode(debug)
        .build()?;

    info!("Proxy ready, listening on STDIO");

    // Send MCP notification that proxy is ready
    send_mcp_notification(LogLevel::Info, "MCP Proxy ready and listening for requests");
    proxy.run().await?;

    Ok(())
}

async fn run_load_balance(
    endpoints: Vec<String>,
    transport: String,
    timeout: u64,
    retry_attempts: u32,
    retry_delay: u64,
    headers: Option<Vec<String>>,
    auth_token: Option<String>,
    api_key: Option<String>,
    user_agent: Option<String>,
    debug: bool,
) -> Result<()> {
    info!("Starting MCP Load Balancing Proxy");
    info!("Endpoints: {:?}", endpoints);
    info!("Transport: {}", transport);

    let transport_type = parse_transport_type(&transport)?;
    let mut clients = Vec::new();

    for endpoint in endpoints {
        let config = build_transport_config(
            endpoint.clone(),
            timeout,
            retry_attempts,
            retry_delay,
            headers.clone(),
            auth_token.clone(),
            api_key.clone(),
            user_agent.clone(),
        )?;

        let transports = vec![(transport_type.clone(), config)];
        let client = McpRemoteClient::with_custom_transports(transports).await;
        clients.push(client);
        info!("Added client for endpoint: {}", endpoint);
    }

    if clients.is_empty() {
        return Err(anyhow::anyhow!("No clients configured"));
    }

    let strategy = Arc::new(LoadBalancingStrategy::new(clients));

    let proxy = StdioProxyBuilder::new()
        .with_strategy(strategy)
        .with_debug_mode(debug)
        .build()?;

    info!("Load balancing proxy ready, listening on STDIO");
    proxy.run().await?;

    Ok(())
}

async fn test_connection(
    endpoint: String,
    transport: String,
    timeout: u64,
    headers: Option<Vec<String>>,
    auth_token: Option<String>,
    api_key: Option<String>,
    user_agent: Option<String>,
) -> Result<()> {
    info!("Testing connection to: {}", endpoint);
    info!("Transport: {}", transport);

    let transport_type = parse_transport_type(&transport)?;
    let config = build_transport_config(
        endpoint.clone(),
        timeout,
        1, // retry_attempts
        100, // retry_delay
        headers,
        auth_token,
        api_key,
        user_agent,
    )?;

    let transports = vec![(transport_type, config)];
    let client = McpRemoteClient::with_custom_transports(transports).await;

    // Test connection
    info!("Connecting...");
    let mut client = client;
    client.connect().await?;

    info!("Initializing...");
    let init_result = client.initialize().await?;
    info!("Server info: {} v{}", init_result.server_info.name, init_result.server_info.version);
    info!("Protocol version: {:?}", init_result.protocol_version);

    info!("Testing tools list...");
    match client.list_tools().await {
        Ok(tools) => info!("Tools: {}", tools),
        Err(e) => warn!("Failed to list tools: {}", e),
    }

    info!("Testing resources list...");
    match client.list_resources().await {
        Ok(resources) => info!("Resources: {}", resources),
        Err(e) => warn!("Failed to list resources: {}", e),
    }

    info!("Disconnecting...");
    client.disconnect().await?;

    info!("Connection test completed successfully!");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    setup_logging(cli.debug, cli.log_level)?;

    let result = match cli.command {
        Commands::Proxy {
            endpoint,
            fallbacks,
            timeout,
            retry_attempts,
            retry_delay,
            headers,
            auth_token,
            api_key,
            user_agent,
        } => {
            run_proxy(
                endpoint,
                fallbacks,
                timeout,
                retry_attempts,
                retry_delay,
                headers,
                auth_token,
                api_key,
                user_agent,
                cli.debug
            ).await
        }

        Commands::LoadBalance {
            endpoints,
            transport,
            timeout,
            retry_attempts,
            retry_delay,
            headers,
            auth_token,
            api_key,
            user_agent,
        } => {
            run_load_balance(
                endpoints,
                transport,
                timeout,
                retry_attempts,
                retry_delay,
                headers,
                auth_token,
                api_key,
                user_agent,
                cli.debug
            ).await
        }

        Commands::Test {
            endpoint,
            transport,
            timeout,
            headers,
            auth_token,
            api_key,
            user_agent,
        } => {
            test_connection(
                endpoint,
                transport,
                timeout,
                headers,
                auth_token,
                api_key,
                user_agent,
            ).await
        }

        Commands::NotificationDemo { count } => {
            run_notification_demo(count).await
        }
    };

    if let Err(e) = result {
        error!("Application error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
