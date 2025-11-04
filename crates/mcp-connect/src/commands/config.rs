//! Config command implementations.
//!
//! Commands for managing the .mcp-connect.json configuration file.

use anyhow::{Context, Result};
use inquire::{Confirm, Text};
use mcp_config::ConfigManager;
use mcp_registry::RegistryClient;
use mcp_types::{KeyValue, RemoteConfig, RemoteTransportType, ServerConfig};
use tracing::{info, warn};

/// Add a new server to the configuration.
pub async fn add(
    name: String,
    registry_path: Option<String>,
    from_url: Option<String>,
    search: bool,
    url: Option<String>,
    auth_header: Option<String>,
) -> Result<()> {
    let mut manager = ConfigManager::new()?;

    // Load existing config or create new one
    let mut config = if manager.exists() {
        manager.load()?
    } else {
        println!("⚠️  No configuration file found. Creating new one...");
        manager.init()?
    };

    // Check if server already exists
    if config.servers.contains_key(&name) {
        anyhow::bail!("Server '{}' already exists in configuration", name);
    }

    let server_config = if search {
        // Interactive search mode
        add_interactive(&name).await?
    } else if let Some(registry_path) = registry_path {
        // Add from registry - check if it's a URL first
        let path = if registry_path.contains("://") || registry_path.starts_with("github.com") || registry_path.starts_with("www.github.com") {
            // It's a URL, extract the registry path
            extract_registry_path(&registry_path)?
        } else {
            // It's already a registry path
            registry_path
        };
        add_from_registry(&path).await?
    } else if let Some(url_str) = from_url {
        // Extract registry path from GitHub URL
        let path = extract_registry_path(&url_str)?;
        add_from_registry(&path).await?
    } else if let Some(url_str) = url {
        // Add custom server
        add_custom(&name, &url_str, auth_header.as_deref())?
    } else {
        // Try to interpret name as registry path - check if it's a URL first
        let path = if name.contains("://") || name.starts_with("github.com") || name.starts_with("www.github.com") {
            // It's a URL, extract the registry path
            extract_registry_path(&name)?
        } else {
            // It's already a registry path
            name.clone()
        };
        add_from_registry(&path).await?
    };

    // Add to config
    config.servers.insert(name.clone(), server_config);

    // Save config
    manager.save(&config)?;

    println!("✓ Added '{}' to .mcp-connect.json", name);
    Ok(())
}

/// Add server interactively with search.
async fn add_interactive(name: &str) -> Result<ServerConfig> {
    let query = Text::new("Search registry:")
        .with_default(name)
        .prompt()?;

    let client = RegistryClient::new()?;
    let servers = client.search(&query).await?;

    if servers.is_empty() {
        anyhow::bail!("No servers found matching '{}'", query);
    }

    // Show options
    println!("\nFound {} server(s):", servers.len());
    for (i, server) in servers.iter().enumerate() {
        println!("{}. {} - {}", i + 1, server.name, &server.description);
    }

    let selection = Text::new("Select server (enter number):")
        .prompt()?;

    let index: usize = selection.parse::<usize>()
        .context("Invalid selection")?
        .checked_sub(1)
        .context("Invalid selection")?;

    let selected = servers.get(index)
        .context("Invalid selection")?;

    add_from_registry(&selected.name).await
}

/// Add server from registry.
async fn add_from_registry(registry_path: &str) -> Result<ServerConfig> {
    info!("Adding server from registry: {}", registry_path);

    let client = RegistryClient::new()?;
    let server_response = client.get_server(registry_path, "latest").await?;
    let server = &server_response.server;

    // Check if server has remote transport
    if !RegistryClient::has_remote_transport(server) {
        anyhow::bail!(
            "❌ Error: {} only supports STDIO transport (not HTTP).\n\n\
            💡 This server must be configured directly in your IDE.\n\
            This tool only supports remote HTTP servers.",
            server.name
        );
    }

    // Convert to RemoteConfig
    let remote = RegistryClient::to_remote_config(server)?
        .context("Server has no remote configuration")?;

    // Prompt for URL if needed
    let url = if remote.url.contains("${") || remote.url.is_empty() {
        Text::new("Remote URL:")
            .with_help_message("Enter the actual endpoint URL")
            .prompt()?
    } else {
        let confirmed_url = Text::new("Remote URL:")
            .with_default(&remote.url)
            .prompt()?;
        confirmed_url
    };

    // Prompt for authentication
    let add_auth = Confirm::new("Add authentication header?")
        .with_default(true)
        .prompt()?;

    let headers = if add_auth {
        let header_value = Text::new("Header value:")
            .with_help_message("Use ${VAR} for environment variables, e.g., Bearer ${GITHUB_TOKEN}")
            .with_default("Bearer ${TOKEN}")
            .prompt()?;

        Some(vec![KeyValue {
            key: "Authorization".to_string(),
            value: header_value,
        }])
    } else {
        None
    };

    Ok(ServerConfig {
        name: Some(server.name.clone()),
        description: Some(server.description.clone()),
        version: Some(server.version.clone()),
        remote: RemoteConfig {
            transport_type: remote.transport_type,
            url,
            headers,
        },
        timeout: Some(30),
        retry_attempts: Some(3),
        fallbacks: None,
    })
}

/// Add custom server with URL.
fn add_custom(name: &str, url: &str, auth_header: Option<&str>) -> Result<ServerConfig> {
    info!("Adding custom server: {}", name);

    let headers = auth_header.map(|h| {
        vec![KeyValue {
            key: "Authorization".to_string(),
            value: h.to_string(),
        }]
    });

    Ok(ServerConfig {
        name: None,
        description: Some(format!("Custom server: {}", name)),
        version: None,
        remote: RemoteConfig {
            transport_type: RemoteTransportType::StreamableHttp,
            url: url.to_string(),
            headers,
        },
        timeout: Some(30),
        retry_attempts: Some(3),
        fallbacks: None,
    })
}

/// Extract registry path from GitHub URL.
fn extract_registry_path(url: &str) -> Result<String> {
    // Normalize URL: trim whitespace
    let url = url.trim();

    // Remove query parameters and fragments first
    let url = url.split('?').next().unwrap_or(url);
    let url = url.split('#').next().unwrap_or(url);

    // Then remove trailing slashes
    let url = url.trim_end_matches('/');

    // Try different GitHub URL patterns
    // Pattern 1: https://github.com/mcp/{publisher}/{server-name}
    if let Some(path) = url.strip_prefix("https://github.com/mcp/") {
        return Ok(path.to_string());
    }

    // Pattern 2: http://github.com/mcp/{publisher}/{server-name}
    if let Some(path) = url.strip_prefix("http://github.com/mcp/") {
        return Ok(path.to_string());
    }

    // Pattern 3: github.com/mcp/{publisher}/{server-name} (no scheme)
    if let Some(path) = url.strip_prefix("github.com/mcp/") {
        return Ok(path.to_string());
    }

    // Pattern 4: www.github.com/mcp/{publisher}/{server-name}
    if let Some(path) = url.strip_prefix("https://www.github.com/mcp/") {
        return Ok(path.to_string());
    }

    if let Some(path) = url.strip_prefix("http://www.github.com/mcp/") {
        return Ok(path.to_string());
    }

    if let Some(path) = url.strip_prefix("www.github.com/mcp/") {
        return Ok(path.to_string());
    }

    anyhow::bail!("Invalid GitHub MCP registry URL: {}\nExpected format: https://github.com/mcp/{{publisher}}/{{server-name}}", url);
}

/// List all configured servers.
pub fn list() -> Result<()> {
    let mut manager = ConfigManager::new()?;

    if !manager.exists() {
        println!("⚠️  No configuration file found.");
        println!("💡 Run 'mcp-connect init' to create one.");
        return Ok(());
    }

    let config = manager.load()?;

    if config.servers.is_empty() {
        println!("📝 No servers configured yet.");
        println!("💡 Run 'mcp-connect config add <name> <registry-path>' to add a server.");
        return Ok(());
    }

    println!("Configured servers in .mcp-connect.json:\n");

    for (name, server) in &config.servers {
        println!("📍 {} {}", name, server.name.as_deref().unwrap_or(""));
        if let Some(desc) = &server.description {
            println!("   {}", desc);
        }
        println!("   URL: {}", server.remote.url);

        if let Some(headers) = &server.remote.headers {
            if !headers.is_empty() {
                println!("   Auth: {} {}", headers[0].key, headers[0].value);
            }
        }

        println!();
    }

    println!("Total: {} server(s)", config.servers.len());
    Ok(())
}

/// Show details of a specific server.
pub fn show(name: &str) -> Result<()> {
    let mut manager = ConfigManager::new()?;
    let config = manager.load()?;

    let server = config
        .servers
        .get(name)
        .with_context(|| format!("Server '{}' not found in configuration", name))?;

    println!("📦 Server: {}\n", name);

    if let Some(registry_name) = &server.name {
        println!("Registry: {}", registry_name);
    }

    if let Some(desc) = &server.description {
        println!("Description: {}", desc);
    }

    if let Some(version) = &server.version {
        println!("Version: {}", version);
    }

    println!("\n🌐 Remote Configuration:");
    println!("  Type: {:?}", server.remote.transport_type);
    println!("  URL: {}", server.remote.url);

    if let Some(headers) = &server.remote.headers {
        println!("\n  Headers:");
        for header in headers {
            println!("    {}: {}", header.key, header.value);
        }
    }

    if let Some(timeout) = server.timeout {
        println!("\n⏱️  Timeout: {}s", timeout);
    }

    if let Some(retry) = server.retry_attempts {
        println!("🔄 Retry attempts: {}", retry);
    }

    Ok(())
}

/// Remove a server from configuration.
pub fn remove(name: &str, force: bool) -> Result<()> {
    let mut manager = ConfigManager::new()?;
    let mut config = manager.load()?;

    if !config.servers.contains_key(name) {
        anyhow::bail!("Server '{}' not found in configuration", name);
    }

    if !force {
        let confirm = Confirm::new(&format!("Remove server '{}'?", name))
            .with_default(false)
            .prompt()?;

        if !confirm {
            println!("Cancelled.");
            return Ok(());
        }
    }

    config.servers.remove(name);
    manager.save(&config)?;

    println!("✓ Removed '{}' from configuration", name);
    Ok(())
}

/// Test connectivity to a server.
pub async fn test(name: Option<String>, test_all: bool) -> Result<()> {
    let mut manager = ConfigManager::new()?;
    let config = manager.load()?;

    if test_all {
        println!("🧪 Testing all configured servers...\n");

        for (server_name, _) in &config.servers {
            test_single_server(&config, server_name).await?;
            println!();
        }
    } else if let Some(name) = name {
        test_single_server(&config, &name).await?;
    } else {
        anyhow::bail!("Either specify a server name or use --all flag");
    }

    Ok(())
}

/// Test a single server connection.
async fn test_single_server(config: &mcp_types::ConnectConfig, name: &str) -> Result<()> {
    let server = config
        .servers
        .get(name)
        .with_context(|| format!("Server '{}' not found", name))?;

    println!("🧪 Testing: {}", name);
    println!("   URL: {}", server.remote.url);

    // Basic URL validation
    if !server.remote.url.starts_with("http://") && !server.remote.url.starts_with("https://") {
        warn!("   ⚠️  Invalid URL scheme");
        return Ok(());
    }

    // Try to make a HEAD request to check if endpoint is reachable
    let client = reqwest::Client::new();
    match client.head(&server.remote.url).send().await {
        Ok(response) => {
            println!("   ✓ Endpoint reachable (HTTP {})", response.status());
        }
        Err(e) => {
            println!("   ✗ Connection failed: {}", e);
        }
    }

    Ok(())
}

/// Validate configuration file.
pub fn validate() -> Result<()> {
    let mut manager = ConfigManager::new()?;

    if !manager.exists() {
        anyhow::bail!("No configuration file found");
    }

    let config = manager.load()?;
    ConfigManager::validate(&config)?;

    println!("✓ Configuration is valid");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_registry_path_basic() {
        let url = "https://github.com/mcp/upstash/context7";
        let result = extract_registry_path(url).unwrap();
        assert_eq!(result, "upstash/context7");
    }

    #[test]
    fn test_extract_registry_path_with_trailing_slash() {
        let url = "https://github.com/mcp/upstash/context7/";
        let result = extract_registry_path(url).unwrap();
        assert_eq!(result, "upstash/context7");
    }

    #[test]
    fn test_extract_registry_path_with_query_params() {
        let url = "https://github.com/mcp/upstash/context7?tab=readme";
        let result = extract_registry_path(url).unwrap();
        assert_eq!(result, "upstash/context7");
    }

    #[test]
    fn test_extract_registry_path_with_fragment() {
        let url = "https://github.com/mcp/upstash/context7#readme";
        let result = extract_registry_path(url).unwrap();
        assert_eq!(result, "upstash/context7");
    }

    #[test]
    fn test_extract_registry_path_http() {
        let url = "http://github.com/mcp/upstash/context7";
        let result = extract_registry_path(url).unwrap();
        assert_eq!(result, "upstash/context7");
    }

    #[test]
    fn test_extract_registry_path_no_scheme() {
        let url = "github.com/mcp/upstash/context7";
        let result = extract_registry_path(url).unwrap();
        assert_eq!(result, "upstash/context7");
    }

    #[test]
    fn test_extract_registry_path_with_www() {
        let url = "https://www.github.com/mcp/upstash/context7";
        let result = extract_registry_path(url).unwrap();
        assert_eq!(result, "upstash/context7");
    }

    #[test]
    fn test_extract_registry_path_with_whitespace() {
        let url = "  https://github.com/mcp/upstash/context7  ";
        let result = extract_registry_path(url).unwrap();
        assert_eq!(result, "upstash/context7");
    }

    #[test]
    fn test_extract_registry_path_complex() {
        let url = "https://github.com/mcp/io.github.modelcontextprotocol/filesystem/?tab=readme#installation";
        let result = extract_registry_path(url).unwrap();
        assert_eq!(result, "io.github.modelcontextprotocol/filesystem");
    }

    #[test]
    fn test_extract_registry_path_invalid() {
        let url = "https://gitlab.com/some/project";
        let result = extract_registry_path(url);
        assert!(result.is_err());
    }
}
