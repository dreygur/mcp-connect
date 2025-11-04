# MCP Remote Proxy

Ever wanted to connect your local MCP client to a remote server but hit a wall with transport compatibility? This Rust-based proxy bridges that gap, letting you connect local MCP applications to remote HTTP servers with a unified configuration system.

## What it does

This tool provides a centralized way to manage and connect to multiple remote MCP servers. Instead of configuring each server separately in your IDE, configure them once in `.mcp-connect.json` and access all servers through a single multiplexing proxy.

Key capabilities:

- **Central Configuration** - Manage all remote servers in one `.mcp-connect.json` file
- **MCP Registry Integration** - Search and add servers from the official MCP Registry
- **Multiplexing Server** - Access multiple remote servers through one connection
- **Namespace Routing** - Tools from different servers are prefixed (e.g., `github/search_code`)
- **Environment Variables** - Secure credential management via `.env` files
- **IDE Integration** - Auto-generate IDE configs (Zed supported)
- **HTTP/HTTPS Only** - Focus on remote HTTP servers (STDIO servers configured directly in IDE)
- Full compatibility with MCP 2024-11-05 specification

## How it's built

The project follows a modular crate structure:

```
├── crates/
│   ├── mcp-types/      # Common data types and interfaces
│   ├── mcp-server/     # Server-side MCP implementation
│   ├── mcp-client/     # Client that talks to remote servers
│   ├── mcp-proxy/      # Message forwarding and routing
│   ├── mcp-registry/   # MCP Registry API client
│   ├── mcp-config/     # Configuration management
│   └── mcp-connect/    # Command-line interface
└── examples/           # Sample usage and tests
```

Here's what each crate does:

- **mcp-types**: Shared types, traits, and error definitions
- **mcp-server**: Handles the local side, talking to your MCP client via STDIO
- **mcp-client**: Connects to remote servers using HTTP transport
- **mcp-proxy**: Message forwarding with multiplexing and namespace routing
- **mcp-registry**: Search and fetch servers from the official MCP Registry
- **mcp-config**: Load, save, and validate `.mcp-connect.json` configurations
- **mcp-connect**: CLI tool that ties everything together

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

## Quick Start

### 1. Initialize Project

Create configuration files in your project directory:

```bash
mcp-connect init
```

This creates:
- `.mcp-connect.json` - Server configuration file
- `.env` - Environment variables template

### 2. Browse and Add Servers

Search the official MCP Registry and add servers:

```bash
# Search the registry
mcp-connect registry search github

# Show details for a specific server
mcp-connect registry show modelcontextprotocol/github-mcp-server

# Add a server from the registry
mcp-connect config add github modelcontextprotocol/github-mcp-server

# Add with interactive search
mcp-connect config add context7 --search

# Add custom server with URL
mcp-connect config add my-server \
  --url "https://my-mcp-server.com/mcp" \
  --auth-header "Authorization: Bearer ${MY_TOKEN}"
```

### 3. Configure Credentials

Edit the `.env` file to add your API tokens:

```bash
# .env
GITHUB_TOKEN=ghp_xxxxxxxxxxxxx
CONTEXT7_API_KEY=ctx7sk_xxxxxxxxxxxxx
MY_TOKEN=xxxxxxxxxxxxx
```

### 4. Generate IDE Configuration

Auto-generate IDE-specific config files:

```bash
# Generate Zed configuration
mcp-connect generate-config --ide zed

# Or specify custom output location
mcp-connect generate-config --ide zed --output ~/.config/zed/settings.json
```

### 5. Start Using MCP Servers

Start the multiplexing server (usually done automatically by your IDE):

```bash
mcp-connect serve
```

Your IDE will now have access to all configured servers. Tools are namespaced by server name:
- `github/search_code`
- `github/create_issue`
- `context7/search_docs`
- `my-server/custom_tool`

### Configuration Management

```bash
# List configured servers
mcp-connect config list

# Show details for a server
mcp-connect config show github

# Test server connectivity
mcp-connect config test github
mcp-connect config test --all

# Remove a server
mcp-connect config remove github

# Validate configuration file
mcp-connect config validate
```

### Registry Commands

```bash
# Search for servers
mcp-connect registry search "file system"

# List all available servers
mcp-connect registry list

# Show only servers with remote HTTP support
mcp-connect registry list --remote-only

# Show server details
mcp-connect registry show modelcontextprotocol/server-filesystem
```

### Legacy Proxy Mode

For advanced users, the direct proxy mode is still available:

```bash
# Simple HTTP proxy
mcp-connect proxy --endpoint "http://remote-server:8080/mcp" --debug

# With authentication
mcp-connect proxy \
  --endpoint "https://api.githubcopilot.com/mcp" \
  --headers "Authorization: Bearer ${TOKEN}" \
  --debug
```

### Global Options

All commands support these options:

```bash
--debug           # Enable debug logging
--log-level info  # Set log level (trace, debug, info, warn, error)
```

## Configuration

### .mcp-connect.json

The main configuration file for managing all remote MCP servers:

```json
{
  "$schema": "https://mcp.run/schema/config.json",
  "version": "1.0",
  "env_file": ".env",
  "routing": {
    "method": "namespace_prefix",
    "separator": "/"
  },
  "servers": {
    "github": {
      "name": "GitHub MCP Server",
      "description": "Access GitHub repositories, issues, and code search",
      "version": "1.0.0",
      "remote": {
        "transport_type": "streamable_http",
        "url": "https://api.githubcopilot.com/mcp",
        "headers": [
          {
            "key": "Authorization",
            "value": "Bearer ${GITHUB_TOKEN}"
          }
        ]
      },
      "timeout": 30,
      "retry_attempts": 3
    },
    "context7": {
      "name": "Context7 MCP Server",
      "description": "Search and analyze code documentation",
      "version": "0.1.0",
      "remote": {
        "transport_type": "streamable_http",
        "url": "https://mcp.context7.com/mcp",
        "headers": [
          {
            "key": "Authorization",
            "value": "Bearer ${CONTEXT7_API_KEY}"
          }
        ]
      },
      "timeout": 30,
      "retry_attempts": 3
    }
  }
}
```

### Environment Variables (.env)

Store sensitive credentials in a `.env` file:

```bash
# .env
GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxx
CONTEXT7_API_KEY=ctx7sk_xxxxxxxxxxxxxxxxxxxxxxxxx
```

The configuration manager automatically loads `.env` and substitutes `${VAR_NAME}` placeholders in the config file.

### Generated IDE Configuration

After running `mcp-connect generate-config --ide zed`, your Zed configuration will include:

```json
{
  "context_servers": {
    "mcp-connect": {
      "source": "custom",
      "command": "/path/to/mcp-connect",
      "args": ["serve"]
    }
  }
}
```

The multiplexing server automatically starts when Zed loads and provides access to all configured servers.

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

With the multiplexing approach:

```json
{
  "mcpServers": {
    "mcp-connect": {
      "command": "mcp-connect",
      "args": ["serve"],
      "env": {
        "GITHUB_TOKEN": "your-github-token",
        "CONTEXT7_API_KEY": "your-context7-key"
      }
    }
  }
}
```

Or using legacy proxy mode for a single server:

```json
{
  "mcpServers": {
    "github": {
      "command": "mcp-connect",
      "args": [
        "proxy",
        "--endpoint",
        "https://api.githubcopilot.com/mcp",
        "--headers",
        "Authorization: Bearer ${GITHUB_TOKEN}"
      ],
      "env": {
        "GITHUB_TOKEN": "your-github-token"
      }
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
WORKDIR /workspace
ENTRYPOINT ["mcp-connect"]
```

```bash
# Build and run in multiplexing mode
docker build -t mcp-connect .
docker run -i \
  -v $(pwd)/.mcp-connect.json:/workspace/.mcp-connect.json \
  -v $(pwd)/.env:/workspace/.env \
  mcp-connect serve

# Or legacy proxy mode
docker run -i mcp-connect proxy --endpoint "http://host.docker.internal:8080/mcp"
```

## CLI Commands

### `init`

Initialize a new mcp-connect project.

```bash
mcp-connect init [--force]
```

**Options:**
- `--force`: Overwrite existing configuration

**Creates:**
- `.mcp-connect.json` - Configuration file
- `.env` - Environment variables template

### `registry`

Browse and search the official MCP Registry.

#### `registry search`

Search for servers in the registry:

```bash
mcp-connect registry search <query> [--remote-only]
```

**Options:**
- `--remote-only`: Show only servers with HTTP/HTTPS support

#### `registry show`

Show details for a specific server:

```bash
mcp-connect registry show <registry-path>
```

**Example:** `mcp-connect registry show modelcontextprotocol/github-mcp-server`

#### `registry list`

List all available servers:

```bash
mcp-connect registry list [--remote-only]
```

### `config`

Manage server configurations.

#### `config add`

Add a new server to configuration:

```bash
# From registry
mcp-connect config add <name> <registry-path>

# From registry with interactive search
mcp-connect config add <name> --search

# From custom URL
mcp-connect config add <name> --url <url> [--auth-header <header>]
```

**Examples:**
```bash
mcp-connect config add github modelcontextprotocol/github-mcp-server
mcp-connect config add context7 --search
mcp-connect config add my-server --url "https://example.com/mcp" --auth-header "Authorization: Bearer ${TOKEN}"
```

#### `config list`

List all configured servers:

```bash
mcp-connect config list
```

#### `config show`

Show details for a configured server:

```bash
mcp-connect config show <name>
```

#### `config remove`

Remove a server from configuration:

```bash
mcp-connect config remove <name> [--force]
```

#### `config test`

Test server connectivity:

```bash
# Test specific server
mcp-connect config test <name>

# Test all servers
mcp-connect config test --all
```

#### `config validate`

Validate configuration file:

```bash
mcp-connect config validate
```

### `serve`

Start the multiplexing MCP server:

```bash
mcp-connect serve [--debug]
```

This command:
- Loads configuration from `.mcp-connect.json`
- Connects to all configured remote servers
- Provides a unified STDIO interface to your IDE
- Routes requests based on namespace prefixes

### `generate-config`

Generate IDE-specific configuration files:

```bash
mcp-connect generate-config --ide <ide> [--output <path>]
```

**Supported IDEs:**
- `zed` - Zed editor

**Options:**
- `--output`: Custom output path (default: IDE's standard config location)

### Legacy Commands

The following commands are still available for advanced use cases:

#### `proxy`

Run as a direct proxy to a single server:

```bash
mcp-connect proxy --endpoint <url> [options]
```

**Options:**
- `--endpoint`: Remote server endpoint
- `--headers`: Custom headers (comma-separated)
- `--timeout`: Connection timeout in seconds
- `--retry-attempts`: Number of retry attempts
- `--debug`: Enable debug logging

#### `load-balance`

Load balance across multiple endpoints:

```bash
mcp-connect load-balance --endpoints <urls> [options]
```

**Options:**
- `--endpoints`: Comma-separated server URLs
- `--transport`: Transport type (default: http)
- `--timeout`: Connection timeout
- `--retry-attempts`: Retry attempts

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
