#!/bin/bash
# AvocadoDB Installation Script
# Downloads and installs the latest AvocadoDB binary for your platform

set -e

REPO="avocadodb/avocadodb"
API_URL="https://api.github.com/repos/${REPO}/releases/latest"

echo "🥑 AvocadoDB Installer"
echo ""

# Detect platform and architecture
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$ARCH" in
    x86_64)
        ARCH="x86_64"
        ;;
    amd64)
        ARCH="x86_64"
        ;;
    arm64|aarch64)
        if [ "$OS" = "darwin" ]; then
            ARCH="aarch64"
        else
            ARCH="aarch64"
        fi
        ;;
    *)
        echo "❌ Unsupported architecture: $ARCH"
        echo "   Please install manually from: https://github.com/${REPO}/releases"
        exit 1
        ;;
esac

case "$OS" in
    linux)
        PLATFORM="linux"
        EXT="tar.gz"
        BINARY="avocado"
        ;;
    darwin)
        PLATFORM="macos"
        EXT="tar.gz"
        BINARY="avocado"
        ;;
    *)
        echo "❌ Unsupported OS: $OS"
        echo "   Please install manually from: https://github.com/${REPO}/releases"
        exit 1
        ;;
esac

# Get latest release version
echo "📦 Fetching latest version..."
VERSION=$(curl -s "$API_URL" | grep '"tag_name":' | sed -E 's/.*"v([^"]+)".*/\1/' || echo "")
if [ -z "$VERSION" ]; then
    echo "❌ Failed to fetch latest version"
    exit 1
fi

echo "   Latest version: v${VERSION}"
echo ""

# Build download URL
ASSET_NAME="avocado-cli-${VERSION}-${PLATFORM}-${ARCH}.${EXT}"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/${ASSET_NAME}"

# Create temporary directory
TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

# Download and extract
echo "⬇️  Downloading ${ASSET_NAME}..."
curl -fsSL -o "${TMP_DIR}/${ASSET_NAME}" "$DOWNLOAD_URL" || {
    echo "❌ Failed to download binary"
    echo "   URL: $DOWNLOAD_URL"
    exit 1
}

echo "📂 Extracting..."
cd "$TMP_DIR"
if [ "$EXT" = "tar.gz" ]; then
    tar -xzf "$ASSET_NAME"
else
    unzip -q "$ASSET_NAME"
fi

EXTRACTED_DIR="avocado-cli-${VERSION}-${PLATFORM}-${ARCH}"

if [ ! -f "$EXTRACTED_DIR/$BINARY" ]; then
    echo "❌ Binary not found in archive"
    exit 1
fi

# Install to /usr/local/bin (requires sudo)
INSTALL_DIR="/usr/local/bin"
BINARY_PATH="$EXTRACTED_DIR/$BINARY"

echo ""
echo "📦 Installing to ${INSTALL_DIR}..."
if [ -w "$INSTALL_DIR" ]; then
    cp "$BINARY_PATH" "$INSTALL_DIR/"
else
    sudo cp "$BINARY_PATH" "$INSTALL_DIR/"
fi

# Make executable
if [ -w "$INSTALL_DIR" ]; then
    chmod +x "$INSTALL_DIR/$BINARY"
else
    sudo chmod +x "$INSTALL_DIR/$BINARY"
fi

# Verify installation
if command -v "$BINARY" >/dev/null 2>&1; then
    INSTALLED_VERSION=$($BINARY --version 2>/dev/null || echo "unknown")
    echo ""
    echo "✅ AvocadoDB installed successfully!"
    echo ""
    echo "   Binary: ${INSTALL_DIR}/${BINARY}"
    echo "   Version: ${INSTALLED_VERSION}"
    echo ""
    echo "   Run '${BINARY} --help' to get started"
    echo "   Or visit: https://github.com/${REPO}"
else
    echo ""
    echo "⚠️  Installation completed, but binary not found in PATH"
    echo "   Please add ${INSTALL_DIR} to your PATH"
fi

