# MCP Project Architecture

## Overview

This project implements a Model Context Protocol (MCP) system using the rmcp Rust crate with the following components:

## Crates Structure

```
.
├── Cargo.toml                  # Workspace configuration
├── crates/
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
│   ├── mcp-types/             # Shared types
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs         # Common types and traits
│   └── mcp-connect/            # Remote proxy executable
│       ├── Cargo.toml
│       └── src/
│           └── main.rs        # CLI application
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
- **Configuration**: Shared configuration structures

### 5. Remote Proxy Executable (`mcp-connect`)

**Responsibilities:**

- CLI application that ties everything together
- Configure and run the proxy with specified parameters

**Key Features:**

- **CLI Interface**: Command-line configuration
- **Transport Selection**: Choose primary and fallback transports
- **Logging Configuration**: Configure debug and notification logging

## Protocol Flow

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
        │                       │                       │
   ┌────▼────┐              ┌───▼───┐              ┌────▼────┐
   │ stdin/  │              │Network│              │ Remote  │
   │ stdout  │              │Transpt│              │ Service │
   └─────────┘              └───────┘              └─────────┘
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

### Phase 4: Integration and Testing

1. Create `mcp-connect` CLI application
2. Add comprehensive testing
3. Create usage examples

## Key Design Decisions

1. **Async-First**: All components use async/await with tokio runtime
2. **Error Handling**: Comprehensive error types with proper propagation
3. **Transport Abstraction**: Clean interfaces allowing multiple transport implementations
4. **Configuration-Driven**: Behavior controlled through CLI flags and configuration
5. **Protocol Compliance**: Strict adherence to MCP specification requirements
