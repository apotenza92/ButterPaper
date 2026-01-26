#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

echo "=== Building ButterPaper ==="
echo ""

# Step 1: Ensure PDFium is available
echo "Step 1: Setting up PDFium..."
"$SCRIPT_DIR/setup-pdfium.sh"
echo ""

# Step 2: Build release binary
echo "Step 2: Building release binary..."
cargo build --release -p butterpaper-gpui
echo ""

# Step 3: Create app bundle
echo "Step 3: Creating app bundle..."
cargo bundle --release -p butterpaper-gpui
echo ""

# Step 4: Copy PDFium to bundle and set @rpath
BUNDLE_DIR="$PROJECT_DIR/target/release/bundle/osx/ButterPaper.app/Contents/MacOS"
cp "$PROJECT_DIR/libpdfium.dylib" "$BUNDLE_DIR/"
echo "✓ Copied libpdfium.dylib to bundle"

# Set @rpath in the executable so it can find the dylib in the same directory
# This uses @executable_path which resolves to .app/Contents/MacOS/
EXECUTABLE="$BUNDLE_DIR/butterpaper"
if [ -f "$EXECUTABLE" ]; then
    # Add @executable_path to rpath if not already present
    if ! otool -l "$EXECUTABLE" | grep -q "@executable_path"; then
        install_name_tool -add_rpath @executable_path "$EXECUTABLE" 2>/dev/null || true
    fi
    echo "✓ Set @rpath to @executable_path"
fi
echo ""

# Step 5: Verify
echo "Step 5: Verifying bundle..."
if [ -f "$BUNDLE_DIR/butterpaper" ] && [ -f "$BUNDLE_DIR/libpdfium.dylib" ]; then
    echo "✓ Bundle is complete!"
    echo ""
    echo "App location:"
    echo "  $PROJECT_DIR/target/release/bundle/osx/ButterPaper.app"
    echo ""
    echo "To run:"
    echo "  open \"$PROJECT_DIR/target/release/bundle/osx/ButterPaper.app\""
    echo ""
    echo "To install:"
    echo "  cp -r \"$PROJECT_DIR/target/release/bundle/osx/ButterPaper.app\" /Applications/"
else
    echo "✗ Bundle verification failed!"
    exit 1
fi
