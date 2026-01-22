#!/usr/bin/env bash
# GUI Testing for PDF Editor using screenshots and automation
#
# This script uses macOS automation to test the GUI.
# For full Playwright-style testing, we'd need a browser-based app.
# Instead, we use AppleScript and screencapture for verification.

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
APP_PATH="$PROJECT_DIR/target/release/bundle/osx/PDF Editor.app"
TEST_PDF="/tmp/sample.pdf"
SCREENSHOT_DIR="$PROJECT_DIR/test-screenshots"

mkdir -p "$SCREENSHOT_DIR"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "${GREEN}✓ PASS${NC}: $1"; }
fail() { echo -e "${RED}✗ FAIL${NC}: $1"; FAILURES=$((FAILURES + 1)); }
info() { echo -e "${YELLOW}→${NC} $1"; }

FAILURES=0

echo "=== PDF Editor GUI Test Suite ==="
echo "App: $APP_PATH"
echo ""

# Ensure app exists
if [ ! -d "$APP_PATH" ]; then
    echo "Building app first..."
    "$SCRIPT_DIR/build-app.sh"
fi

# Ensure test PDF exists
if [ ! -f "$TEST_PDF" ]; then
    curl -sL "https://pdfobject.com/pdf/sample.pdf" -o "$TEST_PDF"
fi

# Test 1: App launches
info "Test 1: App launches..."
open "$APP_PATH"
sleep 2

# Check if app is running
if pgrep -x "pdf-editor" > /dev/null; then
    pass "App launched"
else
    fail "App did not launch"
    exit 1
fi

# Take screenshot
screencapture -x "$SCREENSHOT_DIR/01-app-launched.png"
info "Screenshot saved: 01-app-launched.png"

# Test 2: Window is visible
info "Test 2: Window visible..."
WINDOW_COUNT=$(osascript -e 'tell application "System Events" to count windows of process "pdf-editor"' 2>/dev/null || echo "0")
if [ "$WINDOW_COUNT" -gt 0 ]; then
    pass "Window is visible (count: $WINDOW_COUNT)"
else
    fail "No window visible"
fi

# Test 3: Open PDF via command line (simulate)
info "Test 3: Opening PDF..."
# Kill current instance
pkill -x "pdf-editor" 2>/dev/null || true
sleep 1

# Start with PDF
"$APP_PATH/Contents/MacOS/pdf-editor" "$TEST_PDF" &
APP_PID=$!
sleep 3

screencapture -x "$SCREENSHOT_DIR/02-pdf-opened.png"
info "Screenshot saved: 02-pdf-opened.png"

# Check if still running (didn't crash)
if ps -p $APP_PID > /dev/null 2>&1; then
    pass "App still running with PDF"
else
    fail "App crashed when opening PDF"
fi

# Cleanup
info "Cleaning up..."
kill $APP_PID 2>/dev/null || true
sleep 1

echo ""
echo "=== GUI Test Summary ==="
echo "Screenshots saved to: $SCREENSHOT_DIR"
if [ $FAILURES -eq 0 ]; then
    echo -e "${GREEN}All GUI tests passed!${NC}"
else
    echo -e "${RED}$FAILURES GUI test(s) failed${NC}"
fi

echo ""
echo "Manual verification needed:"
echo "  1. Check $SCREENSHOT_DIR/01-app-launched.png - should show grey window"
echo "  2. Check $SCREENSHOT_DIR/02-pdf-opened.png - should show PDF content"
