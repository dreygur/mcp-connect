#!/bin/bash

# MCP Remote Proxy - Cross-Platform Release Build Script
# Creates standalone binaries for multiple platforms

set -e

PROJECT_NAME="mcp-remote"
BINARY_NAME="mcp-remote"
DIST_DIR="dist-cross"

echo "🌍 Building cross-platform releases for MCP Remote Proxy..."

# Define target platforms
declare -A TARGETS=(
    ["linux-x64"]="x86_64-unknown-linux-musl"
    ["linux-arm64"]="aarch64-unknown-linux-musl"
    ["macos-x64"]="x86_64-apple-darwin"
    ["macos-arm64"]="aarch64-apple-darwin"
    ["windows-x64"]="x86_64-pc-windows-gnu"
)

# Function to print colored output
print_step() {
    echo -e "\n🔧 $1"
}

print_success() {
    echo -e "\n✅ $1"
}

print_error() {
    echo -e "\n❌ $1"
}

# Install cross-compilation tools
install_cross_tools() {
    print_step "Installing cross-compilation tools..."

    # Install cross if not present
    if ! command -v cross >/dev/null 2>&1; then
        print_step "Installing 'cross' for cross-compilation..."
        cargo install cross --git https://github.com/cross-rs/cross
    fi

    # Add targets
    for platform in "${!TARGETS[@]}"; do
        target="${TARGETS[$platform]}"
        if ! rustup target list --installed | grep -q "$target"; then
            print_step "Adding target: $target"
            rustup target add "$target" || echo "Warning: Could not add target $target"
        fi
    done
}

# Build for a specific target
build_target() {
    local platform=$1
    local target=$2
    local binary_suffix=""

    print_step "Building for $platform ($target)..."

    # Windows builds need .exe extension
    if [[ "$target" == *"windows"* ]]; then
        binary_suffix=".exe"
    fi

    # Set environment variables for static linking
    export RUSTFLAGS="-C target-feature=+crt-static"
    export OPENSSL_STATIC=1

    # Try cross-compilation first, fall back to regular cargo build
    if command -v cross >/dev/null 2>&1 && [[ "$target" != *"darwin"* ]]; then
        # Use cross for non-macOS targets
        cross build --release --target "$target" --bin "$BINARY_NAME"
    else
        # Use regular cargo for macOS or if cross is not available
        cargo build --release --target "$target" --bin "$BINARY_NAME"
    fi

    local binary_path="target/$target/release/$BINARY_NAME$binary_suffix"

    if [[ -f "$binary_path" ]]; then
        # Create platform-specific directory
        local platform_dir="$DIST_DIR/$platform"
        mkdir -p "$platform_dir"

        # Copy binary
        cp "$binary_path" "$platform_dir/$BINARY_NAME$binary_suffix"
        chmod +x "$platform_dir/$BINARY_NAME$binary_suffix"

        # Get binary size
        local size=$(du -h "$platform_dir/$BINARY_NAME$binary_suffix" | cut -f1)

        print_success "Built $platform binary ($size)"

        # Test the binary (only for current platform)
        if [[ "$platform" == "linux-x64" ]] || [[ "$OSTYPE" == "darwin"* && "$platform" == "macos"* ]]; then
            if "$platform_dir/$BINARY_NAME$binary_suffix" --version >/dev/null 2>&1; then
                echo "  ✓ Binary test passed"
            else
                echo "  ⚠ Binary test failed"
            fi
        fi

        return 0
    else
        print_error "Failed to build $platform binary"
        return 1
    fi
}

# Main build process
main() {
    # Clean previous builds
    print_step "Cleaning previous builds..."
    cargo clean
    rm -rf "$DIST_DIR"
    mkdir -p "$DIST_DIR"

    # Install cross-compilation tools
    install_cross_tools

    # Build for each target
    local success_count=0
    local total_count=${#TARGETS[@]}

    for platform in "${!TARGETS[@]}"; do
        target="${TARGETS[$platform]}"

        # Skip macOS builds on non-macOS systems (requires Xcode)
        if [[ "$target" == *"apple-darwin"* && "$OSTYPE" != "darwin"* ]]; then
            echo "⏭ Skipping $platform (requires macOS/Xcode)"
            continue
        fi

        if build_target "$platform" "$target"; then
            ((success_count++))
        fi
    done

    # Create universal distribution files
    print_step "Creating distribution files..."

    for platform_dir in "$DIST_DIR"/*; do
        if [[ -d "$platform_dir" ]]; then
            # Copy common files to each platform directory
            cp README.md "$platform_dir/" 2>/dev/null || true
            cp LICENSE "$platform_dir/" 2>/dev/null || true

            # Create platform-specific usage guide
            local platform_name=$(basename "$platform_dir")
            cat > "$platform_dir/USAGE.txt" << EOF
MCP Remote Proxy - $platform_name Release

This is a fully static binary with no dependencies.

Quick start:
1. Make sure the binary is executable (Unix): chmod +x $BINARY_NAME
2. Test connection: ./$BINARY_NAME test --endpoint "https://api.example.com/mcp"
3. Run proxy: ./$BINARY_NAME proxy --endpoint "https://api.example.com/mcp" --auth-token "your-token"

For full documentation, see README.md or run: ./$BINARY_NAME --help
EOF

            # Create build info
            cat > "$platform_dir/BUILD_INFO.txt" << EOF
Build Information:
  Platform: $platform_name
  Built on: $(date)
  Rust version: $(rustc --version)
  Binary size: $(du -h "$platform_dir"/* | grep -E '\.(exe)?$' | cut -f1 | head -1)
  Static linking: Yes
EOF
        fi
    done

    # Create tarballs for each platform
    if command -v tar >/dev/null 2>&1; then
        print_step "Creating release tarballs..."

        for platform_dir in "$DIST_DIR"/*; do
            if [[ -d "$platform_dir" ]]; then
                local platform_name=$(basename "$platform_dir")
                local tarball="$PROJECT_NAME-$platform_name-$(date +%Y%m%d-%H%M%S).tar.gz"

                tar -czf "$tarball" -C "$platform_dir" .
                echo "  📦 Created: $tarball"
            fi
        done
    fi

    # Summary
    print_success "Cross-platform build completed!"
    echo ""
    echo "📊 Build Summary:"
    echo "  ✅ Successful builds: $success_count"
    echo "  📂 Distribution directory: $DIST_DIR/"
    echo ""
    echo "📋 Available platforms:"
    for platform_dir in "$DIST_DIR"/*; do
        if [[ -d "$platform_dir" ]]; then
            local platform_name=$(basename "$platform_dir")
            local binary_file=$(find "$platform_dir" -name "$BINARY_NAME*" -type f | head -1)
            if [[ -f "$binary_file" ]]; then
                local size=$(du -h "$binary_file" | cut -f1)
                echo "  🎯 $platform_name: $size"
            fi
        fi
    done
    echo ""
    echo "🚀 All binaries are fully standalone and can be deployed anywhere!"
}

# Run main function
main
