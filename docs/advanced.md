# Advanced Usage

This guide covers advanced features and power user configurations for MCP Connect.

## Custom Transport Configuration

### HTTP Transport Options

Configure advanced HTTP transport settings:

```json
{
  "servers": {
    "my-server": {
      "remote": {
        "type": "streamable-http",
        "url": "https://api.example.com/mcp",
        "headers": [
          {
            "key": "Authorization",
            "value": "Bearer ${TOKEN}"
          },
          {
            "key": "X-Custom-Header",
            "value": "custom-value"
          }
        ]
      },
      "timeout": 60,
      "retryAttempts": 5
    }
  }
}
```

### Retry Configuration

Customize retry behavior per server:

```json
{
  "servers": {
    "unreliable-server": {
      "remote": { ... },
      "timeout": 30,
      "retryAttempts": 10
    }
  }
}
```

## Namespace Routing

### Custom Separators

Change the namespace separator:

```json
{
  "routing": {
    "method": "namespace-prefix",
    "separator": "::"
  }
}
```

This results in tools like `github::search_code` instead of `github/search_code`.

### Multiple Separators

For complex routing, you can use different separators, but each server must use the same separator configured in routing.

## Environment Variable Management

### Multiple Environment Files

Use different `.env` files for different environments:

```json
{
  "envFile": ".env.production"
}
```

Or use environment-specific files:
- `.env.development`
- `.env.production`
- `.env.local`

### Dynamic Variable Substitution

Environment variables are substituted in:
- Server URLs
- Header values
- Any configuration string

```json
{
  "remote": {
    "url": "https://${API_HOST}/mcp",
    "headers": [
      {
        "key": "Authorization",
        "value": "Bearer ${${ENV_PREFIX}_TOKEN}"
      }
    ]
  }
}
```

## Load Balancing

Use the legacy load balancing mode for distributing requests across multiple endpoints:

```bash
mcp-connect load-balance \
  --endpoints "https://api1.example.com/mcp,https://api2.example.com/mcp" \
  --transport http \
  --timeout 30
```

## Debugging

### Enable Debug Mode

Start the server with debug logging:

```bash
mcp-connect serve --debug
```

### Verbose Logging

Set log level for detailed output:

```bash
mcp-connect serve --log-level trace
```

Available levels: `trace`, `debug`, `info`, `warn`, `error`

### Debug Individual Servers

Test a specific server with debug output:

```bash
mcp-connect config test github --debug
```

## Performance Optimization

### Connection Pooling

MCP Connect automatically manages connections. For high-throughput scenarios:

1. **Increase Timeout**: For slower networks
2. **Adjust Retries**: For unreliable connections
3. **Monitor Logs**: Use debug mode to identify bottlenecks

### Batch Operations

When adding multiple servers, use batch scripts:

```bash
#!/bin/bash
mcp-connect config add github modelcontextprotocol/github-mcp-server
mcp-connect config add context7 --search
mcp-connect config add filesystem modelcontextprotocol/server-filesystem
```

## Custom Registry Implementation

### Building a Compatible Registry

Your registry should implement:

1. **List Endpoint**: `GET /servers?limit={limit}&cursor={cursor}`
2. **Search Endpoint**: `GET /servers?search={query}`
3. **Get Server Endpoint**: `GET /servers/{name}/versions/{version}`

### Registry Authentication

If your registry requires authentication, configure it in the registry URL or headers (if supported by your registry implementation).

## Scripting and Automation

### Configuration Generation

Automate configuration generation:

```bash
#!/bin/bash
# Generate configs for all IDEs
mcp-connect generate --ide zed
mcp-connect generate --ide vscode
mcp-connect generate --ide cursor
```

### CI/CD Integration

Use in CI/CD pipelines:

```yaml
# GitHub Actions example
- name: Test MCP Connect
  run: |
    mcp-connect config validate
    mcp-connect config test --all
```

### Docker Integration

Run MCP Connect in Docker:

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/mcp-connect /usr/local/bin/
WORKDIR /workspace
ENTRYPOINT ["mcp-connect"]
```

## Security Best Practices

### Credential Management

1. **Never commit `.env` files**: Already in `.gitignore`
2. **Use environment variables**: Avoid hardcoding credentials
3. **Rotate tokens regularly**: Update tokens in `.env` periodically
4. **Use secrets management**: For production, use proper secrets management

### Network Security

1. **Use HTTPS**: Always use HTTPS for remote servers
2. **Verify certificates**: MCP Connect validates SSL certificates
3. **Network isolation**: Use VPNs or private networks for internal servers

### Configuration Security

1. **Restrict file permissions**: `chmod 600 .env`
2. **Review configurations**: Regularly audit server configurations
3. **Monitor access**: Log server access and usage

## Custom Integrations

### API Integration

MCP Connect can be integrated into custom applications:

```rust
use mcp_config::ConfigManager;
use mcp_connect::commands::serve;

// Load configuration
let mut manager = ConfigManager::new()?;
let config = manager.load()?;

// Use configuration programmatically
for (name, server) in config.servers {
    println!("Server: {} - {}", name, server.description.unwrap_or_default());
}
```

### Webhook Integration

Set up webhooks to automatically update configurations:

```bash
# Webhook endpoint that updates config
curl -X POST https://your-webhook.com/update-config \
  -H "Content-Type: application/json" \
  -d @updated-config.json
```

## Monitoring and Observability

### Logging

Enable structured logging:

```bash
mcp-connect serve --log-level info > mcp-connect.log 2>&1
```

### Metrics

Monitor server health:

```bash
# Check server status
mcp-connect config test --all

# Validate configuration
mcp-connect config validate
```

## Troubleshooting Advanced Issues

### Connection Timeouts

Increase timeout for slow networks:

```json
{
  "servers": {
    "slow-server": {
      "timeout": 120
    }
  }
}
```

### Memory Management

For many servers, monitor memory usage:

```bash
# Check process memory
ps aux | grep mcp-connect
```

### Performance Profiling

Use Rust profiling tools:

```bash
# Install cargo-flamegraph
cargo install flamegraph

# Profile the server
cargo flamegraph --bin mcp-connect -- serve
```

## Next Steps

- See [Configuration Reference](configuration.md) for complete configuration options
- Check [Troubleshooting](troubleshooting.md) for advanced debugging
- Review [API Reference](api-reference.md) for programmatic usage

