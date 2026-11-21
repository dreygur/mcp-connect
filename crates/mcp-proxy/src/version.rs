use rmcp::model::ProtocolVersion;
use serde_json::Value;

// Versions served to local clients, newest first. Capped at what the
// remotes are spoken to with, since messages pass through untranslated.
const SUPPORTED_VERSIONS: [ProtocolVersion; 3] = [
    ProtocolVersion::V_2025_06_18,
    ProtocolVersion::V_2025_03_26,
    ProtocolVersion::V_2024_11_05,
];

/// Pick the protocol version to answer a client's initialize with: the
/// requested one when supported, else our newest, as the MCP spec requires.
pub fn negotiate_version(request: &Value) -> ProtocolVersion {
    let requested = serde_json::from_value::<ProtocolVersion>(
        request["params"]["protocolVersion"].clone(),
    );

    match requested {
        Ok(version) if SUPPORTED_VERSIONS.contains(&version) => version,
        _ => SUPPORTED_VERSIONS[0].clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_request(version: &str) -> Value {
        serde_json::json!({"params": {"protocolVersion": version}})
    }

    #[test]
    fn test_negotiate_supported_version() {
        let version = negotiate_version(&init_request("2025-03-26"));
        assert_eq!(version, ProtocolVersion::V_2025_03_26);
    }

    #[test]
    fn test_negotiate_unsupported_version() {
        let version = negotiate_version(&init_request("2099-01-01"));
        assert_eq!(version, ProtocolVersion::V_2025_06_18);
    }

    #[test]
    fn test_negotiate_missing_version() {
        let version = negotiate_version(&serde_json::json!({}));
        assert_eq!(version, ProtocolVersion::V_2025_06_18);
    }
}
