# Troubleshooting Guide

Common issues and solutions for MCP Connect.

## Installation Issues

### OpenSSL Errors

**Error**: `libssl.so.3: cannot open shared object file`

**Solution**:
```bash
# Ubuntu/Debian
sudo apt install libssl3 libssl-dev

# Fedora/RHEL
sudo dnf install openssl-devel

# Or build with static OpenSSL
OPENSSL_STATIC=1 cargo build --release
```

### Rust Version Errors

**Error**: `error: edition 2021 is unstable`

**Solution**: Update Rust
```bash
rustup update stable
```

### Permission Errors

**Error**: `Permission denied` when running `mcp-connect`

**Solution**:
```bash
chmod +x target/release/mcp-connect
export PATH="$HOME/.cargo/bin:$PATH"
```

## Configuration Issues

### Configuration File Not Found

**Error**: `No .mcp-connect.json found`

**Solution**:
```bash
# Initialize configuration
mcp-connect init

# Or specify custom path
mcp-connect --config /path/to/config.json serve
```

### Invalid JSON

**Error**: `Failed to parse config file`

**Solution**:
1. Validate JSON syntax: `cat .mcp-connect.json | jq .`
2. Check for trailing commas
3. Verify all quotes are properly escaped

### Missing Environment Variables

**Error**: `Environment variable not found: ${VAR_NAME}`

**Solution**:
1. Check `.env` file exists in project root
2. Verify variable name matches exactly (case-sensitive)
3. Ensure no extra spaces: `VAR=value` not `VAR = value`
4. Restart the server after updating `.env`

## Connection Issues

### Connection Timeout

**Error**: `Connection error: Timeout`

**Solutions**:
1. **Increase timeout**:
   ```json
   {
     "servers": {
       "server": {
         "timeout": 60
       }
     }
   }
   ```

2. **Check network connectivity**:
   ```bash
   curl -I https://your-server.com/mcp
   ```

3. **Verify server is reachable**:
   ```bash
   mcp-connect config test server-name
   ```

### Authentication Errors

**Error**: `Authentication error: 401 Unauthorized`

**Solutions**:
1. **Verify token is correct**:
   ```bash
   # Check .env file
   cat .env | grep TOKEN
   ```

2. **Test token manually**:
   ```bash
   curl -H "Authorization: Bearer $GITHUB_TOKEN" https://api.github.com/user
   ```

3. **Check token format**: Ensure no extra spaces or quotes
4. **Verify token hasn't expired**: Regenerate if needed

### SSL Certificate Errors

**Error**: `SSL certificate verification failed`

**Solutions**:
1. **Check certificate validity**: Verify server certificate is valid
2. **Update CA certificates**:
   ```bash
   # Ubuntu/Debian
   sudo apt update && sudo apt install ca-certificates
   ```
3. **For development**: Use proper certificates (not self-signed in production)

## Server Issues

### Server Not Starting

**Error**: Server fails to start or crashes

**Solutions**:
1. **Check logs**:
   ```bash
   mcp-connect serve --debug
   ```

2. **Validate configuration**:
   ```bash
   mcp-connect config validate
   ```

3. **Test individual servers**:
   ```bash
   mcp-connect config test --all
   ```

### No Servers Configured

**Error**: `No servers configured in .mcp-connect.json`

**Solution**:
```bash
# Add a server
mcp-connect config add github modelcontextprotocol/github-mcp-server

# Verify
mcp-connect config list
```

### Server Not Appearing in IDE

**Error**: MCP Connect doesn't appear in IDE's MCP panel

**Solutions**:
1. **Regenerate IDE config**:
   ```bash
   mcp-connect generate --ide vscode
   ```

2. **Check binary path**: Ensure path in IDE config is correct
3. **Restart IDE**: Fully restart the IDE (not just reload)
4. **Check IDE logs**: Look for errors in IDE's developer console

## Registry Issues

### Registry Connection Failed

**Error**: `Failed to connect to registry`

**Solutions**:
1. **Check internet connection**: Ensure you can reach the registry
2. **Verify registry URL**: Check if the URL is correct
3. **Test manually**:
   ```bash
   curl https://registry.modelcontextprotocol.io/v0.1/servers?limit=1
   ```

### Server Not Found in Registry

**Error**: `Server 'name' not found`

**Solutions**:
1. **Verify server name**: Check spelling and format
2. **Search registry**:
   ```bash
   mcp-connect registry search server-name
   ```
3. **Check registry source**: Use `--source` flag if using custom registry

### Custom Registry Not Working

**Error**: Custom registry fails to connect

**Solutions**:
1. **Verify registry URL**: Ensure URL is accessible
2. **Check API version**: Verify API version path is correct
3. **Test registry API**: Verify it implements the expected API format
4. **Check authentication**: Some registries may require authentication

## IDE Integration Issues

### Zed Integration

**Issues**:
- Server not appearing in context panel
- Tools not available

**Solutions**:
1. **Check Zed settings**: Verify `~/.config/zed/settings.json` has correct config
2. **Check binary path**: Use absolute path in configuration
3. **Restart Zed**: Fully quit and restart Zed
4. **Check Zed logs**: Look for errors in Zed's developer console

### VSCode/Cursor Integration

**Issues**:
- MCP extension not recognizing server
- Tools not appearing

**Solutions**:
1. **Install MCP extension**: Ensure MCP extension is installed
2. **Check workspace settings**: Verify `.vscode/settings.json` exists
3. **Reload window**: Use Ctrl+R / Cmd+R to reload
4. **Check MCP output**: View MCP extension output panel for errors
5. **Verify binary path**: Check path uses correct format (forward slashes on Windows)

## Performance Issues

### Slow Response Times

**Solutions**:
1. **Check network**: Test server response times
2. **Increase timeout**: For slow networks
3. **Reduce retry attempts**: If retries are causing delays
4. **Check server load**: Remote servers may be slow

### High Memory Usage

**Solutions**:
1. **Monitor processes**: Check for memory leaks
2. **Reduce concurrent servers**: Limit number of active servers
3. **Restart server**: Periodically restart to free memory

## Debugging Commands

### Enable Debug Mode

```bash
mcp-connect serve --debug
```

### Verbose Logging

```bash
mcp-connect serve --log-level trace
```

### Test Individual Components

```bash
# Test configuration
mcp-connect config validate

# Test server connection
mcp-connect config test server-name

# Test all servers
mcp-connect config test --all

# Test registry
mcp-connect registry search test
```

## Getting Help

### Check Logs

Always check logs first:
```bash
mcp-connect serve --debug 2>&1 | tee mcp-connect.log
```

### Collect Information

When reporting issues, include:
1. MCP Connect version: `mcp-connect --version`
2. Configuration file (sanitized): `mcp-connect config list`
3. Error messages: Full error output
4. System information: OS, Rust version
5. Steps to reproduce

### Common Resources

- **Documentation**: See other docs in `docs/` directory
- **Issues**: Check GitHub Issues for similar problems
- **Examples**: Review `examples/` directory for usage patterns

## Prevention Tips

1. **Validate before committing**: Always run `mcp-connect config validate`
2. **Test after changes**: Use `mcp-connect config test --all` after adding servers
3. **Keep backups**: Backup `.mcp-connect.json` before major changes
4. **Monitor logs**: Regularly check logs for warnings
5. **Update regularly**: Keep MCP Connect updated to latest version

