//! Configuration file management for mcp-connect.
//!
//! This crate handles loading, saving, and validating `.mcp-connect.json`
//! configuration files, as well as environment variable substitution.

use anyhow::{Context, Result};
use mcp_types::ConnectConfig;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Default configuration file name
pub const DEFAULT_CONFIG_FILE: &str = ".mcp-connect.json";

/// Default environment file name
pub const DEFAULT_ENV_FILE: &str = ".env";

/// Configuration manager for mcp-connect.
///
/// Handles loading, saving, and managing the central configuration file.
pub struct ConfigManager {
    config_path: PathBuf,
    env_vars: HashMap<String, String>,
}

impl ConfigManager {
    /// Create a new ConfigManager with the default config path in current directory.
    pub fn new() -> Result<Self> {
        let config_path = std::env::current_dir()?.join(DEFAULT_CONFIG_FILE);
        Self::with_path(config_path)
    }

    /// Create a new ConfigManager with a specific config file path.
    pub fn with_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config_path = path.as_ref().to_path_buf();
        let env_vars = HashMap::new();

        Ok(Self {
            config_path,
            env_vars,
        })
    }

    /// Load environment variables from .env file if it exists.
    pub fn load_env(&mut self) -> Result<()> {
        // Try to load from .env file in current directory
        let env_path = std::env::current_dir()?.join(DEFAULT_ENV_FILE);

        if env_path.exists() {
            debug!("Loading environment variables from: {}", env_path.display());
            dotenvy::from_path(&env_path)
                .context("Failed to load .env file")?;
            info!("Loaded environment variables from .env file");
        } else {
            debug!("No .env file found at: {}", env_path.display());
        }

        // Load all environment variables into our map
        for (key, value) in std::env::vars() {
            self.env_vars.insert(key, value);
        }

        Ok(())
    }

    /// Load configuration from file.
    pub fn load(&mut self) -> Result<ConnectConfig> {
        info!("Loading configuration from: {}", self.config_path.display());

        let content = fs::read_to_string(&self.config_path)
            .with_context(|| format!("Failed to read config file: {}", self.config_path.display()))?;

        let mut config: ConnectConfig = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", self.config_path.display()))?;

        // Load environment variables
        self.load_env()?;

        // Substitute environment variables in the config
        self.substitute_env_vars(&mut config)?;

        info!("Configuration loaded successfully with {} servers", config.servers.len());
        Ok(config)
    }

    /// Save configuration to file.
    pub fn save(&self, config: &ConnectConfig) -> Result<()> {
        info!("Saving configuration to: {}", self.config_path.display());

        let json = serde_json::to_string_pretty(config)
            .context("Failed to serialize configuration")?;

        fs::write(&self.config_path, json)
            .with_context(|| format!("Failed to write config file: {}", self.config_path.display()))?;

        info!("Configuration saved successfully");
        Ok(())
    }

    /// Check if configuration file exists.
    pub fn exists(&self) -> bool {
        self.config_path.exists()
    }

    /// Create a new configuration file with default settings.
    pub fn init(&self) -> Result<ConnectConfig> {
        info!("Initializing new configuration file");

        if self.exists() {
            anyhow::bail!(
                "Configuration file already exists at: {}",
                self.config_path.display()
            );
        }

        let config = ConnectConfig::default();
        self.save(&config)?;

        // Also create a template .env file
        let env_path = std::env::current_dir()?.join(DEFAULT_ENV_FILE);
        if !env_path.exists() {
            let env_template = "# Environment variables for mcp-connect\n\
                               # Add your API keys and tokens here\n\
                               # Example:\n\
                               # GITHUB_TOKEN=ghp_xxxxx\n\
                               # CONTEXT7_API_KEY=ctx7sk_xxxxx\n";
            fs::write(&env_path, env_template)
                .context("Failed to create .env template")?;
            info!("Created .env template file");
        }

        Ok(config)
    }

    /// Substitute environment variables in configuration.
    ///
    /// Replaces ${VAR_NAME} patterns with actual environment variable values.
    fn substitute_env_vars(&self, config: &mut ConnectConfig) -> Result<()> {
        let re = Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}")?;

        for server_config in config.servers.values_mut() {
            // Substitute in URL
            server_config.remote.url = self.substitute_string(&re, &server_config.remote.url)?;

            // Substitute in headers
            if let Some(headers) = &mut server_config.remote.headers {
                for header in headers.iter_mut() {
                    header.value = self.substitute_string(&re, &header.value)?;
                }
            }
        }

        Ok(())
    }

    /// Substitute environment variables in a single string.
    fn substitute_string(&self, re: &Regex, input: &str) -> Result<String> {
        let mut result = input.to_string();

        for captures in re.captures_iter(input) {
            let var_name = &captures[1];
            let placeholder = &captures[0];

            match self.env_vars.get(var_name) {
                Some(value) => {
                    debug!("Substituting ${{{}}}", var_name);
                    result = result.replace(placeholder, value);
                }
                None => {
                    warn!("Environment variable not found: {}", var_name);
                    // Keep the placeholder if variable not found
                }
            }
        }

        Ok(result)
    }

    /// Validate configuration structure.
    pub fn validate(config: &ConnectConfig) -> Result<()> {
        if config.servers.is_empty() {
            warn!("Configuration contains no servers");
        }

        for (name, server) in &config.servers {
            // Validate URL format
            if !server.remote.url.starts_with("http://") && !server.remote.url.starts_with("https://") {
                anyhow::bail!("Server '{}' has invalid URL: {}", name, server.remote.url);
            }

            // Check for remaining unsubstituted variables (indicates missing env vars)
            let re = Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}")?;
            if re.is_match(&server.remote.url) {
                warn!("Server '{}' URL contains unsubstituted variables: {}", name, server.remote.url);
            }

            if let Some(headers) = &server.remote.headers {
                for header in headers {
                    if re.is_match(&header.value) {
                        warn!("Server '{}' header '{}' contains unsubstituted variables", name, header.key);
                    }
                }
            }
        }

        info!("Configuration validation passed");
        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default ConfigManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_substitution() {
        let mut manager = ConfigManager::new().unwrap();
        manager.env_vars.insert("TEST_TOKEN".to_string(), "secret123".to_string());

        let re = Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").unwrap();
        let result = manager.substitute_string(&re, "Bearer ${TEST_TOKEN}").unwrap();

        assert_eq!(result, "Bearer secret123");
    }

    #[test]
    fn test_config_validation() {
        let config = ConnectConfig::default();
        assert!(ConfigManager::validate(&config).is_ok());
    }
}
