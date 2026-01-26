#!/bin/bash
set -e

SCRIPT_DIR="$(dirname "$0")"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Ensure we're using Xcode.app for the Metal compiler
export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer

# Check for Metal compiler
if ! xcrun --find metal &>/dev/null; then
    echo "Error: Metal compiler not found."
    echo ""
    echo "Please run:"
    echo "  sudo xcode-select -s /Applications/Xcode.app/Contents/Developer"
    echo "  xcodebuild -downloadComponent MetalToolchain"
    exit 1
fi

cd "$PROJECT_ROOT"

# Kill any running instances of the app
pkill -f "target/release/butterpaper" 2>/dev/null || true
pkill -f "target/debug/butterpaper" 2>/dev/null || true

echo "Building ButterPaper (GPUI)..."
cargo build --release -p butterpaper-gpui

# Copy libpdfium.dylib to target/release if it exists in project root
if [ -f "$PROJECT_ROOT/libpdfium.dylib" ]; then
    cp "$PROJECT_ROOT/libpdfium.dylib" "$PROJECT_ROOT/target/release/"
fi

# Create convenience symlink
ln -sf butterpaper "$PROJECT_ROOT/target/release/butterpaper-gpui" 2>/dev/null || true

# Code sign (required on macOS to avoid Gatekeeper blocking)
echo "Signing binary..."
xattr -cr "$PROJECT_ROOT/target/release/butterpaper" 2>/dev/null || true
codesign -s - --force "$PROJECT_ROOT/target/release/butterpaper" 2>/dev/null || true

echo ""
echo "Build complete! Run with:"
echo "  $PROJECT_ROOT/target/release/butterpaper"
