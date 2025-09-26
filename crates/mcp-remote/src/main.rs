use clap::{Parser, ValueEnum};
use mcp_proxy::{McpProxy, TransportStrategy};
use mcp_oauth::OAuthClient;
use tracing::{error, info};
use std::path::PathBuf;
use std::fs;

#[derive(Parser)]
#[command(
    name = "mcp-remote",
    about = "MCP Remote Proxy - Bridge local MCP clients to remote servers",
    version = env!("CARGO_PKG_VERSION")
)]
struct Args {
    /// Remote MCP server URL
    #[arg(value_name = "URL")]
    server_url: String,

    /// Transport strategy for connecting to remote server
    #[arg(long, value_enum, default_value = "sse-first")]
    transport: TransportStrategyArg,

    /// Enable debug logging
    #[arg(long, short)]
    debug: bool,

    /// Custom HTTP headers (format: key:value)
    #[arg(long = "header", value_name = "KEY:VALUE")]
    headers: Vec<String>,

    /// Bind host for local server (not applicable for STDIO)
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Allow HTTP connections (default is HTTPS only)
    #[arg(long)]
    allow_http: bool,

    /// Enable proxy support via environment variables
    #[arg(long)]
    enable_proxy: bool,

    /// Ignore specific tools (supports wildcards)
    #[arg(long = "ignore-tool")]
    ignore_tools: Vec<String>,

    /// Authentication timeout in seconds
    #[arg(long, default_value = "300")]
    auth_timeout: u64,

    /// Enable OAuth 2.0 authentication
    #[arg(long)]
    oauth: bool,

    /// OAuth callback port (0 for auto-select)
    #[arg(long, default_value = "0")]
    oauth_port: u16,

    /// Static OAuth client ID
    #[arg(long)]
    oauth_client_id: Option<String>,

    /// Static OAuth client secret
    #[arg(long)]
    oauth_client_secret: Option<String>,

    /// OAuth scope to request
    #[arg(long, default_value = "mcp")]
    oauth_scope: String,

    /// Static OAuth client metadata (JSON string or @filepath)
    #[arg(long)]
    static_oauth_client_metadata: Option<String>,

    /// Static OAuth client info (JSON string or @filepath)
    #[arg(long)]
    static_oauth_client_info: Option<String>,

    /// Host for OAuth callback URL
    #[arg(long, default_value = "localhost")]
    callback_host: String,
}

#[derive(ValueEnum, Clone, Debug)]
enum TransportStrategyArg {
    #[value(name = "http-first")]
    HttpFirst,
    #[value(name = "sse-first")]
    SseFirst,
    #[value(name = "http-only")]
    HttpOnly,
    #[value(name = "sse-only")]
    SseOnly,
}

impl From<TransportStrategyArg> for TransportStrategy {
    fn from(arg: TransportStrategyArg) -> Self {
        match arg {
            TransportStrategyArg::HttpFirst => TransportStrategy::HttpFirst,
            TransportStrategyArg::SseFirst => TransportStrategy::SseFirst,
            TransportStrategyArg::HttpOnly => TransportStrategy::HttpOnly,
            TransportStrategyArg::SseOnly => TransportStrategy::SseOnly,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging - disable if running under inspector
    let is_under_inspector = std::env::var("MCP_INSPECTOR").is_ok() ||
                             std::env::var("MCP_PROXY_AUTH_TOKEN").is_ok();

    if !is_under_inspector {
        let log_level = if args.debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        };

        tracing_subscriber::fmt()
            .with_max_level(log_level)
            .with_target(false)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false)
            .with_writer(std::io::stderr) // Send logs to stderr, not stdout
            .init();
    } else {
        // Under inspector - disable logging to avoid contaminating MCP protocol
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::ERROR) // Only show critical errors
            .with_writer(std::io::stderr)
            .init();
    }

    // Validate server URL
    let server_url = validate_server_url(&args.server_url, args.allow_http)?;

    info!("MCP Remote Proxy starting");
    info!("Server URL: {}", server_url);
    info!("Transport strategy: {:?}", args.transport);

    if args.debug {
        info!("Debug logging enabled");
    }

    if !args.headers.is_empty() {
        info!("Custom headers: {:?}", args.headers);
    }

    if args.enable_proxy {
        info!("Proxy support enabled");
    }

    if !args.ignore_tools.is_empty() {
        info!("Ignoring tools: {:?}", args.ignore_tools);
    }

    // Handle OAuth authentication if enabled
    let mut headers = args.headers;
    if args.oauth {
        info!("OAuth 2.0 authentication enabled");

        // Get authentication directory
        let auth_dir = get_auth_directory()?;

        // Create OAuth client
        let mut oauth_client = OAuthClient::new(server_url.clone(), auth_dir)?
            .with_callback_port(args.oauth_port)
            .with_callback_host(args.callback_host.clone())
            .with_auth_timeout(args.auth_timeout)
            .with_scope(args.oauth_scope);

        // Add static client info if provided
        if let (Some(client_id), client_secret) = (args.oauth_client_id, args.oauth_client_secret) {
            oauth_client = oauth_client.with_static_client_info(client_id, client_secret);
        }

        // Add static OAuth server metadata if provided
        if let Some(metadata_input) = args.static_oauth_client_metadata {
            match parse_json_or_file(&metadata_input) {
                Ok(metadata_json) => {
                    info!("Using static OAuth client metadata");
                    let metadata: mcp_oauth::types::OAuthServerMetadata = serde_json::from_value(metadata_json)
                        .map_err(|e| anyhow::anyhow!("Invalid OAuth server metadata format: {}", e))?;
                    oauth_client = oauth_client.with_server_metadata(metadata);
                }
                Err(e) => {
                    error!("Failed to parse static OAuth client metadata: {}", e);
                    return Err(e);
                }
            }
        }

        // Add static OAuth client info if provided (in addition to individual flags)
        if let Some(client_info_input) = args.static_oauth_client_info {
            match parse_json_or_file(&client_info_input) {
                Ok(client_json) => {
                    info!("Using static OAuth client info");
                    let client_id = client_json.get("client_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| anyhow::anyhow!("Missing client_id in static OAuth client info"))?
                        .to_string();
                    let client_secret = client_json.get("client_secret")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    oauth_client = oauth_client.with_static_client_info(client_id, client_secret);
                }
                Err(e) => {
                    error!("Failed to parse static OAuth client info: {}", e);
                    return Err(e);
                }
            }
        }

        // Get access token and add to headers
        match oauth_client.get_access_token().await {
            Ok(access_token) => {
                info!("Successfully obtained OAuth access token");
                headers.push(format!("Authorization: Bearer {}", access_token));
            }
            Err(e) => {
                error!("OAuth authentication failed: {}", e);
                return Err(anyhow::anyhow!("OAuth authentication failed: {}", e));
            }
        }
    }

    // Create and start the proxy
    let mut proxy = McpProxy::new(server_url)
        .with_transport_strategy(args.transport.into())
        .with_headers(headers);

    // Handle shutdown gracefully
    let result = tokio::select! {
        result = proxy.start() => {
            match result {
                Ok(()) => {
                    info!("Proxy completed successfully");
                    Ok(())
                }
                Err(e) => {
                    error!("Proxy error: {}", e);
                    Err(anyhow::anyhow!("Proxy failed: {}", e))
                }
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received interrupt signal, shutting down gracefully");
            proxy.stop().await.map_err(|e| anyhow::anyhow!("Shutdown error: {}", e))?;
            Ok(())
        }
    };

    info!("MCP Remote Proxy stopped");
    result
}

/// Parse JSON string or file path (if prefixed with @)
fn parse_json_or_file(input: &str) -> anyhow::Result<serde_json::Value> {
    if input.starts_with('@') {
        // File path - remove @ prefix and read file
        let file_path = &input[1..];
        let content = fs::read_to_string(file_path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", file_path, e))?;
        serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Invalid JSON in file '{}': {}", file_path, e))
    } else {
        // Direct JSON string
        serde_json::from_str(input)
            .map_err(|e| anyhow::anyhow!("Invalid JSON string: {}", e))
    }
}

fn validate_server_url(url: &str, allow_http: bool) -> anyhow::Result<String> {
    use url::Url;

    let parsed = Url::parse(url)
        .map_err(|e| anyhow::anyhow!("Invalid URL '{}': {}", url, e))?;

    match parsed.scheme() {
        "https" => Ok(url.to_string()),
        "http" => {
            if allow_http {
                Ok(url.to_string())
            } else {
                anyhow::bail!(
                    "HTTP URLs are not allowed by default. Use --allow-http flag for trusted networks."
                );
            }
        }
        scheme => {
            anyhow::bail!("Unsupported URL scheme '{}'. Use http:// or https://", scheme);
        }
    }
}

/// Get the authentication directory for storing OAuth tokens
///
/// Returns ~/.mcp-auth on Unix-like systems or equivalent on other platforms
fn get_auth_directory() -> anyhow::Result<PathBuf> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    let auth_dir = home_dir.join(".mcp-auth");

    // Create directory if it doesn't exist
    if !auth_dir.exists() {
        std::fs::create_dir_all(&auth_dir)?;
        info!("Created authentication directory: {:?}", auth_dir);
    }

    Ok(auth_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_server_url_https() {
        let result = validate_server_url("https://example.com/mcp", false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com/mcp");
    }

    #[test]
    fn test_validate_server_url_http_disallowed() {
        let result = validate_server_url("http://example.com/mcp", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_server_url_http_allowed() {
        let result = validate_server_url("http://example.com/mcp", true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "http://example.com/mcp");
    }

    #[test]
    fn test_validate_server_url_invalid() {
        let result = validate_server_url("invalid-url", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_server_url_unsupported_scheme() {
        let result = validate_server_url("ftp://example.com", false);
        assert!(result.is_err());
    }
}
