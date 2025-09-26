# MCP Remote - Architecture Overview

## Project Structure

```
mcp-remote-rs/
├── Cargo.toml              # Workspace configuration
├── README.md               # Project documentation
├── ARCHITECTURE.md         # Architecture overview
├── examples/               # Usage examples
│   └── simple_usage.md
└── crates/                 # All Rust crates
    ├── mcp-types/          # Shared MCP protocol types
    │   ├── Cargo.toml
    │   └── src/
    │       └── lib.rs      # JSON-RPC and MCP types
    ├── mcp-client/         # Remote MCP server client
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs      # Public API
    │       ├── client.rs   # MCP client implementation
    │       ├── error.rs    # Error types
    │       ├── types.rs    # Type re-exports
    │       └── transport/  # Transport implementations
    │           ├── mod.rs  # Transport trait
    │           ├── http.rs # HTTP transport
    │           └── sse.rs  # SSE transport
    ├── mcp-server/         # Local MCP server (STDIO)
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs      # Public API
    │       ├── server.rs   # MCP server implementation
    │       ├── error.rs    # Error types
    │       ├── types.rs    # Type re-exports
    │       └── transport/  # Transport implementations
    │           ├── mod.rs  # Transport trait
    │           └── stdio.rs # STDIO transport
    ├── mcp-proxy/          # Proxy coordination
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs      # Public API
    │       ├── proxy.rs    # Main proxy logic
    │       ├── error.rs    # Error types
    │       └── strategy.rs # Transport strategy
    └── mcp-remote/         # Main CLI binary
        ├── Cargo.toml
        └── src/
            └── main.rs     # CLI interface and main logic
```

## Component Responsibilities

### mcp-types

- Shared MCP protocol types (JSON-RPC, MCP messages)
- Ensures type consistency across all crates
- Serialization/deserialization with serde

### mcp-client

- **Purpose**: Connect to remote MCP servers
- **Transports**: HTTP POST, Server-Sent Events (SSE)
- **Features**:
  - Automatic transport strategy selection
  - Connection management
  - Request/response handling
  - Tool calling support

### mcp-server

- **Purpose**: Provide local MCP server interface
- **Transport**: STDIO (stdin/stdout)
- **Features**:
  - Compatible with IDE MCP clients
  - Request handling and routing
  - Tool registration and management

### mcp-proxy

- **Purpose**: Bridge client and server components
- **Features**:
  - Bidirectional message forwarding
  - Transport strategy management
  - Tool forwarding (planned)
  - Connection lifecycle management

### mcp-remote

- **Purpose**: CLI interface and application entry point
- **Features**:
  - Command-line argument parsing
  - Configuration management
  - Logging setup
  - Signal handling

## Data Flow

```
1. IDE/LLM sends MCP request via STDIO
   ↓
2. mcp-server receives request on stdin
   ↓
3. mcp-proxy forwards request to mcp-client
   ↓
4. mcp-client sends HTTP/SSE request to remote server
   ↓
5. Remote server processes and responds
   ↓
6. Response flows back through the same path in reverse
```

## Transport Strategy System

The proxy supports multiple transport strategies:

- **HTTP-First**: Try HTTP POST, fallback to SSE on 404
- **SSE-First**: Try SSE, fallback to HTTP POST on 405
- **HTTP-Only**: Only HTTP POST requests
- **SSE-Only**: Only SSE connections

This allows compatibility with different remote MCP server implementations.

## Error Handling

Each crate defines its own error types that convert appropriately:

- `mcp_client::ClientError`
- `mcp_server::ServerError`
- `mcp_proxy::ProxyError`

Errors flow up through the proxy to provide meaningful feedback to users.

## Security Model

- **HTTPS by default**: HTTP only allowed with explicit `--allow-http`
- **URL validation**: Prevents invalid or malicious URLs
- **Transport isolation**: Client and server components are isolated
- **No credential storage**: Relies on remote server authentication

## Extension Points

The architecture supports future enhancements:

- **OAuth integration**: Can be added to mcp-client transport layer
- **Tool filtering**: Already planned in mcp-proxy
- **Custom transports**: Transport trait allows new implementations
- **Middleware**: Proxy layer can be extended with request/response middleware
