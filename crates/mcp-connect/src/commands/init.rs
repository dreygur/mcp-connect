//! Init command implementation.
//!
//! Initialize a new mcp-connect project.

use anyhow::Result;
use mcp_config::ConfigManager;

/// Initialize a new mcp-connect configuration.
pub fn init(force: bool) -> Result<()> {
    let manager = ConfigManager::new()?;

    if manager.exists() && !force {
        anyhow::bail!(
            "Configuration file already exists at: .mcp-connect.json\n\
            Use --force to overwrite."
        );
    }

    if force && manager.exists() {
        println!("⚠️  Overwriting existing configuration...");
    }

    let config = manager.init()?;

    println!("✓ Initialized mcp-connect configuration\n");
    println!("Created files:");
    println!("  • .mcp-connect.json - Configuration file");
    println!("  • .env - Environment variables template\n");
    println!("Next steps:");
    println!("  1. Add your API tokens to .env");
    println!("  2. Add servers: mcp-connect config add <name> <registry-path>");
    println!("  3. Generate IDE config: mcp-connect generate-config --ide zed");
    println!("  4. Start using: mcp-connect serve\n");
    println!("Configured {} server(s)", config.servers.len());

    Ok(())
}
