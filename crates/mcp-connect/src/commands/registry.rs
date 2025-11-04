//! Registry command implementations.
//!
//! Commands for exploring the MCP Registry.

use anyhow::Result;
use inquire::Confirm;
use mcp_config::ConfigManager;
use mcp_registry::RegistryClient;
use mcp_types::RegistryConfig;
use std::collections::HashMap;
use tracing::info;

/// Get registry client based on source name or default.
fn get_registry_client(source_name: Option<&str>) -> Result<RegistryClient> {
    if let Some(name) = source_name {
        if name == "default" {
            return RegistryClient::new();
        }

        // Load config to get custom registry source
        let mut manager = ConfigManager::new()?;
        if !manager.exists() {
            anyhow::bail!("No .mcp-connect.json found. Run 'mcp-connect init' first.");
        }

        let config = manager.load()?;
        if let Some(registries) = &config.registries {
            if let Some(registry_config) = registries.get(name) {
                return RegistryClient::with_base_url(
                    &registry_config.base_url,
                    registry_config.api_version.as_deref(),
                );
            }
            anyhow::bail!("Registry source '{}' not found. Use 'mcp-connect registry list-sources' to see available sources.", name);
        } else {
            anyhow::bail!("No custom registry sources configured. Use 'mcp-connect registry add-source' to add one.");
        }
    } else {
        // Use default registry from config or official registry
        let mut manager = ConfigManager::new()?;
        if manager.exists() {
            let config = manager.load()?;
            if let Some(default_name) = &config.default_registry {
                if default_name == "default" {
                    return RegistryClient::new();
                }
                if let Some(registries) = &config.registries {
                    if let Some(registry_config) = registries.get(default_name) {
                        return RegistryClient::with_base_url(
                            &registry_config.base_url,
                            registry_config.api_version.as_deref(),
                        );
                    }
                }
            }
        }
        // Fall back to official registry
        RegistryClient::new()
    }
}

/// Search the MCP Registry for servers with real-time pagination.
pub async fn search(query: &str, remote_only: bool, source_name: Option<&str>) -> Result<()> {
    let client = get_registry_client(source_name)?;

    println!("🔍 Searching registry...\n");

    const PAGE_SIZE: usize = 10;
    let mut cursor: Option<String> = None;
    let mut displayed_count = 0;
    let mut total_in_page;

    loop {
        // Fetch next page from registry (use registry's search parameter)
        let (servers, next_cursor) = client.fetch_servers_page(cursor, 50, Some(query)).await?;

        // Filter to only latest versions to avoid duplicates
        let latest_servers: Vec<_> = servers
            .into_iter()
            .filter(|sr| {
                sr.meta.official
                    .as_ref()
                    .and_then(|o| o.is_latest)
                    .unwrap_or(true)
            })
            .collect();

        // Filter by remote transport if needed
        let filtered_servers: Vec<_> = if remote_only {
            latest_servers
                .into_iter()
                .filter(|sr| RegistryClient::has_remote_transport(&sr.server))
                .collect()
        } else {
            latest_servers
        };

        if displayed_count == 0 && filtered_servers.is_empty() && next_cursor.is_none() {
            if remote_only {
                println!("🔍 No remote-compatible servers found matching '{}'", query);
            } else {
                println!("🔍 No servers found matching '{}'", query);
            }
            return Ok(());
        }

        // Display servers from this page
        total_in_page = filtered_servers.len();
        for (i, server_response) in filtered_servers.iter().enumerate() {
            let server = &server_response.server;
            displayed_count += 1;

            println!("{}. {}", displayed_count, server.name);
            println!("   📝 {}", server.description);
            println!("   📦 Version: {}", server.version);
            println!();

            // Ask user if they want to see more after every PAGE_SIZE items
            if (i + 1) % PAGE_SIZE == 0 && (i + 1) < total_in_page {
                if !prompt_continue(displayed_count, None)? {
                    show_footer(&filtered_servers);
                    return Ok(());
                }
            }
        }

        // Check if there are more pages from the registry
        if let Some(c) = next_cursor {
            cursor = Some(c);

            // Ask if user wants to fetch more
            if total_in_page > 0 {
                if !prompt_continue(displayed_count, None)? {
                    show_footer(&filtered_servers);
                    return Ok(());
                }
            }
        } else {
            // No more results
            break;
        }
    }

    if displayed_count == 0 {
        if remote_only {
            println!("🔍 No remote-compatible servers found matching '{}'", query);
        } else {
            println!("🔍 No servers found matching '{}'", query);
        }
    } else {
        show_footer(&[]);
    }

    Ok(())
}

/// Prompt user to continue viewing more results.
fn prompt_continue(displayed: usize, _total: Option<usize>) -> Result<bool> {
    println!("Showing {} servers so far...", displayed);
    match Confirm::new("Show more?")
        .with_default(true)
        .prompt()
    {
        Ok(result) => Ok(result),
        Err(_) => Ok(false), // User cancelled (Ctrl+C)
    }
}

/// Show footer with usage instructions.
fn show_footer(servers: &[mcp_registry::ServerResponse]) {
    println!("\nTo add a server: mcp-connect config add <local-name> <registry-path>");
    if let Some(first) = servers.first() {
        println!("Example: mcp-connect config add myserver {}", first.server.name);
    }
}

/// Extract registry path from URL if needed.
fn normalize_registry_path(input: &str) -> Result<String> {
    // Check if it's a URL
    if input.contains("://") || input.starts_with("github.com") || input.starts_with("www.github.com") {
        // Try to extract registry path from GitHub URL
        extract_registry_path(input)
    } else {
        // Already a registry path
        Ok(input.to_string())
    }
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
    if let Some(path) = url.strip_prefix("https://github.com/mcp/") {
        return Ok(path.to_string());
    }
    if let Some(path) = url.strip_prefix("http://github.com/mcp/") {
        return Ok(path.to_string());
    }
    if let Some(path) = url.strip_prefix("github.com/mcp/") {
        return Ok(path.to_string());
    }
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

/// Show detailed information about a specific server.
pub async fn show(registry_path: &str, source_name: Option<&str>) -> Result<()> {
    let client = get_registry_client(source_name)?;

    // Normalize the path (extract from URL if needed)
    let registry_path = normalize_registry_path(registry_path)?;

    info!("Fetching server details for: {}", registry_path);

    let server_response = client.get_server(&registry_path, "latest").await?;
    let server = &server_response.server;
    let has_remote = RegistryClient::has_remote_transport(server);

    println!("📦 {}\n", server.name);
    println!("Description: {}", server.description);

    if let Some(title) = &server.title {
        println!("Title: {}", title);
    }

    println!("Version: {} (latest)", server.version);

    if let Some(official) = &server_response.meta.official {
        println!("Status: {}", official.status);
    }

    println!("\n🌐 Remote Transport: {}", if has_remote { "✅" } else { "❌" });

    if has_remote {
        if let Some(remotes) = &server.remotes {
            for remote in remotes {
                println!("  Type: {}", remote.transport_type);
                println!("  URL: {}", remote.url);

                if let Some(headers) = &remote.headers {
                    if !headers.is_empty() {
                        println!("  Headers:");
                        for header in headers {
                            if let Some(value) = &header.value {
                                println!("    - {}: {}", header.name, value);
                            } else {
                                println!("    - {}: <not set>", header.name);
                            }
                        }
                    }
                }
            }
        }

        println!("\n➕ To add:");
        println!("mcp-connect config add <local-name> {}", server.name);
    } else {
        println!("  This server only supports STDIO transport (not compatible with mcp-connect)");
        println!("\n💡 Configure it directly in your IDE config instead.");

        if let Some(packages) = &server.packages {
            if !packages.is_empty() {
                println!("\nPackages:");
                for package in packages {
                    if let Some(version) = &package.version {
                        println!("  - {}: {} ({})", package.registry_type, package.identifier, version);
                    } else {
                        println!("  - {}: {}", package.registry_type, package.identifier);
                    }
                }
            }
        }
    }

    Ok(())
}

/// List all remote-compatible servers in the registry with real-time pagination.
pub async fn list(remote_only: bool, source_name: Option<&str>) -> Result<()> {
    let client = get_registry_client(source_name)?;

    println!("📡 Fetching servers from MCP Registry...\n");

    const PAGE_SIZE: usize = 10;
    let mut cursor: Option<String> = None;
    let mut displayed_count = 0;
    let mut total_in_page;

    loop {
        // Fetch next page from registry
        let (servers, next_cursor) = client.fetch_servers_page(cursor, 50, None).await?;

        // Filter to only latest versions to avoid duplicates
        let latest_servers: Vec<_> = servers
            .into_iter()
            .filter(|sr| {
                sr.meta.official
                    .as_ref()
                    .and_then(|o| o.is_latest)
                    .unwrap_or(true)
            })
            .collect();

        // Filter by remote transport if needed
        let filtered_servers: Vec<_> = if remote_only {
            latest_servers
                .into_iter()
                .filter(|sr| RegistryClient::has_remote_transport(&sr.server))
                .collect()
        } else {
            latest_servers
        };

        if displayed_count == 0 && filtered_servers.is_empty() && next_cursor.is_none() {
            if remote_only {
                println!("🔍 No remote-compatible servers found in the registry");
            } else {
                println!("🔍 No servers found in the registry");
            }
            return Ok(());
        }

        // Display servers from this page
        total_in_page = filtered_servers.len();
        for (i, server_response) in filtered_servers.iter().enumerate() {
            let server = &server_response.server;
            displayed_count += 1;

            println!("{}. {}", displayed_count, server.name);
            println!("   📝 {}", server.description);
            println!("   📦 Version: {}", server.version);
            println!();

            // Ask user if they want to see more after every PAGE_SIZE items
            if (i + 1) % PAGE_SIZE == 0 && (i + 1) < total_in_page {
                if !prompt_continue(displayed_count, None)? {
                    show_footer(&filtered_servers);
                    return Ok(());
                }
            }
        }

        // Check if there are more pages from the registry
        if let Some(c) = next_cursor {
            cursor = Some(c);

            // Ask if user wants to fetch more
            if total_in_page > 0 {
                if !prompt_continue(displayed_count, None)? {
                    show_footer(&filtered_servers);
                    return Ok(());
                }
            }
        } else {
            // No more results
            break;
        }
    }

    if displayed_count > 0 {
        show_footer(&[]);
    }

    Ok(())
}

/// Add a custom registry source to the configuration.
pub async fn add_source(name: &str, url: &str, api_version: Option<&str>) -> Result<()> {
    let mut manager = ConfigManager::new()?;

    if !manager.exists() {
        anyhow::bail!("No .mcp-connect.json found. Run 'mcp-connect init' first.");
    }

    let mut config = manager.load()?;

    // Initialize registries map if needed
    if config.registries.is_none() {
        config.registries = Some(HashMap::new());
    }

    let registries = config.registries.as_mut().unwrap();

    if registries.contains_key(name) {
        anyhow::bail!("Registry source '{}' already exists. Use 'mcp-connect registry remove-source {}' to remove it first.", name, name);
    }

    // Validate URL format
    if !url.starts_with("http://") && !url.starts_with("https://") {
        anyhow::bail!("Invalid URL format. Must start with http:// or https://");
    }

    // Test the registry connection
    println!("Testing connection to registry...");
    match RegistryClient::with_base_url(url, api_version) {
        Ok(client) => {
            // Try to fetch a page to verify it works
            match client.fetch_servers_page(None, 1, None).await {
                Ok(_) => println!("✓ Registry connection successful"),
                Err(e) => {
                    println!("⚠️  Warning: Could not verify registry ({}). Continuing anyway...", e);
                }
            }
        }
        Err(e) => {
            anyhow::bail!("Failed to create registry client: {}", e);
        }
    }

    // Add registry
    registries.insert(name.to_string(), RegistryConfig {
        base_url: url.to_string(),
        api_version: api_version.map(|s| s.to_string()),
    });

    manager.save(&config)?;

    println!("✓ Added registry source '{}'", name);
    println!("  URL: {}", url);
    if let Some(version) = api_version {
        println!("  API Version: {}", version);
    }

    Ok(())
}

/// List all configured registry sources.
pub fn list_sources() -> Result<()> {
    let mut manager = ConfigManager::new()?;

    if !manager.exists() {
        println!("No .mcp-connect.json found. Run 'mcp-connect init' first.");
        return Ok(());
    }

    let config = manager.load()?;

    println!("📋 Configured Registry Sources:\n");

    // Show default registry
    let default_name = config.default_registry.as_deref().unwrap_or("default");
    println!("Default: {}", default_name);
    println!("  Official MCP Registry");
    println!("  URL: https://registry.modelcontextprotocol.io");
    println!();

    // Show custom registries
    if let Some(registries) = &config.registries {
        if registries.is_empty() {
            println!("No custom registry sources configured.");
            println!("\nAdd one with: mcp-connect registry add-source <name> --url <url>");
        } else {
            for (name, registry_config) in registries {
                let is_default = config.default_registry.as_ref().map(|d| d == name).unwrap_or(false);
                println!("{} {}", if is_default { "→" } else { " " }, name);
                println!("  URL: {}", registry_config.base_url);
                if let Some(version) = &registry_config.api_version {
                    println!("  API Version: {}", version);
                }
                println!();
            }
        }
    } else {
        println!("No custom registry sources configured.");
        println!("\nAdd one with: mcp-connect registry add-source <name> --url <url>");
    }

    Ok(())
}

/// Remove a custom registry source from the configuration.
pub async fn remove_source(name: &str, force: bool) -> Result<()> {
    let mut manager = ConfigManager::new()?;

    if !manager.exists() {
        anyhow::bail!("No .mcp-connect.json found. Run 'mcp-connect init' first.");
    }

    let mut config = manager.load()?;

    if name == "default" {
        anyhow::bail!("Cannot remove the default registry source. Use 'mcp-connect registry set-default-source default' to reset.");
    }

    if let Some(registries) = &mut config.registries {
        if !registries.contains_key(name) {
            anyhow::bail!("Registry source '{}' not found.", name);
        }

        // Check if it's the default registry
        if config.default_registry.as_ref().map(|d| d == name).unwrap_or(false) {
            if !force {
                println!("⚠️  Warning: This registry source is set as the default.");
                println!("Removing it will reset the default to 'default' (official registry).");
                if !Confirm::new("Continue?").with_default(false).prompt()? {
                    return Ok(());
                }
            }
            config.default_registry = None;
        }

        registries.remove(name);

        // Remove registries map if empty
        if registries.is_empty() {
            config.registries = None;
        }

        manager.save(&config)?;
        println!("✓ Removed registry source '{}'", name);
    } else {
        anyhow::bail!("Registry source '{}' not found.", name);
    }

    Ok(())
}

/// Set the default registry source to use.
pub async fn set_default_source(name: &str) -> Result<()> {
    let mut manager = ConfigManager::new()?;

    if !manager.exists() {
        anyhow::bail!("No .mcp-connect.json found. Run 'mcp-connect init' first.");
    }

    let mut config = manager.load()?;

    if name == "default" {
        config.default_registry = None; // None means use official registry
        manager.save(&config)?;
        println!("✓ Set default registry source to 'default' (official MCP registry)");
        return Ok(());
    }

    // Verify the registry source exists
    if let Some(registries) = &config.registries {
        if !registries.contains_key(name) {
            anyhow::bail!("Registry source '{}' not found. Use 'mcp-connect registry list-sources' to see available sources.", name);
        }
    } else {
        anyhow::bail!("No custom registry sources configured. Use 'mcp-connect registry add-source' to add one first.");
    }

    config.default_registry = Some(name.to_string());
    manager.save(&config)?;

    println!("✓ Set default registry source to '{}'", name);

    Ok(())
}
