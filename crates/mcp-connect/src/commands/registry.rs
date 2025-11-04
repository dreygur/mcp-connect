//! Registry command implementations.
//!
//! Commands for exploring the MCP Registry.

use anyhow::Result;
use inquire::Confirm;
use mcp_registry::RegistryClient;
use tracing::info;

/// Search the MCP Registry for servers with real-time pagination.
pub async fn search(query: &str, remote_only: bool) -> Result<()> {
    let client = RegistryClient::new()?;

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

/// Display servers with pagination (10 per page).
fn display_servers_paginated(servers: &[mcp_registry::ServerResponse]) -> Result<()> {
    const PAGE_SIZE: usize = 10;
    let total_pages = (servers.len() + PAGE_SIZE - 1) / PAGE_SIZE;

    for page in 0..total_pages {
        let start = page * PAGE_SIZE;
        let end = std::cmp::min(start + PAGE_SIZE, servers.len());
        let page_servers = &servers[start..end];

        for (i, server_response) in page_servers.iter().enumerate() {
            let server = &server_response.server;
            let global_index = start + i + 1;

            println!("{}. {}", global_index, server.name);
            println!("   📝 {}", server.description);
            println!("   📦 Version: {}", server.version);
            println!();
        }

        // Show pagination info and prompt
        if page < total_pages - 1 {
            println!("Showing {} - {} of {} servers", start + 1, end, servers.len());

            match Confirm::new("Show more?")
                .with_default(true)
                .prompt()
            {
                Ok(true) => continue,
                Ok(false) => {
                    println!("\nTo add a server: mcp-connect config add <local-name> <registry-path>");
                    if !servers.is_empty() {
                        println!("Example: mcp-connect config add myserver {}", servers[0].server.name);
                    }
                    return Ok(());
                }
                Err(_) => {
                    // User cancelled (Ctrl+C), exit gracefully
                    return Ok(());
                }
            }
        }
    }

    println!("\nTo add a server: mcp-connect config add <local-name> <registry-path>");
    if !servers.is_empty() {
        println!("Example: mcp-connect config add myserver {}", servers[0].server.name);
    }

    Ok(())
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
pub async fn show(registry_path: &str) -> Result<()> {
    let client = RegistryClient::new()?;

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
pub async fn list(remote_only: bool) -> Result<()> {
    let client = RegistryClient::new()?;

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
