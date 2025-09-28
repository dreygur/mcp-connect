# MCP Remote Proxy - Project Mindmap

```
                                    MCP Remote Proxy
                                          │
                    ┌─────────────────────┼─────────────────────┐
                    │                     │                     │
                 Purpose              Architecture           Features
                    │                     │                     │
        ┌───────────┼───────────┐        │           ┌─────────┼─────────┐
        │           │           │        │           │         │         │
   Bridge MCP    Translate    Remote      │      Transport   Auth     Reliability
   Local↔Remote  Protocols   Servers      │      Support    Support   Features
                                          │                             │
                        ┌─────────────────┼─────────────────┐          │
                        │                 │                 │          │
                    Workspace         Components        Dependencies    │
                        │                 │                 │          │
                 ┌──────┼──────┐         │          ┌──────┼──────┐   │
                 │      │      │         │          │      │      │   │
              crates/  examples/ Cargo    │       Tokio   rmcp   OAuth2 │
                 │              .toml     │                             │
        ┌────────┼────────┐              │                             │
        │        │        │              │                             │
    mcp-types mcp-server │               │                             │
              │          │               │                             │
    ┌─────────┼──────────┼───────────────┼─────────────────────────────┼───┐
    │         │          │               │                             │   │
mcp-client mcp-proxy mcp-remote         │                         Fallbacks │
    │         │          │               │                         Load-Bal  │
    │         │          │               │                         Retry     │
    │         │       CLI Tool           │                         Debug     │
    │         │          │               │                                   │
    │    ┌────┼────┐     │               │                                   │
    │    │    │    │     │               │                                   │
    │  Stdio Auth HTTP   │               │                                   │
    │  Proxy Proxy Stream │               │                                   │
    │         │          │               │                                   │
    │    ┌────┼────┐     │        ┌──────┼──────┐                           │
    │    │         │     │        │      │      │                           │
Transport OAuth    Message        │   Commands  │                           │
Support  Flow     Forward         │      │      │                           │
    │              │              │      │      │                           │
┌───┼───┐          │              │   ┌──┼──┐   │                           │
│   │   │          │              │   │  │  │   │                           │
HTTP STDIO TCP     │              │ proxy test load-balance                 │
│   │   │          │              │   │     │                               │
│   │   │      ┌───┼───┐          │   │  notification-demo                  │
│   │   │      │       │          │   │                                     │
│   │   │   Bidirectional         │   │                                     │
│   │   │   Message Flow          │ ┌─┼─┐                                   │
│   │   │      │       │          │ │ │ │                                   │
│   │   │   Client → Proxy        │ Auth Headers                            │
│   │   │   Proxy → Server        │ Token Config                            │
│   │   │                         │                                         │
│   │   └─── Connection ──────────┼─────────────────────────────────────────┘
│   │         Management          │
│   │              │              │
│   │        ┌─────┼─────┐        │
│   │        │     │     │        │
│   │     Timeouts Retry Error     │
│   │       │      Logic Handle    │
│   │       │        │      │     │
│   └───────┼────────┼──────┼─────┘
│           │        │      │
└───────────┼────────┼──────┘
            │        │
      Network Stack  │
         TLS/SSL     │
                     │
              ┌──────┼──────┐
              │      │      │
           Success  Retry  Fail
              │      │      │
              ↓      ↓      ↓
         Forward  Try Next Give Up
         Message  Transport + Log
```

## Key Components Breakdown

### 🏗️ **Architecture**

- **Workspace**: 5 specialized crates + examples
- **Design**: Async-first with Tokio runtime
- **Pattern**: Modular, trait-based architecture

### 📦 **Crates**

1. **mcp-types**: Shared types, traits, errors
2. **mcp-server**: Local STDIO MCP server
3. **mcp-client**: Multi-transport remote client
4. **mcp-proxy**: Message forwarding engine
5. **mcp-remote**: CLI application

### 🔌 **Transport Layer**

- **HTTP**: Streamable HTTP with MCP-Session-Id
- **STDIO**: Subprocess communication
- **TCP**: Direct socket connections
- **Fallbacks**: Automatic transport switching

### 🔐 **Authentication**

- **Bearer Tokens**: Simple token auth
- **API Keys**: Service-specific keys
- **OAuth 2.1**: Full OAuth flow implementation
- **Custom Headers**: Flexible auth methods

### 🚀 **Features**

- **Load Balancing**: Multi-server distribution
- **Retry Logic**: Configurable retry attempts
- **Debug Logging**: Detailed troubleshooting
- **Connection Management**: Automatic reconnection

### 🛠️ **CLI Commands**

- `proxy`: Main STDIO proxy mode
- `test`: Connection testing
- `load-balance`: Multi-server mode
- `notification-demo`: Testing notifications

### 🔄 **Message Flow**

```
Local MCP Client → mcp-server (STDIO) → mcp-proxy → mcp-client → Remote Server
                                      ↑               ↓
                               Message Forwarding  Response
```

### 🌐 **Use Cases**

- Connect Claude Desktop to remote MCP servers
- Bridge transport incompatibilities
- Add authentication to MCP connections
- Load balance across multiple servers
- Debug MCP communication issues
