MCP Remote Proxy - Release Binary
=================================

This directory contains a release build of the MCP Remote Proxy.

Binary Information:
- File: mcp-connect
- Version: 0.1.0
- Size: 6.2M
- Platform: Linux x86_64

Dependencies:
The binary requires these system libraries:
- libssl.so.3 (OpenSSL 3.x)
- libcrypto.so.3 (OpenSSL crypto)
- Standard system libraries (glibc)

Quick Start:
1. Make sure the binary is executable: chmod +x mcp-connect
2. Test version: ./mcp-connect --version
3. Get help: ./mcp-connect --help

Usage Examples:
1. Basic proxy:
   ./mcp-connect proxy --endpoint "https://api.example.com/mcp" --auth-token "your-token"

2. Test connection:
   ./mcp-connect test --endpoint "https://api.example.com/mcp" --auth-token "your-token"

3. With debug logging:
   ./mcp-connect proxy --endpoint "https://api.example.com/mcp" --debug

Authentication:
- Use --auth-token for Bearer tokens
- Use --api-key for API key authentication
- Use --headers for custom headers like "Authorization:Bearer token"

System Requirements:
- Linux x86_64
- OpenSSL 3.x libraries installed
- Glibc (standard on most Linux distributions)

For full documentation, see the project README.md at:
https://github.com/your-repo/tokio-night-gnome
