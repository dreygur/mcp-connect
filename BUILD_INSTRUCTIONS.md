# MCP Remote Proxy - Build Instructions

This document provides comprehensive instructions for building standalone release binaries of the MCP Remote Proxy.

## Prerequisites

### System Requirements

- Rust 1.75 or later
- Cargo (comes with Rust)
- Git
- OpenSSL development libraries

### Installing Dependencies

**Ubuntu/Debian:**

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install build dependencies
sudo apt update
sudo apt install build-essential libssl-dev pkg-config git
```

**CentOS/RHEL/Fedora:**

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install build dependencies
sudo dnf install gcc openssl-devel pkg-config git
```

## Building Release Binary

### Method 1: Simple Build (Recommended)

```bash
# Clone the repository
git clone <repository-url>
cd tokio-night-gnome

# Build with static OpenSSL
export OPENSSL_STATIC=1
cargo build --release

# The binary will be at: target/release/mcp-remote
```

### Method 2: Using Build Script

```bash
# Use the provided build script
./build-static.sh

# Binary will be created in: dist/mcp-remote
```

### Method 3: Cross-Platform Build

```bash
# Build for multiple platforms
./build-cross-platform.sh

# Creates binaries for Linux, macOS, and Windows in dist-cross/
```

## Build Options

### Static Linking

For maximum portability, use static linking:

```bash
export OPENSSL_STATIC=1
cargo build --release
```

### Musl Target (Fully Static)

For a truly static binary with no dependencies:

```bash
# Install musl target
rustup target add x86_64-unknown-linux-musl

# Install musl tools (Ubuntu/Debian)
sudo apt install musl-tools

# Build static binary
cargo build --release --target x86_64-unknown-linux-musl
```

### Debug Build

For development with debug symbols:

```bash
cargo build --debug
```

## Optimization Flags

### Size Optimization

To minimize binary size:

```bash
export RUSTFLAGS="-C opt-level=z -C lto=yes -C codegen-units=1 -C panic=abort"
cargo build --release
```

### Performance Optimization

For maximum performance:

```bash
export RUSTFLAGS="-C opt-level=3 -C target-cpu=native"
cargo build --release
```

## Troubleshooting

### OpenSSL Issues

**Problem:** `libssl.so.3: cannot open shared object file`
**Solution:**

```bash
# Install OpenSSL 3.x
sudo apt install libssl3 libssl-dev  # Ubuntu/Debian
sudo dnf install openssl-devel       # Fedora/RHEL

# Or build with static OpenSSL
export OPENSSL_STATIC=1
cargo build --release
```

**Problem:** Cross-compilation OpenSSL errors
**Solution:**

```bash
# Use vendored OpenSSL (if available)
cargo build --release --features vendored-openssl

# Or install target-specific OpenSSL
sudo apt install libssl-dev:arm64  # for ARM64 target
```

### Musl Build Issues

**Problem:** Proc-macro compilation errors
**Solution:**

```bash
# Ensure musl-tools is installed
sudo apt install musl-tools

# Set correct environment variables
export CC_x86_64_unknown_linux_musl=musl-gcc
cargo build --release --target x86_64-unknown-linux-musl
```

### Large Binary Size

**Problem:** Binary is larger than expected
**Solutions:**

```bash
# Strip debug symbols
strip target/release/mcp-remote

# Use size optimization flags
export RUSTFLAGS="-C opt-level=z -C lto=yes"
cargo build --release

# Use UPX compression (optional)
upx --best target/release/mcp-remote
```

## Verification

### Test the Binary

```bash
# Check version
./mcp-remote --version

# Test help output
./mcp-remote --help

# Test basic functionality
./mcp-remote test --endpoint "https://httpbin.org/post" --timeout 5
```

### Check Dependencies

```bash
# On Linux, check dynamic libraries
ldd ./mcp-remote

# Check file type
file ./mcp-remote

# Check size
du -h ./mcp-remote
```

### Performance Testing

```bash
# Test connection speed
time ./mcp-remote test --endpoint "https://api.example.com/mcp"

# Memory usage
valgrind --tool=massif ./mcp-remote --help
```

## Distribution

### Creating Release Package

```bash
# Create distribution directory
mkdir -p dist

# Copy binary and documentation
cp target/release/mcp-remote dist/
cp README.md dist/
cp LICENSE dist/ # if available

# Create usage guide
cat > dist/USAGE.txt << 'EOF'
Quick Start Guide for MCP Remote Proxy

1. Test version: ./mcp-remote --version
2. Get help: ./mcp-remote --help
3. Run proxy: ./mcp-remote proxy --endpoint "URL" --auth-token "TOKEN"
EOF

# Create tarball
tar -czf mcp-remote-release.tar.gz -C dist .
```

### Docker Build (Alternative)

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/mcp-remote /usr/local/bin/
ENTRYPOINT ["mcp-remote"]
```

## Continuous Integration

### GitHub Actions Example

```yaml
name: Build Release
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Install dependencies
        run: sudo apt install libssl-dev
      - name: Build
        run: |
          export OPENSSL_STATIC=1
          cargo build --release
      - name: Test
        run: ./target/release/mcp-remote --version
```

## Build Variants

### Development Build

```bash
cargo build  # Fast compilation, debug symbols
```

### Release Build

```bash
cargo build --release  # Optimized, smaller binary
```

### Profile-Guided Optimization (Advanced)

```bash
# Build with profiling
export RUSTFLAGS="-C profile-generate=/tmp/pgo-data"
cargo build --release

# Run typical workload
./target/release/mcp-remote --help
# ... more usage patterns ...

# Rebuild with profile data
export RUSTFLAGS="-C profile-use=/tmp/pgo-data"
cargo build --release
```

This should give you a production-ready binary that can be deployed on any compatible Linux system!
