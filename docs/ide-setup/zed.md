# Zed IDE Setup

This guide explains how to set up MCP Connect with the Zed editor.

## Prerequisites

- Zed editor installed
- MCP Connect installed and configured
- At least one server configured in `.mcp-connect.json`

## Quick Setup

### Step 1: Generate Configuration

Generate the Zed configuration file:

```bash
mcp-connect generate --ide zed
```

This creates or updates `~/.config/zed/settings.json` with the MCP Connect server configuration.

### Step 2: Restart Zed

Restart Zed (or reload the window) for the changes to take effect.

### Step 3: Verify

1. Open Zed's context panel
2. Check that `mcp-connect` appears in the list of context servers
3. Verify that all configured servers are accessible

## Configuration Details

The generated configuration adds the following to your Zed settings:

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

### Custom Output Location

To specify a custom location for the settings file:

```bash
mcp-connect generate --ide zed --output /path/to/settings.json
```

## Manual Configuration

If you prefer to configure manually, add the following to `~/.config/zed/settings.json`:

```json
{
  "context_servers": {
    "mcp-connect": {
      "source": "custom",
      "command": "/absolute/path/to/mcp-connect",
      "args": ["serve"]
    }
  }
}
```

**Important**: Use the absolute path to the `mcp-connect` binary.

## Troubleshooting

### MCP Connect Not Appearing

1. **Check the binary path**: Ensure the path in `settings.json` is correct
2. **Verify permissions**: Make sure `mcp-connect` is executable
3. **Check logs**: Look for errors in Zed's developer console

### Servers Not Loading

1. **Verify configuration**: Run `mcp-connect config validate`
2. **Test manually**: Run `mcp-connect serve --debug` to see errors
3. **Check environment variables**: Ensure `.env` file is in the project root

### Connection Errors

1. **Test connectivity**: Run `mcp-connect config test --all`
2. **Check credentials**: Verify tokens in `.env` are correct
3. **Network issues**: Ensure you can reach the remote servers

## Advanced Configuration

### Custom Working Directory

If your `.mcp-connect.json` is in a different location:

```json
{
  "context_servers": {
    "mcp-connect": {
      "source": "custom",
      "command": "/path/to/mcp-connect",
      "args": ["serve"],
      "cwd": "/path/to/project"
    }
  }
}
```

### Environment Variables

Set environment variables directly in Zed config:

```json
{
  "context_servers": {
    "mcp-connect": {
      "source": "custom",
      "command": "/path/to/mcp-connect",
      "args": ["serve"],
      "env": {
        "GITHUB_TOKEN": "your-token-here",
        "CONTEXT7_API_KEY": "your-key-here"
      }
    }
  }
}
```

**Note**: Using `.env` file is recommended over hardcoding credentials.

## Updating Configuration

When you add or remove servers, regenerate the configuration:

```bash
mcp-connect generate --ide zed
```

The command safely merges with existing settings, preserving other configuration.

## Next Steps

- See [Configuration Reference](../configuration.md) for server setup
- Check [Registry Management](../registry.md) for adding more servers
- Review [Troubleshooting](../troubleshooting.md) for common issues

