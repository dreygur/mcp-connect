# MCP Remote Proxy Usage Examples

## Basic Usage

### 1. Simple HTTP Proxy

Forward requests from local STDIO to a remote HTTP MCP server:

```bash
# Start the proxy
mcp-connect proxy --endpoint "http://remote-server:8080/mcp" --debug

# The proxy will listen on STDIO and forward to the remote server
```

### 2. Proxy with Fallbacks

Use HTTP as primary, with STDIO and TCP as fallbacks:

```bash
mcp-connect proxy \
  --endpoint "http://remote-server:8080/mcp" \
  --fallbacks "stdio,tcp" \
  --timeout 30 \
  --retry-attempts 3 \
  --debug
```

### 3. Load Balancing

Distribute requests across multiple servers:

```bash
mcp-connect load-balance \
  --endpoints "http://server1:8080/mcp,http://server2:8080/mcp,http://server3:8080/mcp" \
  --transport "http" \
  --timeout 30 \
  --debug
```

### 4. STDIO Proxy

Forward to a subprocess MCP server:

```bash
mcp-connect proxy \
  --endpoint "python my-mcp-server.py" \
  --fallbacks "tcp" \
  --debug
```

### 5. TCP Proxy

Forward to a TCP MCP server:

```bash
mcp-connect proxy \
  --endpoint "localhost:9090" \
  --fallbacks "stdio" \
  --debug
```

### 6. Test Connection

Test connectivity to a remote server:

```bash
# Test HTTP connection
mcp-connect test --endpoint "http://remote-server:8080/mcp" --transport "http"

# Test TCP connection
mcp-connect test --endpoint "localhost:9090" --transport "tcp"

# Test STDIO connection
mcp-connect test --endpoint "python my-server.py" --transport "stdio"
```

## Integration Examples

### Using with Claude Desktop

Add to your Claude Desktop configuration:

```json
{
  "mcpServers": {
    "remote-proxy": {
      "command": "mcp-connect",
      "args": [
        "proxy",
        "--endpoint",
        "http://your-server:8080/mcp",
        "--fallbacks",
        "stdio,tcp"
      ]
    }
  }
}
```

### Using with OpenAI Tools

```python
import subprocess
import json

# Start the proxy
proxy = subprocess.Popen([
    "mcp-connect", "proxy",
    "--endpoint", "http://remote-server:8080/mcp",
    "--debug"
], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)

# Send initialize request
init_request = {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {
            "name": "test-client",
            "version": "1.0.0"
        }
    }
}

proxy.stdin.write(json.dumps(init_request) + "\n")
proxy.stdin.flush()

response = proxy.stdout.readline()
print("Server response:", response)
```

### Docker Deployment

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin mcp-connect

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/mcp-connect /usr/local/bin/
ENTRYPOINT ["mcp-connect"]
```

```bash
# Build and run
docker build -t mcp-connect .
docker run -i mcp-connect proxy --endpoint "http://host.docker.internal:8080/mcp"
```

## Advanced Configuration

### Custom Transport Config

```bash
# HTTP with custom timeouts and retries
mcp-connect proxy \
  --endpoint "http://slow-server:8080/mcp" \
  --timeout 60 \
  --retry-attempts 5 \
  --retry-delay 2000 \
  --debug

# Load balancing with health checks
mcp-connect load-balance \
  --endpoints "http://server1:8080/mcp,http://server2:8080/mcp" \
  --transport "http" \
  --timeout 10 \
  --retry-attempts 2 \
  --debug
```

### Logging Configuration

```bash
# Debug mode (writes to stderr)
mcp-connect proxy --endpoint "..." --debug

# Custom log level
mcp-connect proxy --endpoint "..." --log-level "warn"

# Quiet mode
mcp-connect proxy --endpoint "..." --log-level "error"
```

## Error Handling

The proxy automatically handles:

- Connection failures with fallback transports
- Network timeouts with configurable retry logic
- Protocol errors with proper JSON-RPC error responses
- Server disconnections with automatic reconnection

## Performance Notes

- HTTP transport is recommended for remote servers
- STDIO transport is best for local subprocess servers
- TCP transport offers lowest latency for local network servers
- Load balancing distributes requests round-robin
- Connection pooling is handled automatically per transport
