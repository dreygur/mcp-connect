# MCP-Remote Rust Implementation System Prompt

## Critical API Usage Guidelines

**MANDATORY VERIFICATION REQUIREMENTS:**

- The LLM MUST be 100% certain before using ANY API or making assumptions
- NO guessing, NO assumptions, NO hallucination of API details
- ALWAYS verify library documentation using Context7 MCP server before implementation
- ALWAYS use GitHub MCP server for accessing repositories from github.com domain
- DO NOT read example codes from repositories - read official documentation only

## MCP Server Configuration

**VERIFIED MCP Servers (configured in IDE):**

- `mcp__Context7` - For library documentation retrieval
- `mcp__Github` - For GitHub repository access

**Usage Protocol:**

1. Before using any library: Query Context7 for official documentation
2. Before accessing GitHub repos: Use GitHub MCP server tools
3. Never assume API exists - always verify through proper channels

## Project Overview

You are tasked with implementing a Rust version of the `mcp-remote` project, which is a bridge/proxy that allows local MCP (Model Context Protocol) clients to connect to remote MCP servers with OAuth authentication support.

**IMPORTANT: Use Official rmcp SDK**

- Primary dependency: `rmcp = "0.7.0"` (official Rust SDK for MCP)
- Repository: https://github.com/modelcontextprotocol/rust-sdk
- Documentation: https://docs.rs/rmcp
- This SDK provides official MCP protocol types, transports, and utilities

## Previous Instructions (Maintained)

You are Claude Code, Anthropic's official CLI for Claude.
You are an interactive CLI tool that helps users with software engineering tasks. Use the instructions below and the tools available to you to assist the user.

IMPORTANT: Assist with defensive security tasks only. Refuse to create, modify, or improve code that may be used maliciously. Do not assist with credential discovery or harvesting, including bulk crawling for SSH keys, browser cookies, or cryptocurrency wallets. Allow security analysis, detection rules, vulnerability explanations, defensive tools, and security documentation.
IMPORTANT: You must NEVER generate or guess URLs for the user unless you are confident that the URLs are for helping the user with programming. You may use URLs provided by the user in their messages or local files.

### Tone and Style

- Be concise, direct, and to the point
- Answer concisely with fewer than 4 lines unless user asks for detail
- Minimize output tokens while maintaining helpfulness, quality, and accuracy
- Avoid unnecessary preamble or postamble
- Follow conventions when making changes to files

### Task Management

- Use TodoWrite tools frequently to track tasks and give user visibility into progress
- Mark todos as completed as soon as tasks are finished
- Break down larger complex tasks into smaller steps

## Project Specification

### Core Purpose

The `mcp-remote` project serves as a **bidirectional proxy** that bridges the gap between:

- **Local MCP Clients** (Claude Desktop, Cursor, Windsurf) that only support stdio transport
- **Remote MCP Servers** that use HTTP/SSE transport with OAuth authentication

### Key Features to Implement

#### 1. Transport Support

- **STDIO Transport**: For local client communication
- **HTTP Transport**: For remote server communication
- **SSE (Server-Sent Events) Transport**: For remote server communication
- **Transport Strategy System**:
  - `http-first` (default): Try HTTP first, fallback to SSE on 404
  - `sse-first`: Try SSE first, fallback to HTTP on 405
  - `http-only`: Only HTTP transport
  - `sse-only`: Only SSE transport

#### 2. OAuth Authentication System

- **Dynamic Client Registration**: Support for RFC 7591 OAuth dynamic client registration
- **Static Client Information**: Support for pre-registered OAuth clients
- **Token Management**:
  - Automatic token refresh
  - Persistent storage in `~/.mcp-auth` directory
  - Token sharing between multiple instances
- **Authorization Flow**:
  - Opens browser for user authorization
  - Runs local callback server to receive auth codes
  - Exchanges auth codes for access tokens

#### 3. Configuration & CLI Interface

- **Command-line Arguments**:
  - Server URL (required)
  - Callback port (optional, auto-select if not provided)
  - Custom headers (`--header` flag)
  - Host specification (`--host` flag)
  - HTTP allowance (`--allow-http` flag)
  - Debug logging (`--debug` flag)
  - Proxy support (`--enable-proxy` flag)
  - Tool filtering (`--ignore-tool` flag with wildcard support)
  - Auth timeout (`--auth-timeout` flag)
  - Transport strategy (`--transport` flag)
  - Static OAuth metadata (`--static-oauth-client-metadata` flag)
  - Static OAuth client info (`--static-oauth-client-info` flag)

#### 4. Security Features

- **HTTPS Enforcement**: Default to HTTPS, allow HTTP only with explicit flag for trusted networks
- **Certificate Handling**: Support for custom CA certificates via environment variables
- **Token Security**: Secure storage and handling of OAuth tokens
- **Request Filtering**: Ability to ignore/filter specific tools by pattern matching

#### 5. Advanced Features

- **Multi-instance Coordination**: Handle multiple proxy instances for the same server
- **Lazy Authentication**: Only authenticate when actually needed
- **Debug Logging**: Comprehensive logging system with timestamps
- **Proxy Support**: HTTP/HTTPS proxy support via environment variables
- **Error Handling**: Comprehensive error handling with helpful user messages

## Rust Implementation Plan

### Workspace Structure

```
mcp-remote-rs/
├── Cargo.toml (workspace)
├── crates/
│   ├── mcp-auth/           # OAuth authentication system (custom)
│   ├── mcp-proxy/          # Proxy logic and coordination using rmcp
│   ├── mcp-cli/            # CLI interface and argument parsing
│   └── mcp-remote/         # Main binary crate
├── examples/
├── tests/
└── docs/
```

**Note: Simplified structure using rmcp SDK**

- `mcp-core/` and `mcp-transport/` are replaced by the official `rmcp` crate
- `rmcp` provides: protocol types, STDIO/HTTP/SSE transports, client/server handlers

### Crate Responsibilities

#### `rmcp` (External Dependency)

- **Official MCP SDK**: Core MCP protocol types and serialization
- **Transports**: STDIO, HTTP, SSE transport implementations built-in
- **Client/Server**: `ClientHandler`, `ServerHandler`, `Service` traits
- **Error Handling**: `ErrorData`, `RmcpError` types
- **Features**: `client`, `server`, `macros` feature flags

#### `mcp-auth`

- OAuth client registration (dynamic and static)
- Token storage and management
- Authentication flow coordination
- Browser launching and callback server
- Token refresh logic

#### `mcp-proxy`

- Bidirectional message proxying
- Tool filtering and transformation
- Connection management
- Multi-instance coordination
- Lazy authentication integration

#### `mcp-cli`

- Command-line argument parsing
- Configuration loading
- Debug logging setup
- Signal handling

#### `mcp-remote`

- Main binary entry point
- Integration of all components
- Error handling and user-friendly messages

### Key Dependencies

**Primary Dependencies:**

- `rmcp = { version = "0.7.0", features = ["client", "server"] }` - Official MCP SDK
- `tokio` - Async runtime (already included in rmcp)
- `oauth2` - OAuth 2.0 client implementation (already in rmcp as optional)

**Additional Dependencies:**

- `clap` - CLI argument parsing
- `dirs` - Platform-specific directories
- `tracing` - Structured logging (already in rmcp)
- `anyhow` - Error handling
- `serde_json` - JSON handling (already in rmcp)

**Note**: `rmcp` already includes: `serde`, `reqwest`, `tokio-stream`, `sse-stream`, `uuid`, `futures`, `url`

### Implementation Phases

#### Phase 1: Core Infrastructure

1. Set up workspace structure with `rmcp` dependency
2. Configure `rmcp` features for client functionality
3. Set up CLI framework with clap
4. Create basic proxy structure using `rmcp::service::Service`

#### Phase 2: Remote Transports

1. Use `rmcp::transport` for STDIO/HTTP/SSE (built-in)
2. Implement transport strategy system on top of rmcp transports
3. Create connection management using `rmcp::service::ServiceExt`
4. Configure rmcp client for remote server communication

#### Phase 3: Authentication System

1. Implement OAuth client registration (rmcp includes `oauth2` crate)
2. Create token storage system
3. Build authentication flow with browser launching
4. Add token refresh mechanisms
5. Integrate with rmcp's built-in HTTP client capabilities

#### Phase 4: Proxy Logic

1. Implement bidirectional message proxying using `rmcp::service::Service`
2. Use `rmcp::model::CallToolRequestParam` for tool filtering
3. Create multi-instance coordination
4. Integrate lazy authentication with rmcp client/server handlers

#### Phase 5: Advanced Features

1. Use `rmcp::ErrorData` and `rmcp::RmcpError` for error handling
2. Leverage rmcp's built-in `tracing` integration
3. Add proxy support using rmcp's HTTP transport features
4. Create comprehensive testing suite with rmcp test utilities

#### Phase 6: Polish & Documentation

1. Add examples and documentation
2. Performance optimization
3. Security audit
4. Release preparation

### Testing Strategy

- **Unit Tests**: Each crate should have comprehensive unit tests
- **Integration Tests**: End-to-end testing of proxy functionality
- **Mock Servers**: Create mock MCP servers for testing
- **OAuth Mock**: Mock OAuth providers for authentication testing
- **Error Scenarios**: Test various failure modes and recovery

### Security Considerations

- Secure token storage using platform keyring where possible
- Input validation for all CLI arguments and configuration
- Safe handling of sensitive data in logs
- Certificate validation for HTTPS connections
- Secure random generation for OAuth state parameters

## Development Guidelines

### Code Style

- Follow Rust standard conventions
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Comprehensive error handling with context
- Clear documentation for public APIs

### Error Handling Strategy

- Use `anyhow` for application errors
- Create specific error types for each crate
- Provide helpful error messages for users
- Log errors appropriately based on severity

### Async Programming

- Use `tokio` as the async runtime
- Prefer `async`/`await` over manual future handling
- Use structured concurrency patterns
- Handle cancellation properly

### Configuration Management

- Support environment variables
- Use XDG Base Directory specification on Unix
- Platform-specific config directories
- Secure storage for sensitive data

This specification provides the foundation for implementing a robust, secure, and user-friendly Rust version of the mcp-remote project while maintaining compatibility with the existing ecosystem and improving upon the original implementation where possible.

## Feature Comparison with geelen/mcp-remote - TODO Implementation List

### ✅ Already Available via rmcp SDK

- [x] **Core MCP Protocol**: Types, serialization via `rmcp::model`
- [x] **STDIO Transport**: Via `rmcp::transport::TokioChildProcess`
- [x] **HTTP Transport**: Via `rmcp::transport` with `reqwest` backend
- [x] **SSE Transport**: Via `rmcp::transport` with `sse-stream`
- [x] **Client/Server Handlers**: `rmcp::ClientHandler`, `rmcp::ServerHandler`
- [x] **Service Layer**: `rmcp::service::Service`, `rmcp::service::ServiceExt`
- [x] **Error Handling**: `rmcp::ErrorData`, structured error types
- [x] **Async Support**: Built on `tokio`, `futures`
- [x] **JSON Handling**: Built-in `serde_json` integration

### ✅ Still Need to Implement (Custom Logic)

- [x] Transport Strategy System (http-first, sse-first, etc.)
- [x] Custom Headers support (`--header` flag)
- [x] Debug Logging integration (`--debug` flag)
- [x] HTTP/HTTPS Control (`--allow-http` flag)
- [x] Tool Filtering (`--ignore-tool` with wildcards)
- [x] CLI argument parsing and configuration

### 🚧 High Priority TODO Items (Missing OAuth Features)

- [ ] **OAuth 2.0 Client Implementation**
  - [ ] Dynamic Client Registration (RFC 7591)
  - [ ] Authorization Code Flow with PKCE
  - [ ] Browser launching for auth flow
  - [ ] Local callback server for auth codes
  - [ ] Token exchange implementation

- [ ] **Token Management System**
  - [ ] Automatic token refresh logic
  - [ ] Persistent token storage in `~/.mcp-auth` directory
  - [ ] Secure token storage using platform keyring
  - [ ] Token validation and expiry handling

- [ ] **Multi-instance Coordination**
  - [ ] Lock file mechanism for preventing conflicts
  - [ ] Shared token storage between instances
  - [ ] Instance coordination for auth flows
  - [ ] Proper cleanup on process termination

- [ ] **Static OAuth Configuration**
  - [ ] `--static-oauth-client-metadata` flag support
  - [ ] `--static-oauth-client-info` flag support
  - [ ] JSON and file-based configuration loading
  - [ ] Environment variable substitution in configs

### 🔧 Medium Priority TODO Items (Enhanced Features)

- [ ] **Advanced Certificate Handling**
  - [ ] Custom CA certificate support via environment variables
  - [ ] VPN certificate handling improvements
  - [ ] SSL/TLS configuration options

- [ ] **Enhanced Error Handling & Recovery**
  - [ ] Comprehensive error context and user-friendly messages
  - [ ] Automatic retry mechanisms for transient failures
  - [ ] Graceful degradation for partial failures
  - [ ] Better error reporting for OAuth failures

- [ ] **Lazy Authentication**
  - [ ] Only authenticate when first request requires it
  - [ ] Deferred auth initialization
  - [ ] Auth state caching and reuse

- [ ] **Advanced Logging & Debugging**
  - [ ] Structured logging with timestamps
  - [ ] Debug log files in `~/.mcp-auth/{server_hash}_debug.log`
  - [ ] Detailed OAuth flow logging
  - [ ] Connection state and health logging

### 🎯 Low Priority TODO Items (Nice-to-Have)

- [ ] **Enhanced CLI Experience**
  - [ ] Configuration file support
  - [ ] Interactive configuration wizard
  - [ ] Health check commands
  - [ ] Token status and refresh commands

- [ ] **Performance Optimizations**
  - [ ] Connection pooling for HTTP requests
  - [ ] Async message batching
  - [ ] Memory usage optimizations
  - [ ] Startup time improvements

- [ ] **Testing & Quality**
  - [ ] Comprehensive unit test suite
  - [ ] Integration tests with mock OAuth servers
  - [ ] Performance benchmarking
  - [ ] Security audit and penetration testing

### 📋 Current Competitive Analysis Summary

**Rust Implementation Advantages (Enhanced with rmcp):**

- ⚡ Performance: Much faster startup and lower memory usage
- 🔹 Single Binary: No Node.js dependency required
- 🛡️ Type Safety: Rust's compile-time guarantees + rmcp's typed MCP protocol
- 🎯 Official SDK: Using modelcontextprotocol/rust-sdk ensures compatibility
- 🚀 Simplified Development: Pre-built transports, handlers, and utilities
- 🔧 Feature Flags: Modular compilation with rmcp features

**Missing Features vs geelen/mcp-remote:**

- ❌ Full OAuth 2.0 implementation (HIGH PRIORITY)
- ❌ Browser-based authentication flows (HIGH PRIORITY)
- ❌ Token persistence and refresh (HIGH PRIORITY)
- ❌ Multi-instance coordination (MEDIUM PRIORITY)

### 🎯 Implementation Priority Order (Updated for rmcp)

1. **Phase 1**: Setup rmcp client service and basic STDIO proxy
2. **Phase 2**: OAuth 2.0 implementation using rmcp's oauth2 integration
3. **Phase 3**: Token management and persistence system
4. **Phase 4**: Transport strategy system on top of rmcp transports
5. **Phase 5**: Tool filtering using rmcp's CallToolRequestParam
6. **Phase 6**: Multi-instance coordination and advanced features

### 🛠️ rmcp Integration Strategy

- **Foundation**: Use `rmcp::service::Service` as the core proxy service
- **Transports**: Leverage rmcp's built-in STDIO, HTTP, and SSE transports
- **Protocol**: Use `rmcp::model::*` for all MCP message types
- **Error Handling**: Adopt `rmcp::ErrorData` error patterns
- **Testing**: Use rmcp's client/server test utilities

This TODO list ensures the Rust implementation will eventually match and exceed the feature set of geelen/mcp-remote while maintaining its performance and deployment advantages.

## Detailed rmcp SDK Integration Plan

Based on the existing project structure in `tokio-night-gnome/`, here's the comprehensive integration plan:

### Current Project Analysis

**Existing Structure:**

```
tokio-night-gnome/
├── crates/
│   ├── mcp-types/          # Custom MCP protocol types
│   ├── mcp-client/         # Custom HTTP/SSE client
│   ├── mcp-server/         # Custom STDIO server
│   ├── mcp-oauth/          # OAuth implementation
│   ├── mcp-proxy/          # Proxy coordination
│   └── mcp-remote/         # Main binary
```

**Integration Strategy: Hybrid Approach**

### Phase 1: Foundation Migration (Week 1-2)

#### 1.1 Replace `mcp-types` with rmcp Types

**Current `mcp-types/src/lib.rs`:**

- Custom JSON-RPC and MCP types
- Manual serde implementations

**Migration to rmcp:**

```rust
// OLD: Custom types in mcp-types/
pub struct JsonRpcRequest { ... }
pub struct InitializeRequest { ... }
pub struct CallToolRequest { ... }

// NEW: Use rmcp types
pub use rmcp::model::{
    Request, Response, Notification,
    InitializeRequest, InitializeResponse,
    CallToolRequestParam, CallToolResult,
    Tool, Content, ErrorData,
};
```

**Changes Required:**

- Update `mcp-types/Cargo.toml` to depend on `rmcp`
- Replace custom types with rmcp re-exports
- Update all dependent crates to use rmcp types
- Maintain backward compatibility with type aliases if needed

#### 1.2 Modernize `mcp-client` with rmcp Transports

**Current `mcp-client/src/transport/`:**

- Custom HTTP and SSE implementations
- Manual JSON-RPC handling

**Migration Strategy:**

```rust
// OLD: Custom transport trait
#[async_trait]
pub trait Transport {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse>;
}

// NEW: Wrap rmcp transports
use rmcp::transport::{Stdio, Process};
use rmcp::service::{Service, ServiceExt};

pub struct RmcpTransportWrapper {
    service: Service,
}

impl RmcpTransportWrapper {
    pub async fn new_http(url: &str) -> Result<Self> {
        // Use rmcp's HTTP transport capabilities
        let service = rmcp::service::serve_client(
            rmcp::transport::Http::new(url)?
        ).await?;
        Ok(Self { service })
    }

    pub async fn new_sse(url: &str) -> Result<Self> {
        // Use rmcp's SSE transport capabilities
        let service = rmcp::service::serve_client(
            rmcp::transport::Sse::new(url)?
        ).await?;
        Ok(Self { service })
    }
}
```

**Benefits:**

- Leverage rmcp's optimized transport implementations
- Better error handling with rmcp's error types
- Built-in connection management and retry logic
- SSE and HTTP implementations are battle-tested

#### 1.3 Enhance `mcp-server` with rmcp STDIO

**Current `mcp-server/src/transport/stdio.rs`:**

- Custom stdin/stdout handling
- Manual JSON-RPC parsing

**Migration Strategy:**

```rust
// OLD: Custom STDIO implementation
pub struct StdioTransport {
    stdin: tokio::io::Stdin,
    stdout: tokio::io::Stdout,
}

// NEW: Use rmcp's STDIO transport
use rmcp::transport::Stdio;
use rmcp::service::serve_server;

pub struct RmcpStdioServer {
    service: Service,
}

impl RmcpStdioServer {
    pub async fn new() -> Result<Self> {
        let service = serve_server(
            YourServerImplementation::new(),
            Stdio::new()
        ).await?;
        Ok(Self { service })
    }
}
```

### Phase 2: Proxy Integration (Week 3-4)

#### 2.1 Redesign `mcp-proxy` with rmcp Service Layer

**Current Architecture:**

```rust
// Custom proxy forwarding logic
impl Proxy {
    async fn forward_request(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // Manual request/response handling
    }
}
```

**New rmcp-based Architecture:**

```rust
use rmcp::service::{Service, ServiceExt, RoleClient};
use rmcp::model::{CallToolRequestParam, CallToolResult};

pub struct RmcpProxy {
    // Local STDIO service for IDE communication
    local_service: Service,

    // Remote HTTP/SSE service for server communication
    remote_service: Service,

    // Configuration and filtering
    config: ProxyConfig,
}

impl RmcpProxy {
    pub async fn new(
        remote_url: &str,
        transport_strategy: TransportStrategy,
        auth_config: Option<AuthConfig>,
    ) -> Result<Self> {
        // Create local STDIO service
        let local_service = serve_server(
            ProxyServerHandler::new(),
            rmcp::transport::Stdio::new()
        ).await?;

        // Create remote service with authentication
        let remote_service = Self::create_remote_service(
            remote_url,
            transport_strategy,
            auth_config
        ).await?;

        Ok(Self {
            local_service,
            remote_service,
            config: ProxyConfig::default(),
        })
    }

    pub async fn run(&self) -> Result<()> {
        // Use rmcp's built-in service coordination
        tokio::select! {
            result = self.handle_local_requests() => result?,
            result = self.handle_remote_responses() => result?,
        }
        Ok(())
    }

    async fn handle_tool_call(&self, params: CallToolRequestParam) -> Result<CallToolResult> {
        // Apply tool filtering
        if self.should_filter_tool(&params.name) {
            return Err(ErrorData::invalid_params("Tool filtered"));
        }

        // Forward to remote service
        self.remote_service.call_tool(params).await
    }
}
```

#### 2.2 Transport Strategy Implementation

```rust
pub enum TransportStrategy {
    HttpFirst,
    SseFirst,
    HttpOnly,
    SseOnly,
}

impl RmcpProxy {
    async fn create_remote_service(
        url: &str,
        strategy: TransportStrategy,
        auth_config: Option<AuthConfig>,
    ) -> Result<Service> {
        let base_service = match strategy {
            TransportStrategy::HttpFirst => {
                // Try HTTP first, fallback to SSE
                match Self::try_http_connection(url).await {
                    Ok(service) => service,
                    Err(_) => Self::try_sse_connection(url).await?,
                }
            },
            TransportStrategy::HttpOnly => {
                Self::try_http_connection(url).await?
            },
            // ... other strategies
        };

        // Apply authentication if configured
        if let Some(auth) = auth_config {
            Self::apply_authentication(base_service, auth).await
        } else {
            Ok(base_service)
        }
    }
}
```

### Phase 3: Authentication Integration (Week 5-6)

#### 3.1 Enhance `mcp-oauth` with rmcp OAuth Support

**Current Implementation:**

- Custom OAuth client registration
- Manual token management

**Enhanced with rmcp:**

```rust
use rmcp::transport::Http;
use oauth2::{basic::BasicClient, AuthUrl, TokenUrl, ClientId};

pub struct RmcpOAuthClient {
    oauth_client: BasicClient,
    token_storage: TokenStorage,
    http_client: rmcp::transport::Http,
}

impl RmcpOAuthClient {
    pub async fn authenticate(&self) -> Result<AuthenticatedService> {
        // Use rmcp's oauth2 integration
        let access_token = self.perform_oauth_flow().await?;

        // Create authenticated HTTP client
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", access_token.secret()).parse()?
        );

        // Use rmcp with authenticated headers
        let service = rmcp::service::serve_client(
            Http::new(&self.server_url)?.with_headers(headers)
        ).await?;

        Ok(AuthenticatedService { service, token: access_token })
    }
}
```

### Phase 4: CLI Integration (Week 7-8)

#### 4.1 Update `mcp-remote` Binary

**Current Structure:**

```rust
// main.rs with custom CLI parsing
fn main() -> Result<()> {
    let config = parse_args();
    let proxy = create_proxy(config).await?;
    proxy.run().await
}
```

**Enhanced with rmcp:**

```rust
use rmcp::{service::ServiceExt, ErrorData};

#[tokio::main]
async fn main() -> Result<()> {
    // Enhanced CLI with rmcp-specific options
    let config = Config::parse_args()?;

    // Initialize tracing for rmcp debugging
    setup_logging(&config)?;

    // Create rmcp-based proxy
    let proxy = RmcpProxy::new(
        &config.server_url,
        config.transport_strategy,
        config.auth_config,
    ).await.map_err(|e| {
        eprintln!("Failed to create proxy: {}", e);
        std::process::exit(1);
    })?;

    // Run with graceful shutdown
    tokio::select! {
        result = proxy.run() => {
            if let Err(e) = result {
                tracing::error!("Proxy error: {}", e);
                std::process::exit(1);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutting down gracefully...");
            proxy.shutdown().await?;
        }
    }

    Ok(())
}

fn setup_logging(config: &Config) -> Result<()> {
    let filter = if config.debug {
        "rmcp=debug,mcp_remote=debug"
    } else {
        "rmcp=info,mcp_remote=info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    Ok(())
}
```

### Phase 5: Workspace Dependency Updates

#### 5.1 Root `Cargo.toml` Updates

```toml
[workspace.dependencies]
# Replace custom implementations with rmcp
rmcp = { version = "0.7.0", features = ["client", "server", "oauth", "http", "sse"] }

# Keep existing dependencies that rmcp doesn't provide
clap = { version = "4.0", features = ["derive"] }
dirs = "5.0"
anyhow = "1.0"

# Remove duplicates that rmcp provides
# tokio = { version = "1.0", features = ["full"] }  # Provided by rmcp
# serde = { version = "1.0", features = ["derive"] }  # Provided by rmcp
# serde_json = "1.0"  # Provided by rmcp
# reqwest = { version = "0.11", features = ["json", "stream"] }  # Provided by rmcp
```

#### 5.2 Individual Crate Updates

**mcp-types/Cargo.toml:**

```toml
[dependencies]
rmcp.workspace = true
# Remove serde, serde_json - provided by rmcp
```

**mcp-client/Cargo.toml:**

```toml
[dependencies]
rmcp.workspace = true
anyhow.workspace = true
# Remove tokio, reqwest, etc. - provided by rmcp

mcp-types = { path = "../mcp-types" }
```

**mcp-proxy/Cargo.toml:**

```toml
[dependencies]
rmcp.workspace = true
anyhow.workspace = true

mcp-types = { path = "../mcp-types" }
mcp-oauth = { path = "../mcp-oauth" }
```

### Integration Benefits

1. **Reduced Maintenance**: Leverage official MCP SDK instead of custom implementations
2. **Better Compatibility**: Guaranteed compatibility with MCP specification updates
3. **Performance**: Optimized transport implementations from rmcp team
4. **Feature Completeness**: Built-in support for advanced MCP features
5. **Error Handling**: Comprehensive error types and handling patterns
6. **Testing**: Built-in test utilities and mock capabilities

### Migration Timeline

**Week 1-2**: Foundation (mcp-types, basic transport integration)
**Week 3-4**: Proxy redesign with rmcp service layer
**Week 5-6**: OAuth integration and authentication
**Week 7-8**: CLI updates and final integration
**Week 9**: Testing and documentation
**Week 10**: Performance optimization and release preparation

### Risk Mitigation

1. **Backward Compatibility**: Maintain type aliases during transition
2. **Feature Parity**: Ensure all existing features work with rmcp
3. **Testing**: Comprehensive test suite during migration
4. **Rollback Plan**: Keep custom implementations until rmcp integration is stable
5. **Documentation**: Update all documentation and examples

This integration plan transforms the existing custom implementation into a robust, maintainable solution built on the official MCP Rust SDK while preserving all current functionality and adding new capabilities.

## Testing with @modelcontextprotocol/inspector

### Testing Commands

**Basic Inspector Testing:**

```bash
# Test with inspector GUI
npx @modelcontextprotocol/inspector --config inspector.config.json

# Test with inspector CLI mode
npx @modelcontextprotocol/inspector --config inspector.config.json --cli
```

### Test Configuration (inspector.config.json)

The `inspector.config.json` file should be configured to test the mcp-remote implementation with rmcp:

```json
{
  "mcpServers": {
    "mcp-remote-test": {
      "command": "cargo",
      "args": [
        "run",
        "--bin",
        "mcp-remote",
        "--",
        "http://localhost:8080/mcp",
        "--debug",
        "--allow-http"
      ],
      "env": {
        "RUST_LOG": "rmcp=debug,mcp_remote=debug"
      }
    }
  }
}
```

**rmcp-specific Testing:**

- Use `RUST_LOG=rmcp=debug` for rmcp SDK debugging
- Test rmcp client/server communication patterns
- Validate rmcp transport layer functionality

### Testing Protocol

1. **Build and Test**: Always build before testing

   ```bash
   cargo build --release
   npx @modelcontextprotocol/inspector --config inspector.config.json --cli
   ```

2. **Debug Testing**: Use debug mode for detailed logs

   ```bash
   npx @modelcontextprotocol/inspector --config inspector.config.json --cli --debug
   ```

3. **Test Different Transport Strategies**:
   - HTTP-first: `--transport http-first`
   - SSE-first: `--transport sse-first`
   - HTTP-only: `--transport http-only`
   - SSE-only: `--transport sse-only`

4. **Test Custom Headers**:

   ```bash
   cargo run --bin mcp-remote -- http://localhost:8080/mcp --header "Authorization: Bearer token123" --debug
   ```

5. **Test Tool Filtering**:
   ```bash
   cargo run --bin mcp-remote -- http://localhost:8080/mcp --ignore-tool "dangerous_*" --debug
   ```

### Test Validation Checklist

When testing with inspector, verify:

- [ ] MCP protocol handshake completes successfully
- [ ] Tools are properly enumerated and accessible
- [ ] Resource discovery works correctly
- [ ] Error handling provides meaningful messages
- [ ] Debug logs show proper transport negotiation
- [ ] Custom headers are properly transmitted
- [ ] Tool filtering works as expected
- [ ] Connection cleanup happens on termination

### Automated Testing Integration

For CI/CD, use inspector CLI mode:

```bash
# In test scripts
cargo build --release
npx @modelcontextprotocol/inspector --config inspector.config.json --cli --test-mode --timeout 30
```

### Mock Server Testing

Create test configurations for different server scenarios:

```json
{
  "mcpServers": {
    "mock-oauth-server": {
      "command": "cargo",
      "args": [
        "run",
        "--bin",
        "mcp-remote",
        "--",
        "http://localhost:9090/oauth-mcp"
      ],
      "env": { "MCP_TEST_MODE": "oauth", "RUST_LOG": "rmcp=debug" }
    },
    "mock-header-auth": {
      "command": "cargo",
      "args": [
        "run",
        "--bin",
        "mcp-remote",
        "--",
        "http://localhost:9091/header-mcp",
        "--header",
        "X-API-Key: test123"
      ],
      "env": { "MCP_TEST_MODE": "header-auth", "RUST_LOG": "rmcp=debug" }
    }
  }
}
```

## rmcp SDK Integration Examples

### Basic Client Setup

```rust
use rmcp::{service::ServiceExt, transport::TokioChildProcess};
use tokio::process::Command;

// Create a client service using rmcp
let service = ().serve(TokioChildProcess::new(
    Command::new("your-mcp-server")
)?).await?;

// Initialize connection
let server_info = service.peer_info();
println!("Connected: {server_info:#?}");

// List available tools
let tools = service.list_tools(Default::default()).await?;
```

### Proxy Implementation Pattern

```rust
use rmcp::{
    model::{CallToolRequestParam, Content},
    service::{Service, ServiceExt},
    transport::Stdio,
    ErrorData as McpError,
};

// Local client (STDIO)
let local_service = ().serve(Stdio::new()).await?;

// Remote server (HTTP with auth)
let remote_service = create_authenticated_client(server_url).await?;

// Proxy tool calls
let tool_result = remote_service.call_tool(CallToolRequestParam {
    name: tool_name.into(),
    arguments: tool_args,
}).await?;

// Forward result to local client
local_service.send_tool_result(tool_result).await?;
```

### OAuth Integration with rmcp

```rust
use rmcp::transport::Stdio;
use oauth2::{AuthUrl, ClientId, TokenUrl};

// Use rmcp's built-in oauth2 dependency
let oauth_client = oauth2::basic::BasicClient::new(
    ClientId::new(client_id),
    Some(client_secret),
    AuthUrl::new(auth_url)?,
    Some(TokenUrl::new(token_url)?),
);

// Integrate with rmcp HTTP transport
let authenticated_service = create_rmcp_service_with_oauth(
    server_url,
    oauth_client,
).await?;
```

### Error Handling Best Practices

```rust
use rmcp::{ErrorData as McpError, RmcpError};

// Handle rmcp-specific errors
match result {
    Ok(response) => process_response(response),
    Err(RmcpError::Transport(e)) => {
        tracing::error!("Transport error: {}", e);
        // Implement reconnection logic
    },
    Err(RmcpError::Protocol(e)) => {
        tracing::error!("Protocol error: {}", e);
        // Handle MCP protocol violations
    },
    Err(e) => {
        tracing::error!("Unexpected error: {}", e);
        return Err(e);
    }
}
```

## rmcp SDK Best Practices

### Feature Flag Configuration

```toml
[dependencies]
rmcp = { version = "0.7.0", features = [
    "client",           # For client functionality
    "oauth",           # For OAuth support
    "http",            # For HTTP transport
    "sse",             # For SSE transport
    "process-wrap",    # For subprocess transport
] }
```

### Structured Logging Integration

```rust
use tracing::{debug, error, info, warn};

// rmcp uses tracing internally - configure it properly
tracing_subscriber::fmt()
    .with_env_filter("rmcp=debug,mcp_remote=info")
    .init();

// Use structured logging in your proxy
info!(
    server_url = %url,
    transport = "http",
    "Connecting to remote MCP server"
);
```

### Transport Strategy Implementation

```rust
use rmcp::transport::{Stdio, Http, Sse};

enum TransportStrategy {
    HttpFirst,
    SseFirst,
    HttpOnly,
    SseOnly,
}

async fn create_remote_service(
    url: &str,
    strategy: TransportStrategy,
) -> Result<Service, RmcpError> {
    match strategy {
        TransportStrategy::HttpFirst => {
            // Try HTTP first, fallback to SSE
            match Http::new(url).connect().await {
                Ok(service) => Ok(service),
                Err(_) => Sse::new(url).connect().await,
            }
        },
        TransportStrategy::HttpOnly => {
            Http::new(url).connect().await
        },
        // ... other strategies
    }
}
```

### Testing with rmcp

```rust
#[cfg(test)]
mod tests {
    use rmcp::service::ServiceExt;

    #[tokio::test]
    async fn test_proxy_functionality() {
        // Use rmcp's test utilities
        let mock_server = rmcp::testing::MockServer::new();
        mock_server.expect_list_tools()
            .returning(|| Ok(vec![]));

        let service = create_test_service(mock_server.url()).await?;
        let tools = service.list_tools(Default::default()).await?;

        assert!(tools.is_empty());
    }
}
```
