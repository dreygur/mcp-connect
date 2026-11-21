//! Client and HTTP transport behavior against a streamable HTTP remote.

use crate::mock_remote::{mock_config, mock_remote, Shared};
use mcp_client::McpRemoteClient;
use mcp_types::McpClient;
use serde_json::Value;

async fn ready_client() -> (McpRemoteClient, Shared) {
    let (url, state) = mock_remote().await;
    let mut client = McpRemoteClient::new_with_config(mock_config(url), vec![]);
    client.connect().await.unwrap();
    client.initialize().await.unwrap();
    (client, state)
}

async fn call_tool(client: &mut McpRemoteClient, name: &str, text: &str) -> Value {
    let request = serde_json::json!({
        "jsonrpc": "2.0", "id": 42, "method": "tools/call",
        "params": {"name": name, "arguments": {"text": text}}
    });
    let response = client.send_request(&request.to_string()).await.unwrap();
    serde_json::from_str(&response).unwrap()
}

#[tokio::test]
async fn test_session_recovers_after_expiry() {
    let (client, state) = ready_client().await;
    state.lock().await.sessions.clear();

    client.list_tools().await.unwrap();

    let methods = state.lock().await.methods();
    assert_eq!(
        methods[methods.len() - 3..],
        ["initialize", "notifications/initialized", "tools/list"]
    );
}

#[tokio::test]
async fn test_protocol_version_header() {
    let (client, state) = ready_client().await;
    client.list_tools().await.unwrap();

    let state = state.lock().await;
    let header = |method: &str| {
        state
            .hits
            .iter()
            .find(|h| h.body["method"] == method)
            .and_then(|h| h.headers.get("mcp-protocol-version").cloned())
    };
    assert_eq!(header("initialize"), None);
    assert_eq!(header("tools/list").as_deref(), Some("2025-06-18"));
}

#[tokio::test]
async fn test_disconnect_ends_session() {
    let (mut client, state) = ready_client().await;
    client.disconnect().await.unwrap();

    let state = state.lock().await;
    let last = state.hits.last().unwrap();
    assert_eq!(last.http_method, "DELETE");
    assert_eq!(last.headers.get("mcp-session-id").map(String::as_str), Some("s1"));
    assert!(state.sessions.is_empty());
}

#[tokio::test]
async fn test_json_with_data_text_kept() {
    let (mut client, _state) = ready_client().await;
    let response = call_tool(&mut client, "echo", "data: x").await;
    assert_eq!(response["result"]["content"][0]["text"], "data: x");
}

#[tokio::test]
async fn test_sse_picks_response() {
    let (mut client, _state) = ready_client().await;
    let response = call_tool(&mut client, "sse", "").await;
    assert_eq!(response["id"], 42);
    assert_eq!(response["result"]["content"][0]["text"], "streamed");
}
