//! Registry command implementations.
//!
//! Commands for exploring the MCP Registry.

use anyhow::Result;
use mcp_registry::RegistryClient;
use tracing::info;

/// Search the MCP Registry for servers.
pub async fn search(query: &str, remote_only: bool) -> Result<()> {
    let client = RegistryClient::new()?;
    let servers = client.search(query).await?;

    if servers.is_empty() {
        println!("🔍 No servers found matching '{}'", query);
        return Ok(());
    }

    println!("🔍 Found {} server(s) matching '{}':\n", servers.len(), query);

    for (i, server) in servers.iter().enumerate() {
        // Fetch details to check if it has remote transport
        match client.get_server(&server.name, &server.version).await {
            Ok(detail) => {
                let has_remote = RegistryClient::has_remote_transport(&detail);

                // Skip if remote_only is true and server doesn't have remote transport
                if remote_only && !has_remote {
                    continue;
                }

                let icon = if has_remote { "✅" } else { "❌" };
                let transport_info = if has_remote {
                    "Remote: HTTPS ✓"
                } else {
                    "Remote: STDIO only (not compatible)"
                };

                println!("{} {}. {}", icon, i + 1, server.name);
                if let Some(desc) = &server.description {
                    println!("   📝 {}", desc);
                }
                println!("   🌐 {}", transport_info);
                println!("   📦 Version: {}", server.version);
                println!();
            }
            Err(e) => {
                eprintln!("   ⚠️  Failed to fetch details for {}: {}", server.name, e);
            }
        }
    }

    println!("To add: mcp-connect config add <local-name> <registry-path>");
    println!("Example: mcp-connect config add github {}", servers[0].name);

    Ok(())
}

/// Show detailed information about a specific server.
pub async fn show(registry_path: &str) -> Result<()> {
    let client = RegistryClient::new()?;

    info!("Fetching server details for: {}", registry_path);

    let server = client.get_server(registry_path, "latest").await?;
    let has_remote = RegistryClient::has_remote_transport(&server);

    println!("📦 {}\n", server.name);

    if let Some(desc) = &server.description {
        println!("Description: {}", desc);
    }

    println!("Version: {} (latest)", server.version);
    println!("Status: {}", server.status);

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
                            println!("    - {}: {}", header.key, header.value);
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
                    println!("  - {}: {} ({})", package.registry_type, package.identifier, package.version);
                }
            }
        }
    }

    Ok(())
}

/// List all remote-compatible servers in the registry.
pub async fn list(remote_only: bool) -> Result<()> {
    let client = RegistryClient::new()?;

    println!("📡 Fetching servers from MCP Registry...\n");

    // This will fetch all servers using pagination
    let servers = client.search("").await?;

    let mut compatible_count = 0;
    let mut incompatible_count = 0;

    for server in servers {
        match client.get_server(&server.name, &server.version).await {
            Ok(detail) => {
                let has_remote = RegistryClient::has_remote_transport(&detail);

                if remote_only && !has_remote {
                    incompatible_count += 1;
                    continue;
                }

                if has_remote {
                    compatible_count += 1;
                    println!("✅ {}", server.name);
                } else {
                    incompatible_count += 1;
                    if !remote_only {
                        println!("❌ {} (STDIO only)", server.name);
                    }
                }

                if let Some(desc) = &server.description {
                    println!("   {}", desc);
                }
                println!();
            }
            Err(_) => {
                // Skip servers that fail to fetch
                continue;
            }
        }
    }

    println!("\n📊 Summary:");
    println!("   Compatible (HTTP): {}", compatible_count);
    if !remote_only {
        println!("   STDIO only: {}", incompatible_count);
    }

    Ok(())
}
