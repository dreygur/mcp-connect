// Simple JSON-RPC stdio proxy test without rmcp
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Starting simple JSON-RPC proxy...");

    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break; // EOF
        }

        eprintln!("Received: {}", line.trim());

        // Try to parse as JSON-RPC
        if let Ok(request) = serde_json::from_str::<Value>(&line) {
            if let Some(method) = request.get("method").and_then(|m| m.as_str()) {
                let id = request.get("id");

                let response = match method {
                    "initialize" => {
                        eprintln!("Handling initialize request");
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "protocolVersion": "2024-11-05",
                                "capabilities": {
                                    "tools": {"listChanged": false}
                                },
                                "serverInfo": {
                                    "name": "simple-proxy",
                                    "version": "0.1.0"
                                }
                            }
                        })
                    }
                    "ping" => {
                        eprintln!("Handling ping request");
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {}
                        })
                    }
                    "tools/list" => {
                        eprintln!("Handling tools/list request");
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "tools": []
                            }
                        })
                    }
                    _ => {
                        eprintln!("Unknown method: {}", method);
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {
                                "code": -32601,
                                "message": "Method not found"
                            }
                        })
                    }
                };

                let response_str = serde_json::to_string(&response)?;
                eprintln!("Sending: {}", response_str);
                stdout.write_all(response_str.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
            }
        }
    }

    eprintln!("Proxy stopped");
    Ok(())
}
