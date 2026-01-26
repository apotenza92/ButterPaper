#!/bin/bash
# Test script to toggle macOS dark/light mode and capture screenshots

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
SCREENSHOT_DIR="$PROJECT_DIR/test-screenshots"
rm -rf "$SCREENSHOT_DIR"
mkdir -p "$SCREENSHOT_DIR"

echo "=== Theme Switch Test ==="

# Set to light mode as baseline
echo "Setting to LIGHT mode..."
osascript -e 'tell application "System Events" to tell appearance preferences to set dark mode to false'
sleep 3
echo "Taking screenshot (press Space on the app window, or wait)..."
screencapture -x "$SCREENSHOT_DIR/01-light.png"
echo "Saved: 01-light.png"

echo ""
echo "Switching to DARK mode..."
osascript -e 'tell application "System Events" to tell appearance preferences to set dark mode to true'
sleep 3
screencapture -x "$SCREENSHOT_DIR/02-dark.png"
echo "Saved: 02-dark.png"

echo ""
echo "Switching to LIGHT mode..."
osascript -e 'tell application "System Events" to tell appearance preferences to set dark mode to false'
sleep 3
screencapture -x "$SCREENSHOT_DIR/03-light-again.png"
echo "Saved: 03-light-again.png"

echo ""
echo "=== Done ==="
ls -la "$SCREENSHOT_DIR"
