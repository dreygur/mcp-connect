# Configuration Reference

Complete reference for MCP Connect configuration files and options.

## Configuration File Structure

MCP Connect uses `.mcp-connect.json` as its main configuration file. This file defines all servers, routing, and registry sources.

### Schema

```json
{
  "$schema": "https://static.modelcontextprotocol.io/schemas/2025-10-17/mcp-connect-config.schema.json",
  "version": "1.0",
  "envFile": ".env",
  "servers": {},
  "routing": {},
  "registries": {},
  "defaultRegistry": "default"
}
```

## Top-Level Fields

### `$schema` (optional)

JSON Schema URL for validation. Recommended for IDE autocomplete support.

```json
{
  "$schema": "https://static.modelcontextprotocol.io/schemas/2025-10-17/mcp-connect-config.schema.json"
}
```

### `version` (required)

Configuration file format version. Currently `"1.0"`.

```json
{
  "version": "1.0"
}
```

### `envFile` (optional)

Path to environment variables file. Defaults to `.env` in the same directory.

```json
{
  "envFile": ".env"
}
```

### `servers` (required)

Map of server names to server configurations. See [Server Configuration](#server-configuration).

```json
{
  "servers": {
    "github": { ... },
    "context7": { ... }
  }
}
```

### `routing` (optional)

Routing configuration for multiplexing multiple servers. See [Routing Configuration](#routing-configuration).

```json
{
  "routing": {
    "method": "namespace-prefix",
    "separator": "/"
  }
}
```

### `registries` (optional)

Custom registry sources. See [Registry Configuration](#registry-configuration).

```json
{
  "registries": {
    "my-registry": {
      "baseUrl": "https://registry.example.com",
      "apiVersion": "v1"
    }
  }
}
```

### `defaultRegistry` (optional)

Default registry source to use. Use `"default"` for the official MCP registry.

```json
{
  "defaultRegistry": "my-registry"
}
```

## Server Configuration

Each server entry in the `servers` map has the following structure:

```json
{
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
}
```

### Server Fields

#### `name` (optional)

Registry name of the server. Used for identification and display.

```json
{
  "name": "io.github.modelcontextprotocol/github-mcp-server"
}
```

#### `description` (optional)

Human-readable description of the server.

```json
{
  "description": "GitHub repository management"
}
```

#### `version` (optional)

Version from registry. Typically `"latest"`.

```json
{
  "version": "latest"
}
```

#### `remote` (required)

Remote transport configuration. See [Remote Configuration](#remote-configuration).

#### `timeout` (optional)

Connection timeout in seconds. Default: `30`.

```json
{
  "timeout": 30
}
```

#### `retryAttempts` (optional)

Number of retry attempts on failure. Default: `3`.

```json
{
  "retryAttempts": 3
}
```

## Remote Configuration

Remote transport configuration defines how to connect to the remote server.

```json
{
  "remote": {
    "type": "streamable-http",
    "url": "https://api.example.com/mcp",
    "headers": [
      {
        "key": "Authorization",
        "value": "Bearer ${TOKEN}"
      }
    ]
  }
}
```

### Remote Fields

#### `type` (required)

Transport type. Supported values:
- `"streamable-http"` - Streamable HTTP transport (recommended)
- `"sse"` - Server-Sent Events transport

```json
{
  "type": "streamable-http"
}
```

#### `url` (required)

Remote server endpoint URL.

```json
{
  "url": "https://api.example.com/mcp"
}
```

#### `headers` (optional)

Array of HTTP headers to include with requests.

```json
{
  "headers": [
    {
      "key": "Authorization",
      "value": "Bearer ${GITHUB_TOKEN}"
    },
    {
      "key": "X-Custom-Header",
      "value": "custom-value"
    }
  ]
}
```

**Header Fields:**
- `key` (required): Header name
- `value` (required): Header value (supports environment variable substitution)

## Routing Configuration

Routing configuration controls how tools and resources from multiple servers are namespaced.

```json
{
  "routing": {
    "method": "namespace-prefix",
    "separator": "/"
  }
}
```

### Routing Fields

#### `method` (required)

Routing method. Currently only `"namespace-prefix"` is supported.

```json
{
  "method": "namespace-prefix"
}
```

#### `separator` (required)

Separator character used in namespace prefixes. Default: `"/"`.

```json
{
  "separator": "/"
}
```

**Example**: With separator `"/"`, tools are prefixed as `github/search_code`.

## Registry Configuration

Custom registry sources can be configured for discovering servers beyond the official MCP registry.

```json
{
  "registries": {
    "my-registry": {
      "baseUrl": "https://registry.example.com",
      "apiVersion": "v1"
    }
  }
}
```

### Registry Fields

#### `baseUrl` (required)

Base URL of the registry API.

```json
{
  "baseUrl": "https://registry.example.com"
}
```

#### `apiVersion` (optional)

API version path. If not specified, uses the default version.

```json
{
  "apiVersion": "v1"
}
```

## Environment Variables

Environment variables are loaded from the file specified in `envFile` (default: `.env`). Variables are substituted in configuration using `${VAR_NAME}` syntax.

### Example `.env` file:

```bash
# GitHub
GITHUB_TOKEN=ghp_xxxxxxxxxxxxx

# Context7
CONTEXT7_API_KEY=ctx7sk_xxxxxxxxxxxxx

# Custom API
MY_API_TOKEN=xxxxxxxxxxxxx
```

### Usage in Configuration:

```json
{
  "headers": [
    {
      "key": "Authorization",
      "value": "Bearer ${GITHUB_TOKEN}"
    }
  ]
}
```

## Complete Example

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

## Validation

Validate your configuration file:

```bash
mcp-connect config validate
```

This checks:
- JSON syntax
- Required fields
- URL formats
- Environment variable references

## Configuration Management Commands

```bash
# List all configured servers
mcp-connect config list

# Show details for a server
mcp-connect config show <name>

# Test server connectivity
mcp-connect config test <name>

# Remove a server
mcp-connect config remove <name>

# Validate configuration
mcp-connect config validate
```

## Best Practices

1. **Use Environment Variables**: Never hardcode credentials in configuration files
2. **Version Control**: Commit `.mcp-connect.json` but **never** commit `.env`
3. **Schema Validation**: Include the `$schema` field for IDE autocomplete
4. **Descriptive Names**: Use meaningful server names that reflect their purpose
5. **Consistent Separators**: Use the same separator across all servers for consistency
6. **Test Connections**: Use `config test` after adding new servers

## Troubleshooting

See the [Troubleshooting Guide](troubleshooting.md) for common configuration issues.

