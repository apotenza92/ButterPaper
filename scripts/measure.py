#!/usr/bin/env python3
"""
Pixel measurement tool for UI automation.

Usage:
    # Get pixel color and position interactively
    python measure.py /path/to/screenshot.png
    
    # Find center of an element by color sampling a region
    python measure.py /path/to/screenshot.png --find-element 500,200,700,300
    
    # Output coordinates at specific pixel (for 2x retina, divide by 2)
    python measure.py /path/to/screenshot.png --point 1137,278
"""

import sys
import json
from PIL import Image

def get_point_info(img_path: str, x: int, y: int) -> dict:
    """Get info about a specific point in the image."""
    img = Image.open(img_path)
    width, height = img.size
    
    if x < 0 or x >= width or y < 0 or y >= height:
        return {"error": f"Point ({x}, {y}) out of bounds for {width}x{height} image"}
    
    pixel = img.getpixel((x, y))
    return {
        "pixel": {"x": x, "y": y},
        "point": {"x": x // 2, "y": y // 2},  # For 2x retina
        "color": {"r": pixel[0], "g": pixel[1], "b": pixel[2]},
        "hex": f"#{pixel[0]:02x}{pixel[1]:02x}{pixel[2]:02x}",
        "image_size": {"width": width, "height": height},
        "point_size": {"width": width // 2, "height": height // 2}
    }

def find_element_bounds(img_path: str, region: tuple) -> dict:
    """Find the bounds of a UI element within a region by detecting edges."""
    img = Image.open(img_path)
    x1, y1, x2, y2 = region
    
    # Sample the region
    pixels = []
    for y in range(y1, y2):
        row = []
        for x in range(x1, x2):
            row.append(img.getpixel((x, y))[:3])
        pixels.append(row)
    
    # Find bounding box of non-background pixels
    # Assume corners are background
    bg_color = pixels[0][0]
    
    min_x, min_y = x2, y2
    max_x, max_y = x1, y1
    
    for y_idx, row in enumerate(pixels):
        for x_idx, pixel in enumerate(row):
            if pixel != bg_color:
                abs_x = x1 + x_idx
                abs_y = y1 + y_idx
                min_x = min(min_x, abs_x)
                min_y = min(min_y, abs_y)
                max_x = max(max_x, abs_x)
                max_y = max(max_y, abs_y)
    
    if min_x > max_x:
        return {"error": "No element found in region"}
    
    center_x = (min_x + max_x) // 2
    center_y = (min_y + max_y) // 2
    
    return {
        "bounds_pixel": {"x": min_x, "y": min_y, "width": max_x - min_x, "height": max_y - min_y},
        "bounds_point": {"x": min_x // 2, "y": min_y // 2, "width": (max_x - min_x) // 2, "height": (max_y - min_y) // 2},
        "center_pixel": {"x": center_x, "y": center_y},
        "center_point": {"x": center_x // 2, "y": center_y // 2},
        "background_color": f"#{bg_color[0]:02x}{bg_color[1]:02x}{bg_color[2]:02x}"
    }

def scan_for_dropdowns(img_path: str) -> dict:
    """Scan image for dropdown-like UI elements (buttons with borders)."""
    img = Image.open(img_path)
    width, height = img.size
    
    # This is a simplified heuristic - looks for rectangular bordered elements
    # on the right side of the image (where dropdowns typically are in settings)
    results = []
    
    # Scan right third of image for potential dropdowns
    scan_x_start = width * 2 // 3
    
    # Look for horizontal lines that could be top/bottom of dropdowns
    # This is very basic - a real implementation would use edge detection
    
    return {
        "image_size": {"width": width, "height": height},
        "note": "Use --point or --find-element for precise measurements"
    }

def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    
    img_path = sys.argv[1]
    
    if "--point" in sys.argv:
        idx = sys.argv.index("--point")
        coords = sys.argv[idx + 1].split(",")
        x, y = int(coords[0]), int(coords[1])
        result = get_point_info(img_path, x, y)
        print(json.dumps(result, indent=2))
    
    elif "--find-element" in sys.argv:
        idx = sys.argv.index("--find-element")
        coords = [int(c) for c in sys.argv[idx + 1].split(",")]
        result = find_element_bounds(img_path, tuple(coords))
        print(json.dumps(result, indent=2))
    
    elif "--info" in sys.argv:
        img = Image.open(img_path)
        result = {
            "image_size": {"width": img.size[0], "height": img.size[1]},
            "point_size": {"width": img.size[0] // 2, "height": img.size[1] // 2}
        }
        print(json.dumps(result, indent=2))
    
    else:
        # Interactive mode hint
        img = Image.open(img_path)
        print(json.dumps({
            "image": img_path,
            "size_pixels": {"width": img.size[0], "height": img.size[1]},
            "size_points": {"width": img.size[0] // 2, "height": img.size[1] // 2},
            "usage": {
                "--point X,Y": "Get info at pixel coordinates",
                "--find-element X1,Y1,X2,Y2": "Find element bounds in region",
                "--info": "Get image dimensions"
            }
        }, indent=2))

if __name__ == "__main__":
    main()
