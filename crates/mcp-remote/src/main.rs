use clap::{Parser, ValueEnum};
use mcp_proxy::{McpProxy, TransportStrategy};
use tracing::{error, info};

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
    #[arg(long, value_enum, default_value = "http-first")]
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

    // Initialize logging
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
        .init();

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

    // Create and start the proxy
    let mut proxy = McpProxy::new(server_url)
        .with_transport_strategy(args.transport.into())
        .with_headers(args.headers);

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
