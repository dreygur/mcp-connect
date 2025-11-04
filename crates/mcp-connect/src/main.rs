//! # MCP Connect CLI
//!
//! Command-line interface for the Model Context Protocol (MCP) remote proxy system.
//!
//! This application bridges local MCP clients with remote MCP servers, providing:
//! - Multiple transport support (HTTP, STDIO, TCP)
//! - Authentication handling (Bearer tokens, API keys, OAuth 2.1)
//! - Fallback mechanisms and load balancing
//! - Comprehensive logging and debugging
//! - Central configuration management
//! - MCP Registry integration
//!
//! ## Usage
//!
//! Initialize new project:
//! ```bash
//! mcp-connect init
//! ```
//!
//! Add servers from registry:
//! ```bash
//! mcp-connect config add github modelcontextprotocol/github-mcp-server
//! ```
//!
//! Start multiplexing server:
//! ```bash
//! mcp-connect serve
//! ```
//!
//! ## Commands
//!
//! - `init`: Initialize new configuration
//! - `registry`: Search and browse MCP Registry (supports custom registry sources)
//! - `config`: Manage server configurations
//! - `serve`: Run multiplexing server
//! - `generate`: Generate IDE configurations
//! - `proxy`: Run as STDIO proxy (legacy)
//! - `test`: Test connection to remote server
//! - `load-balance`: Distribute requests across multiple servers

mod commands;
mod transport;

use anyhow::Result;
use clap::{Parser, Subcommand};
use mcp_client::McpRemoteClient;
use mcp_proxy::{stdio_proxy::StdioProxyBuilder, strategy::{ForwardingStrategy, LoadBalancingStrategy}};
use mcp_types::{TransportType, McpClient, LogLevel};
use serde_json::json;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

/// Command-line interface for MCP Connect.
///
/// This structure defines the main CLI interface using clap, providing
/// global options and subcommands for different proxy operations.
#[derive(Parser)]
#[command(name = "mcp-connect")]
#[command(about = "MCP Connect - Bridge local MCP clients to remote MCP servers")]
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
enum RegistryCommands {
    /// Search for servers in the registry (only shows remote-compatible servers)
    Search {
        /// Search query
        query: String,

        #[arg(long, help = "Show all servers including STDIO-only (not compatible with mcp-connect)")]
        show_all: bool,

        #[arg(long, help = "Registry source to use (from configured sources, or 'default' for official)")]
        source: Option<String>,
    },

    /// Show detailed information about a specific server
    Show {
        /// Registry path (e.g., "modelcontextprotocol/github-mcp-server")
        registry_path: String,

        #[arg(long, help = "Registry source to use (from configured sources, or 'default' for official)")]
        source: Option<String>,
    },

    /// List all servers in the registry (only shows remote-compatible servers)
    List {
        #[arg(long, help = "Show all servers including STDIO-only (not compatible with mcp-connect)")]
        show_all: bool,

        #[arg(long, help = "Registry source to use (from configured sources, or 'default' for official)")]
        source: Option<String>,
    },

    /// Add a custom registry source
    AddSource {
        /// Name for the registry source
        name: String,

        /// Base URL of the registry (e.g., "https://registry.example.com")
        #[arg(long)]
        url: String,

        /// API version (e.g., "v1", "v0.1"). If not specified, uses default.
        #[arg(long)]
        api_version: Option<String>,
    },

    /// List all configured registry sources
    ListSources,

    /// Remove a custom registry source
    RemoveSource {
        /// Registry source name to remove
        name: String,

        #[arg(long, help = "Skip confirmation prompt")]
        force: bool,
    },

    /// Set the default registry source to use
    SetDefaultSource {
        /// Registry source name (use "default" for official MCP registry)
        name: String,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Add a new server to configuration
    Add {
        /// Local name for the server
        name: String,

        /// Registry path (optional, e.g., "modelcontextprotocol/github-mcp-server")
        registry_path: Option<String>,

        #[arg(long, help = "Add from GitHub registry URL")]
        from_url: Option<String>,

        #[arg(long, help = "Interactive search mode")]
        search: bool,

        #[arg(long, help = "Custom server URL (for non-registry servers)")]
        url: Option<String>,

        #[arg(long, help = "Authorization header value")]
        auth_header: Option<String>,
    },

    /// List all configured servers
    List,

    /// Show details of a specific server
    Show {
        /// Server name
        name: String,
    },

    /// Remove a server from configuration
    Remove {
        /// Server name
        name: String,

        #[arg(long, help = "Skip confirmation prompt")]
        force: bool,
    },

    /// Test connectivity to server(s)
    Test {
        /// Server name (optional, omit to use --all)
        name: Option<String>,

        #[arg(long, help = "Test all configured servers")]
        all: bool,
    },

    /// Validate configuration file
    Validate,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize new mcp-connect configuration
    Init {
        #[arg(long, help = "Overwrite existing configuration")]
        force: bool,
    },

    /// Search and browse the MCP Registry
    Registry {
        #[command(subcommand)]
        command: RegistryCommands,
    },

    /// Manage server configurations
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Start multiplexing server (serves all configured servers)
    Serve,

    /// Generate IDE-specific configuration
    Generate {
        #[arg(long, help = "Target IDE (zed, vscode, cursor)")]
        ide: String,

        #[arg(long, help = "Output path for generated config")]
        output: Option<String>,
    },

    /// Run as a proxy server (STDIO mode) - LEGACY
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
    info!("Starting MCP Connect");
    info!("Primary endpoint: {}", endpoint);

    // Send MCP notification that proxy is starting
    send_mcp_notification(LogLevel::Info, &format!("MCP Proxy starting with endpoint: {}", endpoint));

    let fallback_transports = if let Some(fallbacks) = fallbacks {
        transport::parse_fallback_transports(&fallbacks)?
    } else {
        vec![TransportType::Stdio, TransportType::Tcp]
    };

    info!("Fallback transports: {:?}", fallback_transports);

    // Build primary transport config with headers
    let primary_config = transport::build_transport_config(
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

    let transport_type = transport::parse_transport_type(&transport)?;
    let mut clients = Vec::new();

    for endpoint in endpoints {
        let config = transport::build_transport_config(
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

    let transport_type = transport::parse_transport_type(&transport)?;
    let config = transport::build_transport_config(
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
        Commands::Init { force } => {
            commands::init::init(force)
        }

        Commands::Registry { command } => {
            match command {
                RegistryCommands::Search { query, show_all, source } => {
                    commands::registry::search(&query, !show_all, source.as_deref()).await
                }
                RegistryCommands::Show { registry_path, source } => {
                    commands::registry::show(&registry_path, source.as_deref()).await
                }
                RegistryCommands::List { show_all, source } => {
                    commands::registry::list(!show_all, source.as_deref()).await
                }
                RegistryCommands::AddSource { name, url, api_version } => {
                    commands::registry::add_source(&name, &url, api_version.as_deref()).await
                }
                RegistryCommands::ListSources => {
                    commands::registry::list_sources()
                }
                RegistryCommands::RemoveSource { name, force } => {
                    commands::registry::remove_source(&name, force).await
                }
                RegistryCommands::SetDefaultSource { name } => {
                    commands::registry::set_default_source(&name).await
                }
            }
        }

        Commands::Config { command } => {
            match command {
                ConfigCommands::Add {
                    name,
                    registry_path,
                    from_url,
                    search,
                    url,
                    auth_header,
                } => {
                    commands::config::add(name, registry_path, from_url, search, url, auth_header).await
                }
                ConfigCommands::List => {
                    commands::config::list()
                }
                ConfigCommands::Show { name } => {
                    commands::config::show(&name)
                }
                ConfigCommands::Remove { name, force } => {
                    commands::config::remove(&name, force)
                }
                ConfigCommands::Test { name, all } => {
                    commands::config::test(name, all).await
                }
                ConfigCommands::Validate => {
                    commands::config::validate()
                }
            }
        }

        Commands::Serve => {
            commands::serve::serve(cli.debug).await
        }

        Commands::Generate { ide, output } => {
            use std::str::FromStr;
            let ide_type = commands::generate::IdeType::from_str(&ide)?;
            commands::generate::generate(ide_type, output)
        }

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
