//! Transport configuration building utilities.

use anyhow::Result;
use mcp_client::transport::TransportConfig;
use mcp_types::TransportType;
use std::collections::HashMap;
use std::time::Duration;

/// Parse transport type from string.
pub fn parse_transport_type(transport: &str) -> Result<TransportType> {
    match transport.to_lowercase().as_str() {
        "http" => Ok(TransportType::Http),
        "stdio" => Ok(TransportType::Stdio),
        "tcp" => Ok(TransportType::Tcp),
        _ => Err(anyhow::anyhow!("Unknown transport type: {}", transport)),
    }
}

/// Parse fallback transports from string list.
pub fn parse_fallback_transports(fallbacks: &[String]) -> Result<Vec<TransportType>> {
    fallbacks.iter()
        .map(|s| parse_transport_type(s))
        .collect()
}

/// Parse HTTP headers from key:value string list.
pub fn parse_headers(headers: Option<Vec<String>>) -> Result<HashMap<String, String>> {
    let mut header_map = HashMap::new();

    if let Some(headers) = headers {
        for header in headers {
            if let Some((key, value)) = header.split_once(':') {
                header_map.insert(key.trim().to_string(), value.trim().to_string());
            } else {
                return Err(anyhow::anyhow!("Invalid header format '{}'. Expected 'key:value'", header));
            }
        }
    }

    Ok(header_map)
}

/// Build transport configuration from CLI arguments.
pub fn build_transport_config(
    endpoint: String,
    timeout: u64,
    retry_attempts: u32,
    retry_delay: u64,
    headers: Option<Vec<String>>,
    auth_token: Option<String>,
    api_key: Option<String>,
    user_agent: Option<String>,
) -> Result<TransportConfig> {
    let mut config = TransportConfig {
        endpoint,
        timeout: Duration::from_secs(timeout),
        retry_attempts,
        retry_delay: Duration::from_millis(retry_delay),
        headers: parse_headers(headers)?,
        auth_token: None,
        user_agent,
    };

    // Handle authentication
    if let Some(token) = auth_token {
        config = config.with_bearer_token(token);
    } else if let Some(key) = api_key {
        config = config.with_api_key("X-API-Key".to_string(), key);
    }

    Ok(config)
}
