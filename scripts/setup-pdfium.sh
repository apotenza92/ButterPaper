#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
PDFIUM_URL="https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-mac-arm64.tgz"
PDFIUM_LIB="libpdfium.dylib"

echo "=== PDFium Setup ==="

# Check if already exists
if [ -f "$PROJECT_DIR/$PDFIUM_LIB" ]; then
    echo "✓ $PDFIUM_LIB already exists in project root"
else
    echo "Downloading PDFium..."
    curl -L "$PDFIUM_URL" -o /tmp/pdfium.tgz
    echo "Extracting..."
    tar -xzf /tmp/pdfium.tgz -C /tmp
    cp /tmp/lib/$PDFIUM_LIB "$PROJECT_DIR/"
    rm -rf /tmp/pdfium.tgz /tmp/lib /tmp/include
    echo "✓ Installed $PDFIUM_LIB to project root"
fi

# Check bundle location
BUNDLE_DIR="$PROJECT_DIR/target/release/bundle/osx/PDF Editor.app/Contents/MacOS"
if [ -d "$BUNDLE_DIR" ]; then
    if [ ! -f "$BUNDLE_DIR/$PDFIUM_LIB" ]; then
        cp "$PROJECT_DIR/$PDFIUM_LIB" "$BUNDLE_DIR/"
        echo "✓ Copied $PDFIUM_LIB to app bundle"
    else
        echo "✓ $PDFIUM_LIB already in app bundle"
    fi
else
    echo "⚠ App bundle not found. Run 'cargo bundle --release' first."
fi

echo ""
echo "Setup complete!"
