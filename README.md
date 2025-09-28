# MCP Remote Proxy

Ever wanted to connect your local MCP client to a remote server but hit a wall with transport compatibility? This Rust-based proxy bridges that gap, letting you connect local MCP applications to remote servers regardless of how they communicate.

## What it does

This tool acts as a translator between your local MCP client and remote servers. It supports multiple ways to connect (HTTP, STDIO, TCP) and automatically falls back to alternatives if one doesn't work. Plus, it handles OAuth authentication, load balancing across multiple servers, and gives you detailed logging when things go wrong.

Key capabilities:

- Connect via HTTP, STDIO, or TCP - whatever works
- OAuth 2.1 authentication for secure connections
- Smart fallbacks when connections fail
- Load balancing across multiple remote servers
- Detailed debug logging to troubleshoot issues
- Full compatibility with MCP 2024-11-05 specification

## How it's built

The project is split into several focused modules:

```
├── crates/
│   ├── mcp-types/      # Common data types and interfaces
│   ├── mcp-server/     # Server-side MCP implementation
│   ├── mcp-client/     # Client that talks to remote servers
│   ├── mcp-proxy/      # The magic happens here - message forwarding
│   └── mcp-connect/     # Command-line tool you'll actually use
└── examples/           # Sample usage and tests
```

Here's what each piece does:

- **mcp-server**: Handles the local side, talking to your MCP client via STDIO
- **mcp-client**: Connects to remote servers using HTTP, STDIO, or TCP
- **mcp-proxy**: Sits in the middle, forwarding messages back and forth
- **mcp-connect**: The CLI tool that ties everything together

## Getting started

You'll need Rust 1.75+ and Cargo installed. Then it's pretty straightforward:

```bash
# Clone and build
git clone <repository-url>
cd tokio-night-gnome
cargo build --release

# Or install it system-wide
cargo install --path crates/mcp-connect
```

### Troubleshooting

**OpenSSL errors** (like `libssl.so.3: cannot open shared object file`):

```bash
# Install OpenSSL 3.x
sudo apt install libssl3 libssl-dev  # Ubuntu/Debian
sudo dnf install openssl-devel       # Fedora/RHEL

# Or rebuild with static OpenSSL
cargo clean
OPENSSL_STATIC=1 cargo build --release
```

**Connection errors** (like `MCP error -32000: Connection closed`):

This usually means authentication is missing or wrong:

```bash
# Test if the endpoint needs auth
mcp-connect test --endpoint "https://your-server.com/mcp"

# Add authentication (Context7 example)
mcp-connect proxy \
  --endpoint "https://mcp.context7.com/mcp" \
  --auth-token "ctx7sk-your-api-key" \
  --debug

# Check what's happening with full debug
mcp-connect proxy \
  --endpoint "https://your-server.com/mcp" \
  --debug \
  --log-level "debug"
```

## Usage

### Simple HTTP proxy

Want to connect your local MCP client to a remote HTTP server? Just point it at the endpoint:

```bash
mcp-connect proxy --endpoint "http://remote-server:8080/mcp" --debug
```

### Authentication

Most real servers need authentication. Here are the common patterns:

```bash
# Bearer token (like GitHub Copilot)
mcp-connect proxy \
  --endpoint "https://api.githubcopilot.com/mcp" \
  --auth-token "your-bearer-token" \
  --debug

# API key
mcp-connect proxy \
  --endpoint "https://api.example.com/mcp" \
  --api-key "your-api-key" \
  --debug

# OAuth 2.1 flow (for more complex auth)
mcp-connect auth-proxy \
  --endpoint "https://oauth-server.com/mcp" \
  --client-id "your-client-id" \
  --client-secret "your-client-secret" \
  --auth-url "https://oauth-server.com/oauth/authorize" \
  --token-url "https://oauth-server.com/oauth/token" \
  --redirect-url "http://localhost:8080/callback" \
  --scopes "read,write" \
  --debug

# Custom headers for anything else
mcp-connect proxy \
  --endpoint "http://remote-server:8080/mcp" \
  --headers "Authorization:Bearer token123,X-Custom:value" \
  --debug
```

### Fallbacks and reliability

Sometimes connections fail. The proxy can try different transport methods automatically:

```bash
# Try HTTP first, fall back to STDIO then TCP
mcp-connect proxy \
  --endpoint "http://remote-server:8080/mcp" \
  --fallbacks "stdio,tcp" \
  --timeout 30 \
  --retry-attempts 3 \
  --retry-delay 1000 \
  --debug
```

### Load balancing

Got multiple servers? Spread the load:

```bash
mcp-connect load-balance \
  --endpoints "http://server1:8080/mcp,http://server2:8080/mcp,http://server3:8080/mcp" \
  --transport "http" \
  --timeout 30 \
  --retry-attempts 3 \
  --auth-token "your-token" \
  --debug
```

### Test Connection

Test connectivity to a remote server:

```bash
# Test HTTP connection
mcp-connect test --endpoint "http://remote-server:8080/mcp" --transport "http"

# Test with authentication
mcp-connect test \
  --endpoint "https://api.githubcopilot.com/mcp" \
  --transport "http" \
  --auth-token "your-token"

# Test TCP connection
mcp-connect test --endpoint "localhost:9090" --transport "tcp"

# Test STDIO connection
mcp-connect test --endpoint "python my-server.py" --transport "stdio"
```

### Notification Demo

Test MCP notification system:

```bash
# Send 3 demo notifications
mcp-connect notification-demo --count 3
```

### Global Options

All commands support these global options:

```bash
# Enable debug logging
--debug

# Set custom log level
--log-level "info"  # trace, debug, info, warn, error
```

## Configuration

### Environment Variables

Both `.zed/settings.json` and `inspector.config.json` support environment variables for secure credential management:

**.zed/settings.json:**

```json
{
  "context_servers": {
    "Context7": {
      "source": "custom",
      "command": "./target/release/mcp-connect",
      "args": [
        "proxy",
        "--endpoint",
        "https://mcp.context7.com/mcp",
        "--headers",
        "\"Authorization: Bearer ${PAT_CONTEXT7}\""
      ],
      "env": {
        "PAT_CONTEXT7": "your-api-key"
      }
    },
    "Github": {
      "source": "custom",
      "command": "./target/release/mcp-connect",
      "args": [
        "proxy",
        "--endpoint",
        "https://api.githubcopilot.com/mcp",
        "--headers",
        "\"Authorization: Bearer ${PAT_GITHUB}\""
      ],
      "env": {
        "PAT_GITHUB": "your-github-token"
      }
    }
  }
}
```

**inspector.config.json:**

```json
{
  "mcpServers": {
    "github": {
      "command": "./target/release/mcp-connect",
      "args": [
        "proxy",
        "--endpoint",
        "https://api.githubcopilot.com/mcp",
        "--headers",
        "\"Authorization: Bearer ${PAT_GITHUB}\""
      ],
      "env": {
        "PAT_GITHUB": "your-github-token"
      }
    },
    "Context7": {
      "command": "./target/release/mcp-connect",
      "args": [
        "proxy",
        "--endpoint",
        "https://mcp.context7.com/mcp",
        "--headers",
        "\"Authorization: Bearer ${PAT_CONTEXT7}\""
      ],
      "env": {
        "PAT_CONTEXT7": "your-api-key"
      }
    }
  }
}
```

You can override these values by setting environment variables in your shell:

```bash
export GITHUB_TOKEN="your-actual-token"
export CONTEXT7_API_KEY="your-actual-api-key"
```

### Transport Types

1. **HTTP (Streamable HTTP)**: Primary transport for remote servers
   - Supports MCP-Session-Id headers
   - Handles 202 Accepted responses
   - Full OAuth 2.1 authentication support

2. **STDIO**: For subprocess-based MCP servers
   - Spawns and manages subprocesses
   - JSON-RPC over stdin/stdout
   - Automatic process lifecycle management

3. **TCP**: Direct TCP socket connections
   - Low-latency for local network servers
   - Connection pooling and retry logic
   - Automatic reconnection on failures

### Logging Strategies

The server implements different logging strategies based on the `--debug` flag:

- **Debug Mode**: Logs written to STDIO as MCP notifications
- **Production Mode**: Uses `notifications/message` and writes to STDERR
- **No timestamps/colors** for `notifications/message` logs (MCP compliance)

## Integration Examples

### Claude Desktop Configuration

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

## CLI Commands

### `proxy`

Run as a proxy server (STDIO mode)

**Options:**

- `--endpoint`: Primary remote server endpoint
- `--fallbacks`: Comma-separated fallback transport types
- `--timeout`: Connection timeout in seconds (default: 30)
- `--retry-attempts`: Number of retry attempts (default: 3)
- `--retry-delay`: Retry delay in milliseconds (default: 1000)

### `load-balance`

Run with load balancing across multiple endpoints

**Options:**

- `--endpoints`: Comma-separated remote server endpoints
- `--transport`: Transport type for all endpoints (default: http)
- `--timeout`: Connection timeout in seconds (default: 30)
- `--retry-attempts`: Number of retry attempts (default: 3)
- `--retry-delay`: Retry delay in milliseconds (default: 1000)

### `test`

Test connection to a remote MCP server

**Options:**

- `--endpoint`: Remote server endpoint
- `--transport`: Transport type (default: http)
- `--timeout`: Connection timeout in seconds (default: 10)

## Protocol Details

### MCP Compliance

This implementation follows the MCP 2024-11-05 specification:

- **Initialization**: Proper client-server handshake
- **JSON-RPC 2.0**: All messages use JSON-RPC format
- **STDIO Transport**: Newline-delimited messages, no embedded newlines
- **HTTP Transport**: POST requests with 202 Accepted responses
- **Error Handling**: Proper JSON-RPC error responses

### Message Flow

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Client    │◄──►│   MCP Proxy     │◄──►│  Remote MCP     │
│   (Local)       │    │                 │    │   Server        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
        │                       │                       │
        │ STDIO/JSON-RPC         │ HTTPStream            │
        │                       │ (primary)             │
        │                       │ STDIO/TCP             │
        │                       │ (fallbacks)           │
```

## Testing

### Run Tests

```bash
# Check compilation
cargo check --workspace

# Build all crates
cargo build --workspace

# Run with debug output
cargo run --bin mcp-connect -- proxy --endpoint "http://localhost:8080/mcp" --debug
```

### Integration Testing

The proxy has been tested with:

- Multiple concurrent connections
- Transport fallback scenarios
- Connection timeout and retry logic
- Load balancing across multiple servers
- Error handling and recovery

## API Documentation

### Core Traits

```rust
#[async_trait]
pub trait McpServer: Send + Sync {
    async fn start(&mut self) -> Result<()>;
    async fn handle_message(&mut self, message: &str) -> Result<Option<String>>;
    async fn shutdown(&mut self) -> Result<()>;
}

#[async_trait]
pub trait McpClient: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn send_request(&mut self, request: &str) -> Result<String>;
    async fn disconnect(&mut self) -> Result<()>;
}

#[async_trait]
pub trait McpClientTransport: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn send_request(&mut self, request: &str) -> Result<String>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn is_connected(&self) -> bool;
}
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [RMCP](https://docs.rs/rmcp) - Rust SDK for Model Context Protocol
- [Tokio](https://tokio.rs/) - Asynchronous runtime for Rust
- [Clap](https://clap.rs/) - Command Line Argument Parser
- [Serde](https://serde.rs/) - Serialization framework

## Support

For questions and support:

- Open an issue on GitHub
- Check the [examples](examples/) directory for usage patterns
- Review the [architecture documentation](ARCHITECTURE.md)

---

Built with ❤️ in Rust 🦀
