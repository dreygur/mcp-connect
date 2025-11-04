# MCP Connect

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

A powerful, production-ready proxy and multiplexing server for the Model Context Protocol (MCP). MCP Connect enables seamless integration between local MCP clients and remote HTTP servers, providing centralized configuration management, multi-server support, and comprehensive IDE integration.

## ✨ Features

- **🔌 Remote Server Support** - Connect to multiple remote MCP servers via HTTP/HTTPS
- **📦 Centralized Configuration** - Manage all servers in a single `.mcp-connect.json` file
- **🔍 Registry Integration** - Search and discover servers from the official MCP Registry
- **🌐 Custom Registry Sources** - Add and manage custom registry sources beyond the official one
- **🔀 Multiplexing** - Access multiple servers through a single connection with namespace routing
- **🔐 Secure Credentials** - Environment variable-based authentication with `.env` support
- **💻 IDE Integration** - Auto-generate configuration files for Zed, VSCode, and Cursor
- **📡 Protocol Compliance** - Full MCP 2024-11-05 specification support
- **🔄 Retry & Resilience** - Automatic retry logic and connection management
- **📊 Comprehensive Logging** - Debug and production logging modes

## 🚀 Quick Start

### Installation

```bash
# From source
git clone https://github.com/yourusername/mcp-connect.git
cd mcp-connect
cargo build --release

# Install globally
cargo install --path crates/mcp-connect

# Or use pre-built binaries (when available)
# Download from releases page
```

### Basic Usage

```bash
# 1. Initialize your project
mcp-connect init

# 2. Add servers from the registry
mcp-connect config add github modelcontextprotocol/github-mcp-server

# 3. Configure credentials in .env
echo "GITHUB_TOKEN=your_token_here" >> .env

# 4. Generate IDE configuration
mcp-connect generate --ide vscode

# 5. Start the server (usually automatic via IDE)
mcp-connect serve
```

## 📚 Documentation

Comprehensive documentation is available in the [`docs/`](docs/) directory:

- **[Installation Guide](docs/installation.md)** - Detailed installation instructions for all platforms
- **[Getting Started](docs/getting-started.md)** - Step-by-step tutorial for new users
- **[Configuration Reference](docs/configuration.md)** - Complete configuration file documentation
- **[IDE Setup Guides](docs/ide-setup/)** - IDE-specific setup instructions
  - [Zed](docs/ide-setup/zed.md)
  - [VSCode](docs/ide-setup/vscode.md)
  - [Cursor](docs/ide-setup/cursor.md)
- **[Registry Management](docs/registry.md)** - Working with the MCP Registry and custom sources
- **[Advanced Usage](docs/advanced.md)** - Power user features and customization
- **[Troubleshooting](docs/troubleshooting.md)** - Common issues and solutions
- **[API Reference](docs/api-reference.md)** - Complete API documentation

## 🏗️ Architecture

MCP Connect is built as a modular Rust workspace with the following components:

```
crates/
├── mcp-types/      # Common types, traits, and error definitions
├── mcp-server/     # Server-side MCP implementation (STDIO)
├── mcp-client/     # Client for remote HTTP servers
├── mcp-proxy/      # Message forwarding and routing logic
├── mcp-registry/   # MCP Registry API client
├── mcp-config/     # Configuration management and validation
└── mcp-connect/     # Command-line interface
```

For detailed architecture information, see [ARCHITECTURE.md](ARCHITECTURE.md).

## 📋 Requirements

- **Rust**: 1.75 or later
- **Cargo**: Latest stable version
- **OpenSSL**: 3.x (or build with static linking)

## 🎯 Use Cases

### Multi-Server Management
Manage multiple remote MCP servers from a single configuration file, avoiding the need to configure each server individually in your IDE.

### Custom Registry Sources
Add your own MCP registry sources for private or internal server catalogs.

### Development Workflow
Integrate MCP servers into your development environment with automatic configuration generation for popular IDEs.

### Production Deployment
Deploy MCP Connect as a service to provide centralized access to multiple MCP servers across your team.

## 🔧 Configuration Example

```json
{
  "$schema": "https://static.modelcontextprotocol.io/schemas/2025-10-17/mcp-connect-config.schema.json",
  "version": "1.0",
  "envFile": ".env",
  "routing": {
    "method": "namespace-prefix",
    "separator": "/"
  },
  "servers": {
    "github": {
      "name": "io.github.modelcontextprotocol/github-mcp-server",
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

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](docs/contributing.md) for details.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [RMCP](https://docs.rs/rmcp) - Rust SDK for Model Context Protocol
- [Tokio](https://tokio.rs/) - Asynchronous runtime for Rust
- [Clap](https://clap.rs/) - Command Line Argument Parser
- [Serde](https://serde.rs/) - Serialization framework

## 📞 Support

- **Documentation**: See the [`docs/`](docs/) directory
- **Issues**: [GitHub Issues](https://github.com/yourusername/mcp-connect/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/mcp-connect/discussions)

---

**Built with ❤️ in Rust 🦀**
