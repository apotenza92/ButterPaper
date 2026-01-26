#!/usr/bin/env bash
# Capture a specific window by process name
# Usage: ./capture-window.sh "butterpaper" output.png

PROCESS_NAME="${1:-butterpaper}"
OUTPUT="${2:-/tmp/window-capture.png}"

# Get window ID using CGWindowListCopyWindowInfo via Python
WINDOW_ID=$(python3 << EOF
import Quartz
windows = Quartz.CGWindowListCopyWindowInfo(
    Quartz.kCGWindowListOptionOnScreenOnly,
    Quartz.kCGNullWindowID
)
for w in windows:
    owner = w.get('kCGWindowOwnerName', '')
    if '$PROCESS_NAME' in owner.lower():
        print(w.get('kCGWindowNumber', ''))
        break
EOF
)

if [ -z "$WINDOW_ID" ]; then
    echo "Window not found for: $PROCESS_NAME"
    exit 1
fi

echo "Capturing window ID: $WINDOW_ID"
screencapture -l"$WINDOW_ID" -x "$OUTPUT"

if [ -f "$OUTPUT" ]; then
    echo "Saved: $OUTPUT"
    # Get dimensions
    file "$OUTPUT"
else
    echo "Capture failed (Screen Recording permission needed)"
    exit 1
fi
