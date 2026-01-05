# Installation Guide

## Installation Methods

### npm (recommended)

The easiest way to install MCP Connect:

```bash
npm install -g @dreygur/mcp
```

**Requirements:** Node.js 14+

### Shell script (Linux/macOS)

One-line install with automatic platform detection:

```bash
curl -fsSL https://raw.githubusercontent.com/dreygur/mcp-connect/main/install.sh | bash
```

This downloads the latest release, verifies checksum, and installs to `/usr/local/bin`.

### PowerShell (Windows)

```powershell
irm https://raw.githubusercontent.com/dreygur/mcp-connect/main/install.ps1 | iex
```

This installs to `C:\Program Files\mcp-connect` and adds it to PATH.

### Cargo (from GitHub)

```bash
cargo install --git https://github.com/dreygur/mcp-connect
```

**Requirements:** Rust 1.75+, OpenSSL 3.x

### Cargo (from crates.io)

When published:

```bash
cargo install mcp-connect
```

### From source

```bash
git clone https://github.com/dreygur/mcp-connect.git
cd mcp-connect
cargo install --path crates/mcp-connect
```

Or build without installing:

```bash
cargo build --release
# Binary at: target/release/mcp-connect
```

### Pre-built binaries

Download from the [releases page](https://github.com/dreygur/mcp-connect/releases):

| Platform | Architecture | File |
|----------|-------------|------|
| Linux | x86_64 | `mcp-connect-linux-x86_64.tar.gz` |
| macOS | x86_64 | `mcp-connect-darwin-x86_64.tar.gz` |
| macOS | ARM64 | `mcp-connect-darwin-aarch64.tar.gz` |
| Windows | x86_64 | `mcp-connect-windows-x86_64.zip` |

**Linux/macOS:**
```bash
tar -xzf mcp-connect-*.tar.gz
chmod +x mcp-connect
sudo mv mcp-connect /usr/local/bin/
```

**Windows:**
Extract the zip and add the folder to your PATH.

## Platform-Specific Notes

### Linux (Ubuntu/Debian)

```bash
# Install dependencies for building from source
sudo apt update
sudo apt install libssl-dev pkg-config

# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Linux (Fedora/RHEL)

```bash
sudo dnf install openssl-devel pkg-config
```

### macOS

```bash
# Install OpenSSL via Homebrew (for building from source)
brew install openssl pkg-config
```

### Windows

For building from source, install OpenSSL via [vcpkg](https://vcpkg.io/) or use pre-built binaries.

## Static linking (optional)

For portable binaries without OpenSSL dependency:

```bash
OPENSSL_STATIC=1 cargo build --release
```

## Verify installation

```bash
mcp-connect --version
mcp-connect --help
```

## Troubleshooting

### OpenSSL errors

**Error:** `libssl.so.3: cannot open shared object file`

```bash
# Ubuntu/Debian
sudo apt install libssl3

# Fedora/RHEL
sudo dnf install openssl-libs

# Or build with static linking
OPENSSL_STATIC=1 cargo build --release
```

### Permission errors

```bash
chmod +x mcp-connect
# Or add cargo bin to PATH
export PATH="$HOME/.cargo/bin:$PATH"
```

### Rust version errors

```bash
rustup update stable
```

## Next steps

See [Getting Started](getting-started.md) to configure your first MCP server.
