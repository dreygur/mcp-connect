# MCP Project Architecture

## Overview

This project implements a Model Context Protocol (MCP) system using the rmcp Rust crate with the following components:

## Crates Structure

```
.
├── Cargo.toml                  # Workspace configuration
├── crates/
│   ├── mcp-types/             # Shared types
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs         # Common types and traits
│   ├── mcp-server/            # MCP Server implementation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs      # Core server logic
│   │       └── error.rs       # Error types
│   ├── mcp-client/            # MCP Client implementation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs      # Core client logic
│   │       ├── transport/     # Transport implementations
│   │       │   └── mod.rs
│   │       └── error.rs       # Error types
│   ├── mcp-proxy/             # Proxy implementation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── proxy.rs       # Core proxy logic
│   │       ├── stdio_proxy.rs # STDIO-specific proxy
│   │       ├── strategy.rs    # Proxy strategy patterns
│   │       └── error.rs       # Error types
│   ├── mcp-registry/          # MCP Registry API client
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs         # Registry search and fetch
│   ├── mcp-config/            # Configuration management
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs         # Config loading and env vars
│   └── mcp-connect/           # CLI application
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs        # CLI entry point
│           └── commands/      # Command implementations
│               ├── mod.rs
│               ├── init.rs
│               ├── registry.rs
│               ├── config.rs
│               ├── serve.rs
│               └── generate.rs
└── examples/
    ├── simple_usage.md
    └── minimal_server_test.rs
```

## Component Design

### 1. MCP Server (`mcp-server`)

**Responsibilities:**

- Read/write from/to STDIO using JSON-RPC protocol
- Handle MCP protocol messages (requests, responses, notifications)
- Implement logging strategy based on `--debug` flag

**Key Features:**

- **STDIO Transport**: Read JSON-RPC messages from stdin, write to stdout
- **Logging Strategy**:
  - If `--debug` flag: write logs to STDIO
  - Otherwise: use `notifications/message` to send logs as notifications and write to STDERR
  - No timestamps or colors for `notifications/message` logs
- **Message Processing**: Handle standard MCP messages (ping, initialize, tools, resources, etc.)

**Dependencies:**

- `rmcp` for MCP protocol implementation
- `tokio` for async runtime
- `serde_json` for JSON handling
- `clap` for CLI argument parsing

### 2. MCP Client (`mcp-client`)

**Responsibilities:**

- Connect to remote MCP servers using rmcp
- Support multiple transport protocols with fallbacks
- Provide async client interface for MCP operations

**Key Features:**

- **Primary Transport**: HTTPStream protocol (Streamable HTTP)
- **Fallback Transports**: STDIO, TCP
- **Protocol Support**: Full MCP protocol including tools, resources, prompts
- **Connection Management**: Automatic reconnection and error handling

**Dependencies:**

- `rmcp` for MCP protocol implementation
- `tokio` for async runtime
- `reqwest` for HTTP client
- `serde_json` for JSON handling

### 3. MCP Proxy (`mcp-proxy`)

**Responsibilities:**

- Forward requests between MCP server and client bidirectionally
- Handle protocol translation if needed
- Manage connection lifecycle

**Key Features:**

- **Bidirectional Forwarding**: Server ↔ Client message routing
- **Protocol Bridging**: Handle differences between transports
- **Error Handling**: Graceful degradation and error propagation
- **Session Management**: Maintain connection state

**Dependencies:**

- `mcp-server` and `mcp-client` crates
- `tokio` for async runtime
- `futures` for stream handling

### 4. Shared Types (`mcp-types`)

**Responsibilities:**

- Common error types
- Shared traits and interfaces
- Configuration structures

**Key Features:**

- **Error Types**: Unified error handling across crates
- **Traits**: Common interfaces for servers, clients, and proxies
- **Configuration**: Shared configuration structures (ConnectConfig, ServerConfig, etc.)

**Dependencies:**

- `serde` for serialization
- `serde_json` for JSON handling
- `async-trait` for async traits

### 5. MCP Registry (`mcp-registry`)

**Responsibilities:**

- Search and browse the official MCP Registry
- Fetch server details and metadata
- Convert registry format to internal configuration

**Key Features:**

- **Registry Search**: Search for servers by keyword
- **Server Details**: Fetch full server metadata including transports
- **Remote Filtering**: Identify servers with HTTP/HTTPS support
- **Format Conversion**: Convert registry server.json format to internal RemoteConfig

**Dependencies:**

- `mcp-types` for shared types
- `reqwest` for HTTP client
- `serde_json` for JSON parsing
- `urlencoding` for URL encoding

### 6. Configuration Management (`mcp-config`)

**Responsibilities:**

- Load and save `.mcp-connect.json` configuration files
- Manage environment variable substitution
- Validate configuration structure

**Key Features:**

- **Config Loading**: Parse and validate `.mcp-connect.json`
- **Environment Variables**: Load `.env` files and substitute `${VAR}` placeholders
- **Config Initialization**: Create default configuration files
- **Validation**: Ensure configuration integrity

**Dependencies:**

- `mcp-types` for shared types
- `dotenvy` for .env file loading
- `regex` for environment variable substitution
- `serde_json` for JSON handling

### 7. CLI Application (`mcp-connect`)

**Responsibilities:**

- Command-line interface for all mcp-connect operations
- Orchestrate registry, config, and server components
- Generate IDE-specific configuration files

**Key Features:**

- **Initialization**: `init` command to create configuration files
- **Registry Commands**: Browse and search MCP Registry
- **Config Commands**: Add, list, remove, test servers
- **Multiplexing Server**: `serve` command runs multiplexing proxy
- **IDE Integration**: `generate-config` creates IDE-specific configs
- **Legacy Proxy**: Direct proxy mode for single servers

**Commands:**

- `init` - Initialize project with config files
- `registry search/show/list` - Browse MCP Registry
- `config add/list/show/remove/test/validate` - Manage servers
- `serve` - Start multiplexing server
- `generate-config` - Generate IDE configs
- `proxy` (legacy) - Direct proxy to single server
- `load-balance` (legacy) - Load balance across servers

**Dependencies:**

- All internal crates (mcp-server, mcp-client, mcp-proxy, mcp-registry, mcp-config)
- `clap` for CLI argument parsing
- `tokio` for async runtime
- `tracing` for logging

## Protocol Flow

### Direct Proxy Mode (Legacy)

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Client    │◄──►│   MCP Proxy     │◄──►│  Remote MCP     │
│   (IDE/Agent)   │    │                 │    │   Server        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
        │                       │                       │
        │ STDIO/JSON-RPC         │ HTTPStream            │
        │                       │                       │
   ┌────▼────┐              ┢───▼───┐              ┌────▼────┐
   │ stdin/  │              │Network│              │ Remote  │
   │ stdout  │              │Transpt│              │ Service │
   └─────────┘              └───────┘              └─────────┘
```

### Multiplexing Mode (New)

```
┌──────────────┐         ┌────────────────────┐
│  IDE/Agent   │◄───────►│  mcp-connect serve │
│              │  STDIO  │  (Multiplexer)     │
└──────────────┘         └────────┬───────────┘
                                  │
                    ┌─────────────┼─────────────┐
                    │             │             │
                    │ HTTPStream  │ HTTPStream  │
                    ▼             ▼             ▼
           ┌────────────┐ ┌────────────┐ ┌────────────┐
           │  GitHub    │ │ Context7   │ │  Custom    │
           │  Server    │ │  Server    │ │  Server    │
           └────────────┘ └────────────┘ └────────────┘

Namespace Routing:
• github/search_code → GitHub Server
• context7/search_docs → Context7 Server
• custom/my_tool → Custom Server

Tools/Resources aggregated from all servers
```

## Implementation Strategy

### Phase 1: Core Types and Server

1. Create `mcp-types` with basic error types and traits
2. Implement `mcp-server` with STDIO transport and logging
3. Add CLI argument parsing for debug mode

### Phase 2: Client Implementation

1. Implement `mcp-client` with HTTPStream transport
2. Add fallback transport mechanisms
3. Implement connection management and retry logic

### Phase 3: Proxy Implementation

1. Create `mcp-proxy` for message forwarding
2. Implement bidirectional communication
3. Add error handling and session management

### Phase 4: Basic CLI Integration

1. Create `mcp-connect` CLI application
2. Implement direct proxy mode
3. Add load balancing support

### Phase 5: Registry and Configuration (Completed)

1. Create `mcp-registry` crate for MCP Registry API
2. Create `mcp-config` crate for configuration management
3. Extend `mcp-types` with configuration structures
4. Implement environment variable substitution

### Phase 6: Centralized Configuration (Completed)

1. Implement `init` command for project initialization
2. Add registry commands (search, show, list)
3. Add config commands (add, list, show, remove, test, validate)
4. Implement IDE config generation

### Phase 7: Multiplexing Server (Completed)

1. Implement `serve` command with MultiplexingStrategy
2. Add namespace routing for tools and resources
3. Implement aggregation of capabilities from multiple servers
4. Test with multiple concurrent servers

## Key Design Decisions

1. **Async-First**: All components use async/await with tokio runtime
2. **Error Handling**: Comprehensive error types with proper propagation
3. **Transport Abstraction**: Clean interfaces allowing multiple transport implementations
4. **Configuration-Driven**: Single `.mcp-connect.json` file for all servers
5. **Protocol Compliance**: Strict adherence to MCP specification requirements
6. **Namespace Routing**: Tool/resource names prefixed with server name for disambiguation
7. **HTTP-Only Focus**: Only remote HTTP/HTTPS servers supported; STDIO servers configured directly in IDE
8. **Environment Variables**: Secure credential management via `.env` files with `${VAR}` substitution
9. **Modular Crates**: Each feature in separate crate for clean architecture
