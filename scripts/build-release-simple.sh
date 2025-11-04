#!/bin/bash

# Simple standalone release build for MCP Connect

set -e

echo "🚀 Building standalone MCP Connect..."

# Set environment variables for static linking
export OPENSSL_STATIC=1

# Clean and build
echo "🧹 Cleaning previous build..."
cargo clean

echo "🔨 Building release with static linking..."
# Use musl target for truly static binary with vendored OpenSSL
if rustup target list --installed | grep -q "x86_64-unknown-linux-musl"; then
    echo "📦 Using musl target with vendored OpenSSL..."
    cargo build --release --target x86_64-unknown-linux-musl --features vendored-openssl
    BINARY_PATH="target/x86_64-unknown-linux-musl/release/mcp-connect"
else
    echo "📦 Using default target with static OpenSSL..."
    cargo build --release
    BINARY_PATH="target/release/mcp-connect"
fi

# Create distribution directory
DIST_DIR="dist"
mkdir -p "$DIST_DIR"

# Copy binary
if [[ -f "$BINARY_PATH" ]]; then
    cp "$BINARY_PATH" "$DIST_DIR/"
    chmod +x "$DIST_DIR/mcp-connect"
    echo "✅ Binary copied to $DIST_DIR/"
else
    echo "❌ Binary not found at $BINARY_PATH"
    exit 1
fi

# Test binary
echo "🧪 Testing standalone binary..."
if "$DIST_DIR/mcp-connect" --version; then
    echo "✅ Binary test passed!"
else
    echo "❌ Binary test failed!"
    exit 1
fi

# Check dependencies (Linux only)
if command -v ldd >/dev/null 2>&1; then
    echo "🔍 Checking dependencies..."
    if ldd "$DIST_DIR/mcp-connect" 2>/dev/null | grep -v "not a dynamic executable"; then
        echo "⚠️  Binary has dependencies:"
        ldd "$DIST_DIR/mcp-connect"
    else
        echo "✅ Binary is fully static!"
    fi
fi

# Get size
SIZE=$(du -h "$DIST_DIR/mcp-connect" | cut -f1)
echo "📏 Binary size: $SIZE"

echo "✅ Standalone build completed!"
echo "📂 Binary location: $DIST_DIR/mcp-connect"
