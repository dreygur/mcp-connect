//! Generate IDE configuration command.
//!
//! Generate IDE-specific configuration files for mcp-connect.

use anyhow::{Context, Result};
use mcp_config::ConfigManager;
use serde_json::json;
use std::fs;
use std::path::Path;

/// Supported IDEs for config generation.
#[derive(Debug, Clone, Copy)]
pub enum IdeType {
    Zed,
    Vscode,
    Cursor,
}

impl std::str::FromStr for IdeType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "zed" => Ok(IdeType::Zed),
            "vscode" | "vs-code" | "code" => Ok(IdeType::Vscode),
            "cursor" => Ok(IdeType::Cursor),
            _ => anyhow::bail!("Unsupported IDE: {}. Supported: zed, vscode, cursor", s),
        }
    }
}

/// Generate IDE-specific configuration file.
pub fn generate(ide: IdeType, output: Option<String>) -> Result<()> {
    let mut manager = ConfigManager::new()?;

    if !manager.exists() {
        anyhow::bail!("No .mcp-connect.json found. Run 'mcp-connect init' first.");
    }

    let config = manager.load()?;

    if config.servers.is_empty() {
        println!("⚠️  Warning: No servers configured in .mcp-connect.json");
        println!("Add servers with: mcp-connect config add <name> <registry-path>");
    }

    match ide {
        IdeType::Zed => generate_zed_config(&config, output.as_deref())?,
        IdeType::Vscode => generate_vscode_config(&config, output.as_deref())?,
        IdeType::Cursor => generate_cursor_config(&config, output.as_deref())?,
    }

    Ok(())
}

/// Generate Zed IDE configuration.
fn generate_zed_config(_config: &mcp_types::ConnectConfig, output: Option<&str>) -> Result<()> {
    // Get the mcp-connect binary path
    let binary_path = std::env::current_exe().context("Failed to get current executable path")?;

    let binary_path_str = binary_path
        .to_str()
        .context("Binary path is not valid UTF-8")?;

    // Create Zed config structure
    let zed_config = json!({
        "context_servers": {
            "mcp-connect": {
                "source": "custom",
                "command": binary_path_str,
                "args": ["serve"]
            }
        }
    });

    // Determine output path
    let output_path = if let Some(path) = output {
        Path::new(path).to_path_buf()
    } else {
        // Default Zed config location
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .context("Failed to get home directory")?;

        Path::new(&home).join(".config/zed/settings.json")
    };

    // Check if file already exists
    let (existing_config, is_new) = if output_path.exists() {
        let content =
            fs::read_to_string(&output_path).context("Failed to read existing Zed config")?;

        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(config) => (config, false),
            Err(_) => {
                println!("⚠️  Warning: Existing config file is not valid JSON");
                println!("Creating backup at: {}.backup", output_path.display());
                fs::copy(&output_path, format!("{}.backup", output_path.display()))?;
                (json!({}), true)
            }
        }
    } else {
        // Create parent directory if it doesn't exist
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }
        (json!({}), true)
    };

    // Merge configurations
    let merged_config = if is_new {
        zed_config
    } else {
        merge_json_objects(existing_config, zed_config)
    };

    // Write config file
    let json_str =
        serde_json::to_string_pretty(&merged_config).context("Failed to serialize config")?;

    fs::write(&output_path, json_str).context("Failed to write config file")?;

    println!(
        "✓ Generated Zed configuration at: {}",
        output_path.display()
    );
    println!("\n📝 Configuration added:");
    println!("   • mcp-connect server configured");
    println!("\n💡 Next steps:");
    println!("   1. Restart Zed editor");
    println!("   2. mcp-connect will automatically start when Zed loads");
    println!("   3. Check that servers are accessible in Zed's context panel");

    Ok(())
}

/// Generate VSCode IDE configuration.
fn generate_vscode_config(_config: &mcp_types::ConnectConfig, output: Option<&str>) -> Result<()> {
    // Get the mcp-connect binary path
    let binary_path = std::env::current_exe().context("Failed to get current executable path")?;

    let binary_path_str = binary_path
        .to_str()
        .context("Binary path is not valid UTF-8")?;

    // Create VSCode config structure
    let vscode_config = json!({
        "mcp.servers": {
            "mcp-connect": {
                "command": binary_path_str,
                "args": ["serve"]
            }
        }
    });

    // Determine output path
    let output_path = if let Some(path) = output {
        Path::new(path).to_path_buf()
    } else {
        // Default VSCode config location (workspace settings)
        let current_dir = std::env::current_dir().context("Failed to get current directory")?;
        current_dir.join(".vscode/settings.json")
    };

    // Check if file already exists
    let (existing_config, is_new) = if output_path.exists() {
        let content =
            fs::read_to_string(&output_path).context("Failed to read existing VSCode config")?;

        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(config) => (config, false),
            Err(_) => {
                println!("⚠️  Warning: Existing config file is not valid JSON");
                println!("Creating backup at: {}.backup", output_path.display());
                fs::copy(&output_path, format!("{}.backup", output_path.display()))?;
                (json!({}), true)
            }
        }
    } else {
        // Create parent directory if it doesn't exist
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }
        (json!({}), true)
    };

    // Merge configurations
    let merged_config = if is_new {
        vscode_config
    } else {
        merge_json_objects(existing_config, vscode_config)
    };

    // Write config file
    let json_str =
        serde_json::to_string_pretty(&merged_config).context("Failed to serialize config")?;

    fs::write(&output_path, json_str).context("Failed to write config file")?;

    println!(
        "✓ Generated VSCode configuration at: {}",
        output_path.display()
    );
    println!("\n📝 Configuration added:");
    println!("   • mcp-connect server configured");
    println!("\n💡 Next steps:");
    println!("   1. Restart VSCode (or reload window)");
    println!("   2. Install the MCP extension if not already installed");
    println!("   3. mcp-connect will automatically start when VSCode loads");
    println!("   4. Check that servers are accessible in VSCode's MCP panel");

    Ok(())
}

/// Generate Cursor IDE configuration.
fn generate_cursor_config(_config: &mcp_types::ConnectConfig, output: Option<&str>) -> Result<()> {
    // Get the mcp-connect binary path
    let binary_path = std::env::current_exe().context("Failed to get current executable path")?;

    let binary_path_str = binary_path
        .to_str()
        .context("Binary path is not valid UTF-8")?;

    // Create Cursor config structure (same as VSCode)
    let cursor_config = json!({
        "mcp.servers": {
            "mcp-connect": {
                "command": binary_path_str,
                "args": ["serve"]
            }
        }
    });

    // Determine output path
    let output_path = if let Some(path) = output {
        Path::new(path).to_path_buf()
    } else {
        // Default Cursor config location (workspace settings)
        // Cursor uses the same .vscode directory as VSCode
        let current_dir = std::env::current_dir().context("Failed to get current directory")?;
        current_dir.join(".vscode/settings.json")
    };

    // Check if file already exists
    let (existing_config, is_new) = if output_path.exists() {
        let content =
            fs::read_to_string(&output_path).context("Failed to read existing Cursor config")?;

        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(config) => (config, false),
            Err(_) => {
                println!("⚠️  Warning: Existing config file is not valid JSON");
                println!("Creating backup at: {}.backup", output_path.display());
                fs::copy(&output_path, format!("{}.backup", output_path.display()))?;
                (json!({}), true)
            }
        }
    } else {
        // Create parent directory if it doesn't exist
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }
        (json!({}), true)
    };

    // Merge configurations
    let merged_config = if is_new {
        cursor_config
    } else {
        merge_json_objects(existing_config, cursor_config)
    };

    // Write config file
    let json_str =
        serde_json::to_string_pretty(&merged_config).context("Failed to serialize config")?;

    fs::write(&output_path, json_str).context("Failed to write config file")?;

    println!(
        "✓ Generated Cursor configuration at: {}",
        output_path.display()
    );
    println!("\n📝 Configuration added:");
    println!("   • mcp-connect server configured");
    println!("\n💡 Next steps:");
    println!("   1. Restart Cursor (or reload window)");
    println!("   2. Install the MCP extension if not already installed");
    println!("   3. mcp-connect will automatically start when Cursor loads");
    println!("   4. Check that servers are accessible in Cursor's MCP panel");

    Ok(())
}

/// Merge two JSON objects, with new values taking precedence.
fn merge_json_objects(
    mut base: serde_json::Value,
    overlay: serde_json::Value,
) -> serde_json::Value {
    if let (Some(base_obj), Some(overlay_obj)) = (base.as_object_mut(), overlay.as_object()) {
        for (key, value) in overlay_obj {
            if let Some(base_value) = base_obj.get_mut(key) {
                if base_value.is_object() && value.is_object() {
                    *base_value = merge_json_objects(base_value.clone(), value.clone());
                } else {
                    *base_value = value.clone();
                }
            } else {
                base_obj.insert(key.clone(), value.clone());
            }
        }
        base
    } else {
        overlay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_json_objects() {
        let base = json!({
            "existing": "value",
            "context_servers": {
                "old_server": {
                    "command": "old"
                }
            }
        });

        let overlay = json!({
            "context_servers": {
                "mcp-connect": {
                    "command": "new"
                }
            }
        });

        let merged = merge_json_objects(base, overlay);

        assert_eq!(merged["existing"], "value");
        assert_eq!(merged["context_servers"]["old_server"]["command"], "old");
        assert_eq!(merged["context_servers"]["mcp-connect"]["command"], "new");
    }
}
