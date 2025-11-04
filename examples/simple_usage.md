# MCP Connect Usage Examples

Complete examples for using MCP Connect in various scenarios.

## Quick Start Examples

### 1. Basic Setup (Recommended)

The recommended workflow using centralized configuration:

```bash
# 1. Initialize project
mcp-connect init

# 2. Search and add servers from registry
mcp-connect registry search github
mcp-connect config add github modelcontextprotocol/github-mcp-server

# 3. Configure credentials
echo "GITHUB_TOKEN=your_token_here" >> .env

# 4. Generate IDE configuration
mcp-connect generate --ide vscode

# 5. Start server (usually automatic via IDE)
mcp-connect serve
```

### 2. Add Multiple Servers

```bash
# Add from registry
mcp-connect config add github modelcontextprotocol/github-mcp-server
mcp-connect config add context7 --search

# Add custom server
mcp-connect config add my-api \
  --url "https://api.example.com/mcp" \
  --auth-header "Authorization: Bearer ${API_TOKEN}"

# List all servers
mcp-connect config list

# Test all servers
mcp-connect config test --all
```

### 3. Custom Registry Sources

```bash
# Add custom registry
mcp-connect registry registry add company-registry \
  --url "https://mcp-registry.company.com" \
  --api-version "v1"

# Set as default
mcp-connect registry registry set-default company-registry

# Search in custom registry
mcp-connect registry search internal-server --source company-registry

# List all registries
mcp-connect registry registry list
```

## IDE Integration Examples

### Zed Editor

```bash
# Generate Zed configuration
mcp-connect generate --ide zed

# Or with custom output
mcp-connect generate --ide zed --output ~/custom/path/settings.json
```

**Generated configuration** (`~/.config/zed/settings.json`):
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

### VSCode

```bash
# Generate VSCode configuration
mcp-connect generate --ide vscode
```

**Generated configuration** (`.vscode/settings.json`):
```json
{
  "mcp.servers": {
    "mcp-connect": {
      "command": "/path/to/mcp-connect",
      "args": ["serve"]
    }
  }
}
```

### Cursor

```bash
# Generate Cursor configuration (uses same .vscode directory)
mcp-connect generate --ide cursor
```

## Configuration Examples

### Complete Configuration File

`.mcp-connect.json`:
```json
{
  "$schema": "https://static.modelcontextprotocol.io/schemas/2025-10-17/mcp-connect-config.schema.json",
  "version": "1.0",
  "envFile": ".env",
  "routing": {
    "method": "namespace-prefix",
    "separator": "/"
  },
  "registries": {
    "company-registry": {
      "baseUrl": "https://mcp-registry.company.com",
      "apiVersion": "v1"
    }
  },
  "defaultRegistry": "default",
  "servers": {
    "github": {
      "name": "io.github.modelcontextprotocol/github-mcp-server",
      "description": "GitHub repository management",
      "version": "latest",
      "remote": {
        "type": "streamable-http",
        "url": "https://api.githubcopilot.com/mcp",
        "headers": [
          {
            "key": "Authorization",
            "value": "Bearer ${GITHUB_TOKEN}"
          }
        ]
      },
      "timeout": 30,
      "retryAttempts": 3
    },
    "context7": {
      "name": "io.github.modelcontextprotocol/context7-mcp-server",
      "description": "Code documentation search",
      "version": "latest",
      "remote": {
        "type": "streamable-http",
        "url": "https://mcp.context7.com/mcp",
        "headers": [
          {
            "key": "Authorization",
            "value": "Bearer ${CONTEXT7_API_KEY}"
          }
        ]
      },
      "timeout": 30,
      "retryAttempts": 3
    }
  }
}
```

### Environment Variables

`.env`:
```bash
# GitHub
GITHUB_TOKEN=ghp_xxxxxxxxxxxxx

# Context7
CONTEXT7_API_KEY=ctx7sk_xxxxxxxxxxxxx

# Custom API
API_TOKEN=xxxxxxxxxxxxx
```

## Advanced Usage Examples

### Legacy Proxy Mode

For single-server scenarios or advanced use cases:

```bash
# Simple HTTP proxy
mcp-connect proxy \
  --endpoint "https://api.example.com/mcp" \
  --auth-token "your-token" \
  --debug

# With custom headers
mcp-connect proxy \
  --endpoint "https://api.example.com/mcp" \
  --headers "Authorization: Bearer ${TOKEN},X-Custom-Header: value" \
  --timeout 60 \
  --retry-attempts 5 \
  --debug
```

### Load Balancing

```bash
# Distribute requests across multiple endpoints
mcp-connect load-balance \
  --endpoints "https://api1.example.com/mcp,https://api2.example.com/mcp" \
  --transport "http" \
  --timeout 30 \
  --retry-attempts 3 \
  --debug
```

### Testing Connections

```bash
# Test specific server from config
mcp-connect config test github

# Test all configured servers
mcp-connect config test --all

# Test direct endpoint
mcp-connect test \
  --endpoint "https://api.example.com/mcp" \
  --transport "http" \
  --auth-token "your-token"
```

## Registry Examples

### Searching the Registry

```bash
# Search for servers
mcp-connect registry search github

# Search with remote-only filter
mcp-connect registry search filesystem --remote-only

# List all servers
mcp-connect registry list

# Show server details
mcp-connect registry show modelcontextprotocol/github-mcp-server
```

### Managing Custom Registries

```bash
# Add custom registry
mcp-connect registry registry add my-registry \
  --url "https://registry.example.com" \
  --api-version "v1"

# List all registries
mcp-connect registry registry list

# Set default registry
mcp-connect registry registry set-default my-registry

# Use specific registry for search
mcp-connect registry search query --source my-registry

# Remove registry
mcp-connect registry registry remove my-registry
```

## Integration Examples

### Claude Desktop Configuration

Using the multiplexing server (recommended):

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
        "--auth-token",
        "${GITHUB_TOKEN}"
      ],
      "env": {
        "GITHUB_TOKEN": "your-github-token"
      }
    }
  }
}
```

### Python Integration

```python
import subprocess
import json
import os

# Start the multiplexing server
proxy = subprocess.Popen(
    ["mcp-connect", "serve", "--debug"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    cwd="/path/to/project"  # Where .mcp-connect.json is located
)

# Send initialize request
init_request = {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {
            "name": "python-client",
            "version": "1.0.0"
        }
    }
}

proxy.stdin.write(json.dumps(init_request) + "\n")
proxy.stdin.flush()

# Read response
response_line = proxy.stdout.readline()
response = json.loads(response_line)
print(f"Server: {response['result']['serverInfo']['name']}")

# List tools (namespaced)
tools_request = {
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list"
}

proxy.stdin.write(json.dumps(tools_request) + "\n")
proxy.stdin.flush()

tools_response = json.loads(proxy.stdout.readline())
print(f"Available tools: {len(tools_response['result']['tools'])}")
```

### Node.js Integration

```javascript
const { spawn } = require('child_process');
const readline = require('readline');

// Start the multiplexing server
const proxy = spawn('mcp-connect', ['serve', '--debug'], {
  cwd: '/path/to/project',  // Where .mcp-connect.json is located
  stdio: ['pipe', 'pipe', 'pipe']
});

const rl = readline.createInterface({
  input: proxy.stdout,
  crlfDelay: Infinity
});

// Send initialize request
const initRequest = {
  jsonrpc: '2.0',
  id: 1,
  method: 'initialize',
  params: {
    protocolVersion: '2024-11-05',
    capabilities: {},
    clientInfo: {
      name: 'node-client',
      version: '1.0.0'
    }
  }
};

proxy.stdin.write(JSON.stringify(initRequest) + '\n');

// Read responses
rl.on('line', (line) => {
  const response = JSON.parse(line);
  console.log('Response:', response);
});
```

## Docker Examples

### Dockerfile

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

### Docker Compose

```yaml
version: '3.8'
services:
  mcp-connect:
    build: .
    volumes:
      - ./:/workspace
      - ./.env:/workspace/.env:ro
    command: serve
    stdin_open: true
    tty: true
```

### Running in Docker

```bash
# Build image
docker build -t mcp-connect .

# Run multiplexing server
docker run -it \
  -v $(pwd)/.mcp-connect.json:/workspace/.mcp-connect.json:ro \
  -v $(pwd)/.env:/workspace/.env:ro \
  mcp-connect serve

# Run with custom config
docker run -it \
  -v $(pwd)/config.json:/workspace/.mcp-connect.json:ro \
  mcp-connect serve
```

## Debugging Examples

### Enable Debug Mode

```bash
# Debug multiplexing server
mcp-connect serve --debug

# Debug with verbose logging
mcp-connect serve --log-level trace

# Debug specific server test
mcp-connect config test github --debug
```

### View Logs

```bash
# Save logs to file
mcp-connect serve --debug 2>&1 | tee mcp-connect.log

# Filter logs
mcp-connect serve --debug 2>&1 | grep "error\|warn"
```

## Common Workflows

### Setting Up a New Project

```bash
#!/bin/bash
# setup-mcp.sh

# Initialize
mcp-connect init

# Add servers
mcp-connect config add github modelcontextprotocol/github-mcp-server
mcp-connect config add context7 --search

# Generate IDE config
mcp-connect generate --ide vscode

# Test everything
mcp-connect config test --all
mcp-connect config validate

echo "Setup complete! Add your tokens to .env and restart your IDE."
```

### Updating Servers

```bash
# Remove old server
mcp-connect config remove old-server

# Add new server
mcp-connect config add new-server --url "https://new-server.com/mcp"

# Regenerate IDE config
mcp-connect generate --ide vscode

# Verify
mcp-connect config test --all
```

### Managing Multiple Projects

Each project can have its own `.mcp-connect.json`:

```bash
# Project A
cd project-a
mcp-connect init
mcp-connect config add github modelcontextprotocol/github-mcp-server
mcp-connect generate --ide vscode

# Project B
cd project-b
mcp-connect init
mcp-connect config add context7 --search
mcp-connect generate --ide vscode
```

## Troubleshooting Examples

### Validate Configuration

```bash
# Check configuration syntax
mcp-connect config validate

# Test server connectivity
mcp-connect config test github

# Test all servers
mcp-connect config test --all
```

### Check Registry Connection

```bash
# Test registry access
mcp-connect registry search test

# Test custom registry
mcp-connect registry registry add test-registry --url "https://registry.example.com"
mcp-connect registry search test --source test-registry
```

## Performance Tips

### Optimize Timeouts

For slow networks or servers:

```json
{
  "servers": {
    "slow-server": {
      "remote": { ... },
      "timeout": 120,
      "retryAttempts": 5
    }
  }
}
```

### Reduce Retry Attempts

For fast-fail scenarios:

```json
{
  "servers": {
    "fast-server": {
      "remote": { ... },
      "timeout": 10,
      "retryAttempts": 1
    }
  }
}
```

## Next Steps

- See [Configuration Reference](../docs/configuration.md) for complete configuration options
- Check [IDE Setup Guides](../docs/ide-setup/) for IDE-specific instructions
- Review [Advanced Usage](../docs/advanced.md) for power user features
- Read [Troubleshooting](../docs/troubleshooting.md) if you encounter issues
