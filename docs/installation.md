# Installation Guide

This guide covers installing MCP Connect on various platforms and methods.

## Prerequisites

- **Rust**: Version 1.75 or later
- **Cargo**: Latest stable version (comes with Rust)
- **OpenSSL**: 3.x (for SSL/TLS support)

## Installation Methods

### Method 1: From Source (Recommended)

This is the recommended method for getting the latest features and bug fixes.

```bash
# Clone the repository
git clone https://github.com/dreygur/mcp-connect.git
cd mcp-connect

# Build in release mode
cargo build --release

# The binary will be at: target/release/mcp-connect
```

**Install globally:**
```bash
cargo install --path crates/mcp-connect
```

This installs `mcp-connect` to your Cargo bin directory (usually `~/.cargo/bin/`).

### Method 2: Using Cargo (from crates.io)

When published to crates.io:

```bash
cargo install mcp-connect
```

### Method 3: Pre-built Binaries

Download pre-built binaries from the [Releases](https://github.com/dreygur/mcp-connect/releases) page.

**Linux:**
```bash
wget https://github.com/dreygur/mcp-connect/releases/latest/download/mcp-connect-x86_64-unknown-linux-gnu.tar.gz
tar -xzf mcp-connect-x86_64-unknown-linux-gnu.tar.gz
sudo mv mcp-connect /usr/local/bin/
```

**macOS:**
download from releases page

**Windows:**
Download the `.exe` from the releases page and add to your PATH.

## Platform-Specific Instructions

### Linux (Ubuntu/Debian)

```bash
# Install OpenSSL
sudo apt update
sudo apt install libssl3 libssl-dev

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build MCP Connect
git clone https://github.com/dreygur/mcp-connect.git
cd mcp-connect
cargo build --release
```

### Linux (Fedora/RHEL)

```bash
# Install OpenSSL
sudo dnf install openssl-devel

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build MCP Connect
git clone https://github.com/dreygur/mcp-connect.git
cd mcp-connect
cargo build --release
```

### macOS

```bash
# Install OpenSSL via Homebrew
brew install openssl

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build MCP Connect
git clone https://github.com/dreygur/mcp-connect.git
cd mcp-connect
cargo build --release
```

### Windows

**Using Rustup (recommended):**

1. Download and run [rustup-init.exe](https://rustup.rs/)
2. Install OpenSSL (via vcpkg or pre-built binaries)
3. Clone and build:

```powershell
git clone https://github.com/dreygur/mcp-connect.git
cd mcp-connect
cargo build --release
```

## Static Linking (Optional)

For better portability, you can build with statically linked OpenSSL:

```bash
# Install pkg-config
# Linux: sudo apt install pkg-config
# macOS: brew install pkg-config

# Build with static OpenSSL
OPENSSL_STATIC=1 cargo build --release
```

## Verifying Installation

After installation, verify it works:

```bash
mcp-connect --version
mcp-connect --help
```

## Troubleshooting Installation

### OpenSSL Errors

**Error**: `libssl.so.3: cannot open shared object file`

**Solution**:
```bash
# Ubuntu/Debian
sudo apt install libssl3

# Fedora/RHEL
sudo dnf install openssl-libs

# Or build with static OpenSSL
OPENSSL_STATIC=1 cargo build --release
```

### Rust Version Errors

**Error**: `error: edition 2021 is unstable`

**Solution**: Update Rust:
```bash
rustup update stable
```

### Permission Errors

**Error**: `Permission denied` when running `mcp-connect`

**Solution**:
```bash
# Make executable
chmod +x target/release/mcp-connect

# Or add to PATH
export PATH="$HOME/.cargo/bin:$PATH"
```

## Next Steps

After installation, proceed to the [Getting Started Guide](getting-started.md) to set up your first MCP Connect configuration.

