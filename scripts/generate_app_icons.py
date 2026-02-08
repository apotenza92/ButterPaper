#!/usr/bin/env python3
"""Generate ButterPaper app icons for macOS, Windows, and Linux.

Requirements:
- rsvg-convert (required)
- iconutil (optional; required only for macOS .icns generation)
"""

from __future__ import annotations

import argparse
import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path


PNG_SIZES = [16, 24, 32, 40, 48, 64, 128, 256, 512, 1024]
ICO_SIZES = [16, 24, 32, 40, 48, 64, 128, 256]


def run(cmd: list[str]) -> None:
    subprocess.run(cmd, check=True)


def render_png(svg_path: Path, size: int, output_path: Path) -> None:
    run(
        [
            "rsvg-convert",
            "-w",
            str(size),
            "-h",
            str(size),
            str(svg_path),
            "-o",
            str(output_path),
        ]
    )


def png_dimensions(png_path: Path) -> tuple[int, int]:
    data = png_path.read_bytes()
    if len(data) < 24 or data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{png_path} is not a valid PNG")
    width, height = struct.unpack(">II", data[16:24])
    return width, height


def build_ico(png_paths: list[Path], ico_path: Path) -> None:
    image_payloads = [p.read_bytes() for p in png_paths]

    entries = bytearray()
    offset = 6 + (16 * len(image_payloads))
    payload_blob = bytearray()

    for payload, png_path in zip(image_payloads, png_paths):
        width, height = png_dimensions(png_path)
        if width != height:
            raise ValueError(f"{png_path} is not square ({width}x{height})")
        if width > 256 or height > 256:
            raise ValueError(f"{png_path} exceeds ICO max size (256x256)")

        entry_width = 0 if width == 256 else width
        entry_height = 0 if height == 256 else height

        entries.extend(
            struct.pack(
                "<BBBBHHII",
                entry_width,
                entry_height,
                0,  # color count (0 = truecolor)
                0,  # reserved
                1,  # planes
                32,  # bit depth hint
                len(payload),
                offset,
            )
        )
        payload_blob.extend(payload)
        offset += len(payload)

    header = struct.pack("<HHH", 0, 1, len(image_payloads))
    ico_path.write_bytes(header + entries + payload_blob)


def generate_icns(out_dir: Path, size_to_png: dict[int, Path]) -> bool:
    if shutil.which("iconutil") is None:
        return False

    with tempfile.TemporaryDirectory() as tmp:
        iconset = Path(tmp) / "butterpaper.iconset"
        iconset.mkdir(parents=True, exist_ok=True)

        mapping = {
            "icon_16x16.png": 16,
            "icon_16x16@2x.png": 32,
            "icon_32x32.png": 32,
            "icon_32x32@2x.png": 64,
            "icon_128x128.png": 128,
            "icon_128x128@2x.png": 256,
            "icon_256x256.png": 256,
            "icon_256x256@2x.png": 512,
            "icon_512x512.png": 512,
            "icon_512x512@2x.png": 1024,
        }

        for filename, size in mapping.items():
            shutil.copy2(size_to_png[size], iconset / filename)

        run(
            [
                "iconutil",
                "-c",
                "icns",
                str(iconset),
                "-o",
                str(out_dir / "butterpaper-icon.icns"),
            ]
        )

    return True


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--svg",
        type=Path,
        default=Path("crates/gpui-app/assets/butterpaper-icon.svg"),
        help="Path to source SVG.",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path("crates/gpui-app/assets/app-icons"),
        help="Output directory for generated icon assets.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    svg_path = args.svg
    out_dir = args.out_dir

    if shutil.which("rsvg-convert") is None:
        print("error: rsvg-convert is required but was not found on PATH", file=sys.stderr)
        return 1
    if not svg_path.exists():
        print(f"error: source SVG not found: {svg_path}", file=sys.stderr)
        return 1

    out_dir.mkdir(parents=True, exist_ok=True)

    size_to_png: dict[int, Path] = {}
    for size in PNG_SIZES:
        png_path = out_dir / f"butterpaper-icon-{size}.png"
        render_png(svg_path, size, png_path)
        size_to_png[size] = png_path

    ico_sources = [size_to_png[size] for size in ICO_SIZES]
    build_ico(ico_sources, out_dir / "butterpaper-icon.ico")

    icns_generated = generate_icns(out_dir, size_to_png)

    print(f"Generated PNG sizes: {', '.join(str(s) for s in PNG_SIZES)}")
    print(f"Generated Windows ICO: {out_dir / 'butterpaper-icon.ico'}")
    if icns_generated:
        print(f"Generated macOS ICNS: {out_dir / 'butterpaper-icon.icns'}")
    else:
        print("Skipped ICNS generation (iconutil not found).")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
