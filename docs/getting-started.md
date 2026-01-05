# Getting Started

This guide will walk you through setting up MCP Connect from scratch.

## Quick Connection (No Config Needed)

Connect to a remote MCP server instantly:

```bash
# Basic connection
npx @dreygur/mcp https://remote.mcp.server/sse

# With authentication
npx @dreygur/mcp https://api.example.com/mcp --auth-token "your_token"

# With custom headers
npx @dreygur/mcp https://api.example.com/mcp --headers "X-Api-Key:your_key"
```

This is perfect for testing or one-off connections. For managing multiple servers, continue below.

## Prerequisites

- MCP Connect installed (see [Installation Guide](installation.md))
- An IDE that supports MCP (Zed, VSCode with MCP extension, or Cursor)
- Access to at least one remote MCP server (or use the MCP Registry)

## Step 1: Initialize Your Project

Navigate to your project directory and initialize MCP Connect:

```bash
cd /path/to/your/project
mcp-connect init
```

This creates:
- `.mcp-connect.json` - Main configuration file
- `.env` - Environment variables template

## Step 2: Discover and Add Servers

### Option A: Add from MCP Registry

Search the official MCP Registry for available servers:

```bash
# Search for servers
mcp-connect registry search github

# View server details
mcp-connect registry show modelcontextprotocol/github-mcp-server

# Add the server
mcp-connect config add github modelcontextprotocol/github-mcp-server
```

### Option B: Interactive Search

Use the interactive search to find servers:

```bash
mcp-connect config add github --search
```

This will:
1. Search the registry for matching servers
2. Let you select from the results
3. Prompt for required configuration

### Option C: Add Custom Server

Add a server with a custom URL:

```bash
mcp-connect config add my-server \
  --url "https://my-mcp-server.com/mcp" \
  --auth-header "Authorization: Bearer ${MY_TOKEN}"
```

## Step 3: Configure Credentials

Edit the `.env` file to add your API tokens and credentials:

```bash
# .env
GITHUB_TOKEN=ghp_xxxxxxxxxxxxx
CONTEXT7_API_KEY=ctx7sk_xxxxxxxxxxxxx
MY_TOKEN=xxxxxxxxxxxxx
```

**Security Note**: Never commit `.env` files to version control. The `.env` file is already in `.gitignore`.

## Step 4: Verify Configuration

Check your configuration:

```bash
# List configured servers
mcp-connect config list

# Show details for a specific server
mcp-connect config show github

# Test server connectivity
mcp-connect config test github

# Test all servers
mcp-connect config test --all
```

## Step 5: Generate IDE Configuration

Generate the configuration file for your IDE:

### For Zed

```bash
mcp-connect generate --ide zed
```

This creates/updates `~/.config/zed/settings.json`.

### For VSCode

```bash
mcp-connect generate --ide vscode
```

This creates/updates `.vscode/settings.json` in your project.

### For Cursor

```bash
mcp-connect generate --ide cursor
```

This creates/updates `.vscode/settings.json` (Cursor uses the same directory structure as VSCode).

## Step 6: Start Using MCP Servers

### Automatic Start (Recommended)

Your IDE will automatically start MCP Connect when it loads. The server runs in the background and provides access to all configured servers.

### Manual Start

For testing or debugging, you can start the server manually:

```bash
mcp-connect serve
```

Use `--debug` for verbose logging:

```bash
mcp-connect serve --debug
```

## Step 7: Verify Everything Works

### In Your IDE

1. **Restart your IDE** (or reload the window)
2. **Check the MCP panel** - You should see `mcp-connect` listed
3. **Verify servers** - All configured servers should be accessible
4. **Test tools** - Try using tools from your servers (they'll be namespaced like `github/search_code`)

### Using the CLI

Test connectivity and list available tools:

```bash
# Test a specific server
mcp-connect config test github

# Validate configuration
mcp-connect config validate
```

## Understanding Namespace Routing

MCP Connect uses namespace prefixes to distinguish tools from different servers:

- `github/search_code` - Tool from GitHub server
- `github/create_issue` - Another tool from GitHub server
- `context7/search_docs` - Tool from Context7 server

This allows you to use multiple servers without naming conflicts.

## Common Workflows

### Adding Multiple Servers

```bash
# Add GitHub
mcp-connect config add github modelcontextprotocol/github-mcp-server

# Add Context7
mcp-connect config add context7 --search

# Add custom server
mcp-connect config add my-api --url "https://api.example.com/mcp" \
  --auth-header "Authorization: Bearer ${API_TOKEN}"
```

### Updating Server Configuration

Edit `.mcp-connect.json` directly or use the config commands:

```bash
# Remove a server
mcp-connect config remove old-server

# Add it back with new configuration
mcp-connect config add new-server --url "https://new-url.com/mcp"
```

### Managing Registry Sources

```bash
# List registry sources
mcp-connect registry registry list

# Add custom registry source
mcp-connect registry registry add my-registry \
  --url "https://my-registry.com" \
  --api-version "v1"

# Set default registry
mcp-connect registry registry set-default my-registry
```

## Next Steps

- **Configuration Reference**: See [Configuration Guide](configuration.md) for detailed configuration options
- **IDE Setup**: Check [IDE Setup Guides](ide-setup/) for IDE-specific instructions
- **Advanced Usage**: Explore [Advanced Guide](advanced.md) for power user features
- **Troubleshooting**: If you encounter issues, see [Troubleshooting Guide](troubleshooting.md)

## Quick Reference

```bash
# Initialize
mcp-connect init

# Search registry
mcp-connect registry search <query>

# Add server
mcp-connect config add <name> <registry-path>

# List servers
mcp-connect config list

# Generate IDE config
mcp-connect generate --ide <zed|vscode|cursor>

# Start server
mcp-connect serve

# Test server
mcp-connect config test <name>
```

