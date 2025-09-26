use serde_json;

fn main() {
    let response = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{"logging":{},"completion":{},"prompts":{},"resources":{"subscribe":true,"listChanged":true},"tools":{"listChanged":true}},"serverInfo":{"name":"github-mcp-server","version":"github-mcp-server/remote-2d5f3db039c1be25c60bdfbd3f70741d82c8ec13"}}}"#;
    
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(response);
    match parsed {
        Ok(json) => println!("Parsed JSON: {:#}", json),
        Err(e) => println!("Parse error: {}", e),
    }
}
EOF </dev/null