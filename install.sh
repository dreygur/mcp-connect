#!/usr/bin/env bash
set -e

BINARY="mcp-connect"
OWNER="dreygur"
REPO="mcp-connect"

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
  x86_64|amd64) ARCH="x86_64" ;;
  arm64|aarch64) ARCH="aarch64" ;;
  *) echo "❌ Unsupported architecture: $ARCH"; exit 1 ;;
esac

case "$OS" in
  linux) FILE="${BINARY}-linux-${ARCH}.tar.gz" ;;
  darwin) FILE="${BINARY}-darwin-${ARCH}.tar.gz" ;;
  *) echo "❌ Unsupported OS: $OS"; exit 1 ;;
esac

# Fetch latest release tag
TAG=$(curl -s https://api.github.com/repos/$OWNER/$REPO/releases/latest | grep tag_name | cut -d '"' -f 4)
if [ -z "$TAG" ]; then
  echo "❌ Could not determine latest release tag."
  exit 1
fi

BASE_URL="https://github.com/$OWNER/$REPO/releases/download/$TAG"
CHECKSUM_FILE="${FILE}.sha256"

echo "📦 Downloading ${FILE} (version ${TAG})..."
curl -fsSL "$BASE_URL/$FILE" -o "$FILE"
curl -fsSL "$BASE_URL/$CHECKSUM_FILE" -o "$CHECKSUM_FILE"

echo "🔐 Verifying checksum..."
shasum -a 256 -c "$CHECKSUM_FILE"

echo "📂 Extracting..."
tar -xzf "$FILE"

echo "⚙️ Installing to /usr/local/bin (sudo required)..."
chmod +x "$BINARY"
sudo mv "$BINARY" /usr/local/bin/

echo "🧹 Cleaning up temp files..."
rm -f "$FILE" "$CHECKSUM_FILE"

echo "✅ Installed $BINARY successfully!"
$BINARY --version
