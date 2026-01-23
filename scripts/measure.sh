#!/bin/bash
# Pixel measurement tool using macOS sips
# Usage:
#   ./measure.sh image.png info          # Get image dimensions
#   ./measure.sh image.png center X Y W H  # Calculate center of a region

IMAGE="$1"
CMD="$2"

if [ -z "$IMAGE" ] || [ ! -f "$IMAGE" ]; then
    echo "Usage: $0 <image.png> <command> [args...]"
    echo "Commands:"
    echo "  info                  - Get image dimensions"
    echo "  center X Y W H        - Calculate center of region (in 2x pixels)"
    echo "  to-points PX PY       - Convert 2x pixels to points"
    exit 1
fi

case "$CMD" in
    info)
        W=$(sips -g pixelWidth "$IMAGE" | tail -1 | awk '{print $2}')
        H=$(sips -g pixelHeight "$IMAGE" | tail -1 | awk '{print $2}')
        echo "{"
        echo "  \"pixels\": {\"width\": $W, \"height\": $H},"
        echo "  \"points\": {\"width\": $((W/2)), \"height\": $((H/2))}"
        echo "}"
        ;;
    center)
        X="$3"
        Y="$4"
        W="$5"
        H="$6"
        if [ -z "$W" ] || [ -z "$H" ]; then
            echo "Usage: $0 $IMAGE center X Y W H (all in 2x pixels)"
            exit 1
        fi
        CX=$((X + W/2))
        CY=$((Y + H/2))
        echo "{"
        echo "  \"region_pixel\": {\"x\": $X, \"y\": $Y, \"w\": $W, \"h\": $H},"
        echo "  \"center_pixel\": {\"x\": $CX, \"y\": $CY},"
        echo "  \"center_point\": {\"x\": $((CX/2)), \"y\": $((CY/2))}"
        echo "}"
        ;;
    to-points)
        PX="$3"
        PY="$4"
        if [ -z "$PX" ] || [ -z "$PY" ]; then
            echo "Usage: $0 $IMAGE to-points PX PY"
            exit 1
        fi
        echo "{"
        echo "  \"pixel\": {\"x\": $PX, \"y\": $PY},"
        echo "  \"point\": {\"x\": $((PX/2)), \"y\": $((PY/2))}"
        echo "}"
        ;;
    *)
        echo "Unknown command: $CMD"
        exit 1
        ;;
esac
