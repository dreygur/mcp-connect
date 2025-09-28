#!/bin/bash

# Generate Rust documentation for MCP Remote Proxy project
# This script builds comprehensive documentation for all crates

echo "🚀 Generating Rust documentation for MCP Remote Proxy..."

# Clean previous documentation
echo "🧹 Cleaning previous documentation..."
cargo clean --doc

# Generate documentation with all features enabled
echo "📚 Building documentation..."
cargo doc \
    --workspace \
    --no-deps \
    --document-private-items \
    --open

echo "✅ Documentation generated successfully!"
echo "📖 Documentation is available at: target/doc/mcp_remote/index.html"

# Additional documentation generation options:
# --all-features          # Enable all features
# --document-private-items # Include private items in documentation
# --open                  # Open documentation in browser after generation
# --no-deps               # Don't document dependencies
