# Examples

This directory contains examples and usage patterns for MCP Connect.

## Files

- **[simple_usage.md](simple_usage.md)** - Complete usage examples and common workflows
- **[minimal_server_test.rs](minimal_server_test.rs)** - Minimal example demonstrating core components
- **[config_example.json](config_example.json)** - Example configuration file
- **[env_example](env_example)** - Environment variables template

## Running Examples

### Rust Example

Run the minimal server test:

```bash
cargo run --example minimal_server_test -p mcp-connect
```

The example demonstrates:
- Configuration management
- Registry client usage
- MCP client creation
- Forwarding strategy
- STDIO proxy
- JSON-RPC message handling

### Configuration Example

Copy the example configuration:

```bash
cp examples/config_example.json .mcp-connect.json
# Edit .mcp-connect.json with your actual server URLs
```

### Environment Variables

Copy the environment template:

```bash
cp examples/env_example .env
# Edit .env with your actual tokens
```

## Quick Start Example

```bash
# 1. Initialize
mcp-connect init

# 2. Add servers
mcp-connect config add github modelcontextprotocol/github-mcp-server

# 3. Configure credentials
echo "GITHUB_TOKEN=your_token" >> .env

# 4. Generate IDE config
mcp-connect generate --ide vscode

# 5. Start server
mcp-connect serve
```

## Integration Examples

See [simple_usage.md](simple_usage.md) for:
- Python integration
- Node.js integration
- Docker deployment
- IDE configuration examples

## More Examples

For more detailed examples, see:
- [Usage Examples](simple_usage.md)
- [Configuration Reference](../docs/configuration.md)
- [Getting Started Guide](../docs/getting-started.md)

