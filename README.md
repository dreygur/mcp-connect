# MCP Connect

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![npm](https://img.shields.io/npm/v/@dreygur/mcp)](https://www.npmjs.com/package/@dreygur/mcp)
[![GitHub stars](https://img.shields.io/github/stars/dreygur/mcp-connect?style=social)](https://github.com/dreygur/mcp-connect/stargazers)

A production-ready proxy and multiplexing server for the Model Context Protocol (MCP). MCP Connect enables seamless integration between local MCP clients and remote HTTP servers with OAuth support.

## Works With

Connect your favorite AI agents to remote MCP servers:

| Agent | Description |
|-------|-------------|
| [Claude Desktop](https://claude.ai/download) | Anthropic's official desktop app |
| [Claude Code](https://docs.anthropic.com/en/docs/claude-code) | Anthropic's CLI coding assistant |
| [Cursor](https://cursor.sh/) | AI-powered code editor |
| [Windsurf](https://codeium.com/windsurf) | Codeium's AI IDE |
| [Zed](https://zed.dev/) | High-performance code editor |
| [Continue](https://continue.dev/) | Open-source AI assistant for VSCode/JetBrains |

## Features

- **Remote Server Support** - Connect to multiple remote MCP servers via HTTP/HTTPS
- **OAuth Authentication** - Automatic OAuth 2.0 flow with token caching
- **Centralized Configuration** - Manage all servers in a single `.mcp-connect.json` file
- **Registry Integration** - Search and discover servers from the official MCP Registry
- **Multiplexing** - Access multiple servers through a single connection with namespace routing
- **IDE Integration** - Auto-generate configuration for Zed, VSCode, and Cursor

## Installation

### npx (no install)

```bash
npx @dreygur/mcp https://remote.server/mcp
```

### npm (global install)

```bash
npm install -g @dreygur/mcp
```

### Shell script (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/dreygur/mcp-connect/main/scripts/install.sh | bash
```

### PowerShell (Windows)

```powershell
irm https://raw.githubusercontent.com/dreygur/mcp-connect/main/scripts/install.ps1 | iex
```

### Cargo

```bash
# From GitHub
cargo install --git https://github.com/dreygur/mcp-connect

# From crates.io (when published)
cargo install mcp-connect
```

### From source

```bash
git clone https://github.com/dreygur/mcp-connect.git
cd mcp-connect
cargo install --path crates/mcp-connect
```

### Pre-built binaries

Download from [releases page](https://github.com/dreygur/mcp-connect/releases) for:
- Linux (x86_64)
- macOS (x86_64, ARM64)
- Windows (x86_64)

## Quick Start

**One-off connection (no config needed):**
```bash
npx @dreygur/mcp https://remote.mcp.server/sse
```

**With auth token:**
```bash
npx @dreygur/mcp https://api.example.com/mcp --auth-token "your_token"
```

**Multi-server setup:**
```bash
# Initialize configuration
mcp-connect init

# Add a server from registry
mcp-connect config add github modelcontextprotocol/github-mcp-server

# Configure credentials
echo "GITHUB_TOKEN=your_token" >> .env

# Generate IDE configuration
mcp-connect generate --ide vscode

# Start the server
mcp-connect serve
```

## Documentation

Full documentation is available at [dreygur.js.org/mcp-connect](https://dreygur.js.org/mcp-connect/)

- [Installation Guide](docs/installation.md)
- [Getting Started](docs/getting-started.md)
- [Configuration Reference](docs/configuration.md)
- [IDE Setup](docs/ide-setup/) - Zed, VSCode, Cursor
- [Registry Management](docs/registry.md)
- [Troubleshooting](docs/troubleshooting.md)

## Architecture

```
crates/
├── mcp-types/      # Common types and traits
├── mcp-server/     # Server-side MCP (STDIO)
├── mcp-client/     # Client for remote HTTP servers
├── mcp-proxy/      # Message forwarding and routing
├── mcp-registry/   # MCP Registry API client
├── mcp-config/     # Configuration management
└── mcp-connect/    # CLI
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for details.

## Configuration Example

```json
{
  "version": "1.0",
  "envFile": ".env",
  "routing": {
    "method": "namespace-prefix",
    "separator": "/"
  },
  "servers": {
    "github": {
      "name": "github-mcp-server",
      "remote": {
        "type": "streamable-http",
        "url": "https://api.githubcopilot.com/mcp",
        "headers": [
          {
            "key": "Authorization",
            "value": "Bearer ${GITHUB_TOKEN}"
          }
        ]
      }
    }
  }
}
```

## Requirements

- Node.js 14+ (for npm installation)
- Rust 1.75+ (for building from source)

## Contributing

See [Contributing Guide](docs/contributing.md) for details.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Support

If you find MCP Connect useful, please consider giving it a star on GitHub! It helps others discover the project.

[![Star on GitHub](https://img.shields.io/github/stars/dreygur/mcp-connect?style=social)](https://github.com/dreygur/mcp-connect)

## Acknowledgments

- [RMCP](https://docs.rs/rmcp) - Rust SDK for Model Context Protocol
- [Tokio](https://tokio.rs/) - Asynchronous runtime for Rust
