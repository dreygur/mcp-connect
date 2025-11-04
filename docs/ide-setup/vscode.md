# VSCode Setup

This guide explains how to set up MCP Connect with Visual Studio Code.

## Prerequisites

- Visual Studio Code installed
- MCP extension installed (if required)
- MCP Connect installed and configured
- At least one server configured in `.mcp-connect.json`

## Quick Setup

### Step 1: Install MCP Extension

Install the MCP extension for VSCode (if not already installed):

1. Open VSCode
2. Go to Extensions (Ctrl+Shift+X / Cmd+Shift+X)
3. Search for "MCP" or "Model Context Protocol"
4. Install the official MCP extension

### Step 2: Generate Configuration

Generate the VSCode configuration file:

```bash
mcp-connect generate --ide vscode
```

This creates or updates `.vscode/settings.json` in your project directory.

### Step 3: Reload Window

Reload the VSCode window (Ctrl+R / Cmd+R) or restart VSCode for changes to take effect.

### Step 4: Verify

1. Open the MCP panel in VSCode
2. Check that `mcp-connect` appears in the list of MCP servers
3. Verify that all configured servers are accessible

## Configuration Details

The generated configuration adds the following to `.vscode/settings.json`:

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

### Custom Output Location

To specify a custom location for the settings file:

```bash
mcp-connect generate --ide vscode --output /path/to/settings.json
```

## Workspace vs User Settings

### Workspace Settings (Recommended)

The default configuration is written to `.vscode/settings.json` in your project directory. This is recommended because:
- Configuration is version-controlled with your project
- Each project can have different servers
- Team members share the same configuration

### User Settings

To configure globally for all projects, use:

```bash
mcp-connect generate --ide vscode --output ~/.config/Code/User/settings.json
```

Or manually add to User Settings (File > Preferences > Settings > Open User Settings JSON):

```json
{
  "mcp.servers": {
    "mcp-connect": {
      "command": "/absolute/path/to/mcp-connect",
      "args": ["serve"]
    }
  }
}
```

## Manual Configuration

If you prefer to configure manually, add the following to `.vscode/settings.json`:

```json
{
  "mcp.servers": {
    "mcp-connect": {
      "command": "/absolute/path/to/mcp-connect",
      "args": ["serve"]
    }
  }
}
```

**Important**:
- Use the absolute path to the `mcp-connect` binary
- Ensure the path uses forward slashes on Windows (e.g., `C:/path/to/mcp-connect.exe`)

## Troubleshooting

### MCP Connect Not Appearing

1. **Check extension**: Ensure MCP extension is installed and enabled
2. **Check binary path**: Verify the path in `settings.json` is correct
3. **Check permissions**: Ensure `mcp-connect` is executable
4. **View output**: Open Output panel and select "MCP" to see error messages

### Servers Not Loading

1. **Verify configuration**: Run `mcp-connect config validate` from terminal
2. **Test manually**: Run `mcp-connect serve --debug` to see errors
3. **Check environment variables**: Ensure `.env` file is in the project root
4. **Check MCP output**: View MCP extension output for errors

### Connection Errors

1. **Test connectivity**: Run `mcp-connect config test --all` from terminal
2. **Check credentials**: Verify tokens in `.env` are correct
3. **Network issues**: Ensure you can reach the remote servers
4. **Check logs**: Enable debug mode in VSCode MCP settings

## Advanced Configuration

### Custom Working Directory

If your `.mcp-connect.json` is in a different location:

```json
{
  "mcp.servers": {
    "mcp-connect": {
      "command": "/path/to/mcp-connect",
      "args": ["serve"],
      "cwd": "/path/to/project"
    }
  }
}
```

### Environment Variables

Set environment variables directly in VSCode config:

```json
{
  "mcp.servers": {
    "mcp-connect": {
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

### Debug Mode

Enable debug logging for troubleshooting:

```json
{
  "mcp.servers": {
    "mcp-connect": {
      "command": "/path/to/mcp-connect",
      "args": ["serve", "--debug"]
    }
  }
}
```

## Updating Configuration

When you add or remove servers, regenerate the configuration:

```bash
mcp-connect generate --ide vscode
```

The command safely merges with existing settings, preserving other VSCode configuration.

## Multiple Projects

Each project can have its own `.vscode/settings.json` with different server configurations. This allows you to:
- Use different servers per project
- Share configuration with team members via version control
- Maintain project-specific credentials

## Integration with Other Extensions

MCP Connect works alongside other VSCode extensions:
- **GitHub Copilot**: Both can run simultaneously
- **Other MCP servers**: MCP Connect can be used alongside direct MCP server configurations
- **Custom extensions**: Any extension that uses MCP will work with MCP Connect

## Next Steps

- See [Configuration Reference](../configuration.md) for server setup
- Check [Registry Management](../registry.md) for adding more servers
- Review [Troubleshooting](../troubleshooting.md) for common issues

