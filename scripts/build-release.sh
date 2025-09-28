#!/bin/bash

# MCP Remote Proxy - Standalone Release Build Script
# Creates a fully static binary with no shared library dependencies

set -e  # Exit on any error

PROJECT_NAME="mcp-connect"
BINARY_NAME="mcp-connect"
BUILD_DIR="target/release"
DIST_DIR="dist"

echo "🚀 Building standalone release for MCP Remote Proxy..."
echo "📦 Target: ${PROJECT_NAME} -> ${BINARY_NAME}"

# Function to print colored output
print_step() {
    echo -e "\n🔧 $1"
}

print_success() {
    echo -e "\n✅ $1"
}

print_error() {
    echo -e "\n❌ $1"https://github.com/dreygur/mcp-connect
}

# Check if we're on Linux for musl target
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    TARGET="x86_64-unknown-linux-musl"
    print_step "Detected Linux - using musl target for maximum compatibility"

    # Install musl target if not present
    if ! rustup target list --installed | grep -q "$TARGET"; then
        print_step "Installing musl target..."
        rustup target add "$TARGET"
    fi

    # Install musl-tools if not present (Ubuntu/Debian)
    if command -v apt-get >/dev/null 2>&1; then
        if ! dpkg -l | grep -q musl-tools; then
            print_step "Installing musl-tools (may require sudo)..."
            sudo apt-get update && sudo apt-get install -y musl-tools
        fi
    fi

    TARGET_FLAG="--target $TARGET"
else
    print_step "Non-Linux platform detected - using default target"
    TARGET_FLAG=""
fi

# Clean previous builds
print_step "Cleaning previous builds..."
cargo clean

# Set environment variables for static linking
export RUSTFLAGS="-C target-feature=+crt-static"
export OPENSSL_STATIC=1
export OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu
export OPENSSL_INCLUDE_DIR=/usr/include/openssl

# For musl builds, we need additional flags
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    export CC_x86_64_unknown_linux_musl=musl-gcc
    export RUSTFLAGS="-C target-feature=+crt-static -C link-arg=-static"
fi

print_step "Building release binary with static linking..."
print_step "Environment variables:"
echo "  RUSTFLAGS: $RUSTFLAGS"
echo "  OPENSSL_STATIC: $OPENSSL_STATIC"
echo "  TARGET: ${TARGET:-default}"

# Build the release binary
if ! cargo build --release $TARGET_FLAG; then
    print_error "Build failed!"
    exit 1
fi

# Determine the actual binary path
if [[ -n "$TARGET" ]]; then
    BINARY_PATH="target/$TARGET/release/$BINARY_NAME"
else
    BINARY_PATH="$BUILD_DIR/$BINARY_NAME"
fi

# Verify the binary was created
if [[ ! -f "$BINARY_PATH" ]]; then
    print_error "Binary not found at $BINARY_PATH"
    exit 1
fi

print_success "Build completed successfully!"

# Create distribution directory
print_step "Creating distribution package..."
mkdir -p "$DIST_DIR"

# Copy binary to dist directory
cp "$BINARY_PATH" "$DIST_DIR/"

# Make binary executable
chmod +x "$DIST_DIR/$BINARY_NAME"

# Check if binary is truly standalone (Linux only)
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    print_step "Checking binary dependencies..."

    if command -v ldd >/dev/null 2>&1; then
        echo "Dependencies check:"
        if ldd "$DIST_DIR/$BINARY_NAME" 2>/dev/null | grep -v "not a dynamic executable"; then
            print_error "Binary has dynamic dependencies - not fully static!"
            ldd "$DIST_DIR/$BINARY_NAME"
        else
            print_success "Binary is fully static with no shared library dependencies!"
        fi
    fi

    if command -v file >/dev/null 2>&1; then
        echo "File type:"
        file "$DIST_DIR/$BINARY_NAME"
    fi
fi

# Get binary size
BINARY_SIZE=$(du -h "$DIST_DIR/$BINARY_NAME" | cut -f1)

# Test the binary
print_step "Testing the standalone binary..."
if "$DIST_DIR/$BINARY_NAME" --version >/dev/null 2>&1; then
    VERSION_OUTPUT=$("$DIST_DIR/$BINARY_NAME" --version)
    print_success "Binary test passed!"
    echo "  Version: $VERSION_OUTPUT"
else
    print_error "Binary test failed!"
    exit 1
fi

# Create additional distribution files
print_step "Creating distribution package..."

# Copy additional files
cp README.md "$DIST_DIR/" 2>/dev/null || echo "README.md not found, skipping..."
cp LICENSE "$DIST_DIR/" 2>/dev/null || echo "LICENSE not found, skipping..."

# Create a simple usage guide
cat > "$DIST_DIR/USAGE.txt" << 'EOF'
MCP Remote Proxy - Standalone Release

This is a fully static binary with no dependencies.

Quick start:
1. Make sure the binary is executable: chmod +x mcp-connect
2. Test connection: ./mcp-connect test --endpoint "https://api.example.com/mcp"
3. Run proxy: ./mcp-connect proxy --endpoint "https://api.example.com/mcp" --auth-token "your-token"

For full documentation, see README.md or run: ./mcp-connect --help
EOF

# Create version info
echo "Build Information:" > "$DIST_DIR/BUILD_INFO.txt"
echo "  Built on: $(date)" >> "$DIST_DIR/BUILD_INFO.txt"
echo "  Rust version: $(rustc --version)" >> "$DIST_DIR/BUILD_INFO.txt"
echo "  Binary size: $BINARY_SIZE" >> "$DIST_DIR/BUILD_INFO.txt"
echo "  Target: ${TARGET:-default}" >> "$DIST_DIR/BUILD_INFO.txt"
echo "  Static linking: Yes" >> "$DIST_DIR/BUILD_INFO.txt"

print_success "Release build completed!"
echo ""
echo "📂 Distribution files created in: $DIST_DIR/"
echo "📄 Files included:"
ls -la "$DIST_DIR/"
echo ""
echo "🎯 Binary size: $BINARY_SIZE"
echo "🔧 Usage: ./$DIST_DIR/$BINARY_NAME --help"
echo ""
echo "🚀 The binary is fully standalone and can be deployed anywhere!"

# Optional: Create a tarball
if command -v tar >/dev/null 2>&1; then
    TARBALL="$PROJECT_NAME-release-$(date +%Y%m%d-%H%M%S).tar.gz"
    print_step "Creating release tarball: $TARBALL"
    tar -czf "$TARBALL" -C "$DIST_DIR" .
    print_success "Release tarball created: $TARBALL"
fi

print_success "Build script completed successfully!"
