//! Streamable HTTP MCP remote for tests. Tracks sessions like a real
//! server: initialize opens one, an unknown session gets 404, DELETE ends it.

use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

const SESSION_HEADER: &str = "mcp-session-id";
const NOT_FOUND: &str = "HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\n\r\n";
const ACCEPTED: &str = "HTTP/1.1 202 Accepted\r\ncontent-length: 0\r\n\r\n";
const NOT_ALLOWED: &str = "HTTP/1.1 405 Method Not Allowed\r\ncontent-length: 0\r\n\r\n";
const OK_EMPTY: &str = "HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n";

/// One HTTP request as the remote saw it.
pub struct Hit {
    pub http_method: String,
    pub headers: HashMap<String, String>,
    pub body: Value,
}

/// What the remote saw, plus its live sessions.
#[derive(Default)]
pub struct MockState {
    pub hits: Vec<Hit>,
    pub sessions: HashSet<String>,
    next_session: u32,
}

impl MockState {
    /// JSON-RPC methods received, in order.
    pub fn methods(&self) -> Vec<String> {
        self.hits
            .iter()
            .filter_map(|h| h.body["method"].as_str().map(str::to_string))
            .collect()
    }
}

pub type Shared = Arc<Mutex<MockState>>;

fn json_reply(id: &Value, result: Value, session: Option<&str>) -> String {
    let out = json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string();
    let session = session
        .map(|s| format!("{SESSION_HEADER}: {s}\r\n"))
        .unwrap_or_default();

    format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n{session}content-length: {}\r\n\r\n{out}",
        out.len()
    )
}

// A progress notification first, then the response, as servers stream it.
fn sse_reply(id: &Value) -> String {
    let progress = json!({"jsonrpc": "2.0", "method": "notifications/progress", "params": {"progress": 1}});
    let response = json!({"jsonrpc": "2.0", "id": id, "result": {"content": [{"type": "text", "text": "streamed"}]}});
    let body = format!("event: message\ndata: {progress}\n\nevent: message\ndata: {response}\n\n");

    format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn tool_result(msg: &Value) -> Value {
    let text = msg["params"]["arguments"]["text"].as_str().unwrap_or_default();
    json!({"content": [{"type": "text", "text": text}]})
}

async fn reply(state: &Shared, http_method: &str, headers: &HashMap<String, String>, msg: &Value) -> String {
    let mut state = state.lock().await;
    let session = headers.get(SESSION_HEADER).cloned().unwrap_or_default();

    if http_method == "DELETE" {
        state.sessions.remove(&session);
        return OK_EMPTY.to_string();
    }
    if http_method != "POST" {
        return NOT_ALLOWED.to_string();
    }

    if msg["method"] == "initialize" {
        state.next_session += 1;
        let new_session = format!("s{}", state.next_session);
        state.sessions.insert(new_session.clone());
        let result = json!({
            "protocolVersion": msg["params"]["protocolVersion"],
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "mock", "version": "1"}
        });
        return json_reply(&msg["id"], result, Some(&new_session));
    }

    if !state.sessions.contains(&session) {
        return NOT_FOUND.to_string();
    }
    let Some(id) = msg.get("id") else {
        return ACCEPTED.to_string();
    };

    match msg["method"].as_str() {
        Some("tools/list") => json_reply(id, json!({"tools": [{"name": "echo", "inputSchema": {"type": "object"}}]}), None),
        Some("tools/call") if msg["params"]["name"] == "sse" => sse_reply(id),
        Some("tools/call") => json_reply(id, tool_result(msg), None),
        _ => json_reply(id, json!({}), None),
    }
}

async fn serve_conn(stream: tokio::net::TcpStream, state: Shared) {
    let mut stream = BufReader::new(stream);
    loop {
        let mut line = String::new();
        if stream.read_line(&mut line).await.unwrap_or(0) == 0 {
            return;
        }
        let http_method = line.split_whitespace().next().unwrap_or_default().to_string();

        let mut headers = HashMap::new();
        loop {
            let mut header = String::new();
            stream.read_line(&mut header).await.unwrap();
            if header == "\r\n" {
                break;
            }
            if let Some((key, value)) = header.split_once(':') {
                headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
            }
        }

        let len = headers.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
        let mut body = vec![0; len];
        stream.read_exact(&mut body).await.unwrap();
        let msg: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);

        let out = reply(&state, &http_method, &headers, &msg).await;
        state.lock().await.hits.push(Hit { http_method, headers, body: msg });
        stream.get_mut().write_all(out.as_bytes()).await.unwrap();
    }
}

/// Start the remote on a free port; returns its URL and what it saw.
pub async fn mock_remote() -> (String, Shared) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let state: Shared = Default::default();
    let accept_state = state.clone();

    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            tokio::spawn(serve_conn(stream, accept_state.clone()));
        }
    });

    (url, state)
}

/// Transport config for the mock: one attempt, no delay, so failures show fast.
pub fn mock_config(url: String) -> mcp_client::transport::TransportConfig {
    mcp_client::transport::TransportConfig {
        endpoint: url,
        retry_attempts: 1,
        retry_delay: std::time::Duration::ZERO,
        ..Default::default()
    }
}
