# Registry Management

MCP Connect supports both the official MCP Registry and custom registry sources. This guide explains how to work with registries.

## Official MCP Registry

The official MCP Registry is the default source for discovering MCP servers. It's automatically available and doesn't require configuration.

### Searching the Registry

```bash
# Search for servers
mcp-connect registry search github

# Search with remote-only filter
mcp-connect registry search github --remote-only

# Show all servers (including STDIO-only)
mcp-connect registry search github --show-all
```

### Listing Servers

```bash
# List all servers
mcp-connect registry list

# List only remote-compatible servers
mcp-connect registry list --remote-only
```

### Viewing Server Details

```bash
# Show server details
mcp-connect registry show modelcontextprotocol/github-mcp-server

# Show details from a specific registry
mcp-connect registry show modelcontextprotocol/github-mcp-server --source default
```

## Custom Registry Sources

You can add custom registry sources for private or internal server catalogs.

### Adding a Custom Registry

```bash
# Add a custom registry
mcp-connect registry registry add my-registry \
  --url "https://registry.example.com" \
  --api-version "v1"
```

**Options:**
- `--url`: Base URL of the registry API
- `--api-version`: API version path (optional, e.g., "v1", "v0.1")

### Listing Registry Sources

```bash
# List all configured registry sources
mcp-connect registry registry list
```

Output shows:
- Default registry (official MCP registry)
- All custom registries with their URLs and API versions
- Which registry is set as default (indicated with →)

### Setting Default Registry

```bash
# Set a custom registry as default
mcp-connect registry registry set-default my-registry

# Reset to official registry
mcp-connect registry registry set-default default
```

### Using a Specific Registry

All registry commands support the `--source` option:

```bash
# Search in a specific registry
mcp-connect registry search github --source my-registry

# Show server from specific registry
mcp-connect registry show server-name --source my-registry

# List servers from specific registry
mcp-connect registry list --source my-registry
```

### Removing a Registry Source

```bash
# Remove a custom registry
mcp-connect registry registry remove my-registry

# Force remove without confirmation
mcp-connect registry registry remove my-registry --force
```

**Note**: You cannot remove the default registry. Reset it first:

```bash
mcp-connect registry registry set-default default
mcp-connect registry registry remove my-registry
```

## Registry Configuration

Registry sources are stored in `.mcp-connect.json`:

```json
{
  "registries": {
    "my-registry": {
      "baseUrl": "https://registry.example.com",
      "apiVersion": "v1"
    },
    "company-registry": {
      "baseUrl": "https://mcp-registry.company.com",
      "apiVersion": "v0.1"
    }
  },
  "defaultRegistry": "my-registry"
}
```

### Registry Fields

- `baseUrl` (required): Base URL of the registry API
- `apiVersion` (optional): API version path

## Registry API Compatibility

Custom registries should implement the MCP Registry API format:

### List Servers Endpoint

```
GET /servers?limit={limit}&cursor={cursor}&search={query}
```

**Response:**
```json
{
  "servers": [...],
  "metadata": {
    "count": 10,
    "nextCursor": "..."
  }
}
```

### Get Server Endpoint

```
GET /servers/{name}/versions/{version}
```

**Response:**
```json
{
  "server": {
    "name": "...",
    "description": "...",
    "remotes": [...]
  },
  "_meta": {
    "io.modelcontextprotocol.registry/official": {
      "status": "active",
      "isLatest": true
    }
  }
}
```

## Use Cases

### Private Company Registry

```bash
# Add internal registry
mcp-connect registry registry add company \
  --url "https://mcp-registry.company.com" \
  --api-version "v1"

# Set as default
mcp-connect registry registry set-default company

# Now all commands use company registry by default
mcp-connect registry search internal-server
```

### Multiple Registries

```bash
# Add multiple registries
mcp-connect registry registry add public --url "https://public-registry.com"
mcp-connect registry registry add private --url "https://private-registry.com"

# Use specific registry for each command
mcp-connect registry search query --source public
mcp-connect registry search query --source private
```

### Registry Validation

When adding a registry, MCP Connect automatically tests the connection:

```bash
mcp-connect registry registry add my-registry --url "https://registry.example.com"
# Testing connection to registry...
# ✓ Registry connection successful
# ✓ Added registry 'my-registry'
```

If the connection test fails, you'll see a warning but can still add the registry (it might be temporarily unavailable).

## Best Practices

1. **Use Descriptive Names**: Use meaningful names for custom registries
2. **Test Connections**: Verify registry URLs before adding them
3. **Version Control**: Commit registry configurations to share with team
4. **Fallback to Default**: Keep official registry as fallback
5. **Document Internal APIs**: Document custom registry API formats for your team

## Troubleshooting

### Registry Connection Errors

**Issue**: Registry connection test fails

**Solutions**:
1. Verify the URL is correct and accessible
2. Check if the registry requires authentication
3. Ensure the API version path is correct
4. Check network connectivity

### Server Not Found

**Issue**: Server not found in custom registry

**Solutions**:
1. Verify the server name is correct
2. Check if the registry has the server listed
3. Try using the `--source` flag to specify the registry
4. Verify the registry API format matches expected format

### Default Registry Issues

**Issue**: Commands not using expected registry

**Solutions**:
1. Check current default: `mcp-connect registry registry list`
2. Set explicit default: `mcp-connect registry registry set-default <name>`
3. Use `--source` flag to override default

## Next Steps

- See [Configuration Reference](configuration.md) for registry configuration details
- Check [Advanced Usage](advanced.md) for power user features
- Review [Troubleshooting](troubleshooting.md) for common issues

