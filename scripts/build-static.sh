#!/bin/bash

# Simple static build for MCP Connect
# Creates a binary with statically linked OpenSSL

set -e

echo "🚀 Building MCP Connect with static OpenSSL..."

# Set environment for static OpenSSL
export OPENSSL_STATIC=1

# Clean previous build
echo "🧹 Cleaning..."
cargo clean

# Build with static OpenSSL
echo "🔨 Building release..."
cargo build --release

# Create dist directory
DIST_DIR="dist"
mkdir -p "$DIST_DIR"

# Copy binary
if [[ -f "target/release/mcp-connect" ]]; then
    cp "target/release/mcp-connect" "$DIST_DIR/"
    chmod +x "$DIST_DIR/mcp-connect"
    echo "✅ Binary created: $DIST_DIR/mcp-connect"
else
    echo "❌ Build failed - binary not found"
    exit 1
fi

# Test the binary
echo "🧪 Testing binary..."
if "$DIST_DIR/mcp-connect" --version; then
    echo "✅ Binary works!"
else
    echo "❌ Binary test failed"
    exit 1
fi

# Check size
SIZE=$(du -h "$DIST_DIR/mcp-connect" | cut -f1)
echo "📏 Binary size: $SIZE"

# Check dependencies (if ldd is available)
if command -v ldd >/dev/null 2>&1; then
    echo "🔍 Dependencies:"
    ldd "$DIST_DIR/mcp-connect" || echo "✅ Static binary (no dynamic dependencies)"
fi

echo "✅ Build completed successfully!"
echo "📂 Binary location: $DIST_DIR/mcp-connect"
