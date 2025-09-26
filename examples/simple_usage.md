# MCP Remote - Simple Usage Examples

## Basic Usage

Connect to a remote MCP server via proxy:

```bash
# Connect to an HTTPS MCP server (recommended)
./target/debug/mcp-remote https://example.com/mcp

# Use specific transport strategy
./target/debug/mcp-remote --transport sse-only https://example.com/mcp

# Allow HTTP for local development (not recommended for production)
./target/debug/mcp-remote --allow-http http://localhost:3000/mcp

# Enable debug logging
./target/debug/mcp-remote --debug https://example.com/mcp
```

## Transport Strategies

- **http-first** (default): Try HTTP POST first, fallback to SSE
- **sse-first**: Try SSE first, fallback to HTTP POST
- **http-only**: Only HTTP POST requests
- **sse-only**: Only SSE connections

## How It Works

1. **Local MCP Server**: Provides STDIO interface for IDEs/LLMs
2. **Remote MCP Client**: Connects to remote servers via HTTP/SSE
3. **Proxy**: Forwards requests bidirectionally between local and remote

```
IDE/LLM <--STDIO--> Local MCP Server <--> Proxy <--> Remote MCP Client <--HTTP/SSE--> Remote MCP Server
```

## Integration with IDEs

Configure your IDE to use this proxy as an MCP server:

```json
{
  "mcpServers": {
    "remote-proxy": {
      "command": "/path/to/mcp-remote",
      "args": ["https://your-remote-server.com/mcp"]
    }
  }
}
```

## Security Considerations

- HTTPS is enforced by default
- Use `--allow-http` only for trusted local networks
- Remote servers should implement proper authentication
- Validate Origin headers for web-based servers
