# MCP Remote Proxy

A Rust implementation of a Model Context Protocol (MCP) remote proxy system that enables bridging local MCP clients to remote MCP servers with multiple transport options and fallback mechanisms.

## Features

- **Multiple Transport Support**: HTTP (Streamable HTTP), STDIO, and TCP transports
- **Fallback Mechanisms**: Automatic fallback between transport types on connection failure
- **Load Balancing**: Distribute requests across multiple remote servers
- **Debug Logging**: Configurable logging with `--debug` flag support
- **Protocol Compliance**: Full MCP 2024-11-05 protocol specification compliance
- **Async/Await**: Built with Tokio for high-performance async operations

## Architecture

The project is organized as a Rust workspace with the following crates:

```
├── crates/
│   ├── mcp-types/      # Shared types and traits
│   ├── mcp-server/     # MCP server implementation with STDIO transport
│   ├── mcp-client/     # MCP client with multiple transport support
│   ├── mcp-proxy/      # Proxy implementation with strategies
│   └── mcp-remote/     # CLI application
└── examples/           # Usage examples and tests
```

### Component Overview

- **MCP Server** (`mcp-server`): STDIO-based MCP server with configurable debug logging
- **MCP Client** (`mcp-client`): Multi-transport client supporting HTTP, STDIO, and TCP
- **MCP Proxy** (`mcp-proxy`): Bidirectional message forwarding with multiple strategies
- **MCP Remote** (`mcp-remote`): CLI tool for running proxies and testing connections

## Installation

### Prerequisites

- Rust 1.75 or later
- Cargo

### Build from Source

```bash
git clone <repository-url>
cd tokio-night-gnome
cargo build --release
```

### Install Locally

```bash
cargo install --path crates/mcp-remote
```

## Usage

### Basic HTTP Proxy

Forward requests from local STDIO to a remote HTTP MCP server:

```bash
mcp-remote proxy --endpoint "http://remote-server:8080/mcp" --debug
```

### Proxy with Authentication

Use Bearer token or API key authentication:

```bash
# With Bearer token
mcp-remote proxy \
  --endpoint "https://api.githubcopilot.com/mcp" \
  --auth-token "your-bearer-token" \
  --debug

# With API key
mcp-remote proxy \
  --endpoint "https://api.example.com/mcp" \
  --api-key "your-api-key" \
  --debug

# With custom headers
mcp-remote proxy \
  --endpoint "http://remote-server:8080/mcp" \
  --headers "Authorization:Bearer token123,X-Custom:value" \
  --debug
```

### Proxy with Fallbacks

Use HTTP as primary, with STDIO and TCP as fallbacks:

```bash
mcp-remote proxy \
  --endpoint "http://remote-server:8080/mcp" \
  --fallbacks "stdio,tcp" \
  --timeout 30 \
  --retry-attempts 3 \
  --retry-delay 1000 \
  --debug
```

### Load Balancing

Distribute requests across multiple servers:

```bash
mcp-remote load-balance \
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
mcp-remote test --endpoint "http://remote-server:8080/mcp" --transport "http"

# Test with authentication
mcp-remote test \
  --endpoint "https://api.githubcopilot.com/mcp" \
  --transport "http" \
  --auth-token "your-token"

# Test TCP connection
mcp-remote test --endpoint "localhost:9090" --transport "tcp"

# Test STDIO connection
mcp-remote test --endpoint "python my-server.py" --transport "stdio"
```

### Notification Demo

Test MCP notification system:

```bash
# Send 3 demo notifications
mcp-remote notification-demo --count 3
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
      "command": "npx",
      "args": [
        "-y",
        "@upstash/context7-mcp",
        "--api-key",
        "${CONTEXT7_API_KEY}"
      ],
      "env": {
        "CONTEXT7_API_KEY": "your-api-key"
      }
    },
    "Github": {
      "source": "custom",
      "command": "npx",
      "args": [
        "-y",
        "mcp-remote",
        "https://api.githubcopilot.com/mcp",
        "--header",
        "Authorization: Bearer ${GITHUB_TOKEN}"
      ],
      "env": {
        "GITHUB_TOKEN": "your-github-token"
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
      "command": "./target/release/mcp-remote",
      "args": [
        "proxy",
        "--endpoint",
        "https://api.githubcopilot.com/mcp",
        "--headers",
        "\"Authorization: Bearer ${GITHUB_TOKEN}\""
      ],
      "env": {
        "GITHUB_TOKEN": "your-github-token"
      }
    },
    "cc": {
      "command": "./target/release/mcp-remote",
      "args": [
        "proxy",
        "--endpoint",
        "https://mcp.context7.com/mcp",
        "--headers",
        "\"Authorization: Bearer ${CONTEXT7_API_KEY}\""
      ],
      "env": {
        "CONTEXT7_API_KEY": "your-api-key"
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
   - OAuth 2.1 ready (future enhancement)

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
      "command": "mcp-remote",
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
RUN cargo build --release --bin mcp-remote

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/mcp-remote /usr/local/bin/
ENTRYPOINT ["mcp-remote"]
```

```bash
# Build and run
docker build -t mcp-remote .
docker run -i mcp-remote proxy --endpoint "http://host.docker.internal:8080/mcp"
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
cargo run --bin mcp-remote -- proxy --endpoint "http://localhost:8080/mcp" --debug
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
