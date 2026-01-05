//! MCP Connect CLI - Bridge local MCP clients to remote MCP servers.

mod commands;
mod transport;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::io::{self, Write};
use tracing::{error, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "mcp-connect")]
#[command(about = "MCP Connect - Bridge local MCP clients to remote MCP servers")]
#[command(version = "0.3.2")]
struct Cli {
    /// Remote server URL for quick connection (e.g., mcp-connect https://server.com/mcp)
    #[arg(value_name = "URL")]
    url: Option<String>,

    /// The subcommand to execute
    #[command(subcommand)]
    command: Option<Commands>,

    /// Enable debug logging for detailed troubleshooting
    #[arg(long, global = true, help = "Enable debug logging")]
    debug: bool,

    /// Set the log level (trace, debug, info, warn, error)
    #[arg(long, global = true, help = "Set log level")]
    log_level: Option<String>,

    /// HTTP headers in key:value format (for quick connection)
    #[arg(long, help = "HTTP headers", value_delimiter = ',')]
    headers: Option<Vec<String>>,

    /// Authorization token (for quick connection)
    #[arg(long, help = "Bearer token")]
    auth_token: Option<String>,
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

    /// Run as a proxy server (STDIO mode) with auto OAuth detection
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

        #[arg(long, help = "OAuth client ID (for servers requiring OAuth)")]
        oauth_client_id: Option<String>,

        #[arg(long, help = "OAuth client secret (optional, for confidential clients)")]
        oauth_client_secret: Option<String>,

        #[arg(long, help = "OAuth redirect port (default: 8085)", default_value = "8085")]
        oauth_redirect_port: u16,
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

    /// Pre-authenticate with an OAuth-protected endpoint
    Auth {
        #[arg(long, help = "Remote server endpoint")]
        endpoint: String,

        #[arg(long, help = "OAuth client ID (optional if server supports dynamic registration)")]
        oauth_client_id: Option<String>,

        #[arg(long, help = "OAuth client secret (optional)")]
        oauth_client_secret: Option<String>,

        #[arg(long, help = "OAuth redirect port (default: 8085)", default_value = "8085")]
        oauth_redirect_port: u16,

        #[arg(long, help = "Clear cached token instead of authenticating")]
        clear: bool,
    },
}


struct ConditionalWriter { debug_mode: bool }

impl ConditionalWriter {
    fn new(debug_mode: bool) -> Self { Self { debug_mode } }
}

impl Write for ConditionalWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.debug_mode { io::stderr().write(buf) } else { Ok(buf.len()) }
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.debug_mode { io::stderr().flush() } else { Ok(()) }
    }
}

impl tracing_subscriber::fmt::MakeWriter<'_> for ConditionalWriter {
    type Writer = Self;
    fn make_writer(&self) -> Self::Writer { ConditionalWriter::new(self.debug_mode) }
}

fn setup_logging(debug: bool, log_level: Option<String>) -> Result<()> {
    let level = match (debug, log_level.as_deref()) {
        (true, _) => Level::DEBUG,
        (_, Some("trace")) => Level::TRACE,
        (_, Some("debug")) => Level::DEBUG,
        (_, Some("info")) => Level::INFO,
        (_, Some("warn")) => Level::WARN,
        (_, Some("error")) => Level::ERROR,
        (_, Some(l)) => return Err(anyhow::anyhow!("Invalid log level: {}", l)),
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_writer(ConditionalWriter::new(debug))
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    setup_logging(cli.debug, cli.log_level.clone())?;

    // Quick connection mode: mcp-connect <url>
    if let Some(url) = cli.url {
        if url.starts_with("http://") || url.starts_with("https://") {
            return commands::proxy::run_proxy(
                url,
                None,  // fallbacks
                30,    // timeout
                3,     // retry_attempts
                1000,  // retry_delay
                cli.headers,
                cli.auth_token,
                None,  // api_key
                None,  // user_agent
                None,  // oauth_client_id
                None,  // oauth_client_secret
                8085,  // oauth_redirect_port
                cli.debug,
            ).await;
        }
    }

    let Some(command) = cli.command else {
        eprintln!("Usage: mcp-connect <URL> or mcp-connect <COMMAND>");
        eprintln!("Run 'mcp-connect --help' for more information.");
        std::process::exit(1);
    };

    let result = match command {
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
            endpoint, fallbacks, timeout, retry_attempts, retry_delay,
            headers, auth_token, api_key, user_agent,
            oauth_client_id, oauth_client_secret, oauth_redirect_port,
        } => {
            commands::proxy::run_proxy(
                endpoint, fallbacks, timeout, retry_attempts, retry_delay,
                headers, auth_token, api_key, user_agent,
                oauth_client_id, oauth_client_secret, oauth_redirect_port, cli.debug,
            ).await
        }

        Commands::LoadBalance {
            endpoints, transport, timeout, retry_attempts, retry_delay,
            headers, auth_token, api_key, user_agent,
        } => {
            commands::proxy::run_load_balance(
                endpoints, transport, timeout, retry_attempts, retry_delay,
                headers, auth_token, api_key, user_agent, cli.debug,
            ).await
        }

        Commands::Test {
            endpoint, transport, timeout, headers, auth_token, api_key, user_agent,
        } => {
            commands::proxy::test_connection(
                endpoint, transport, timeout, headers, auth_token, api_key, user_agent,
            ).await
        }

        Commands::NotificationDemo { count } => {
            commands::proxy::run_notification_demo(count).await
        }

        Commands::Auth {
            endpoint, oauth_client_id, oauth_client_secret, oauth_redirect_port, clear,
        } => {
            commands::proxy::run_auth(
                endpoint, oauth_client_id, oauth_client_secret, oauth_redirect_port, clear,
            ).await
        }
    };

    if let Err(e) = result {
        error!("Application error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
