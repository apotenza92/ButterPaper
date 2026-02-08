#!/usr/bin/env python3
"""Validate generated ButterPaper app icon assets."""

from __future__ import annotations

import argparse
import struct
import sys
import zlib
from pathlib import Path


EXPECTED_PNG_SIZES = [16, 24, 32, 40, 48, 64, 128, 256, 512, 1024]
EXPECTED_ICO_SIZES = [16, 24, 32, 40, 48, 64, 128, 256]


def fail(message: str) -> None:
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def parse_ico_sizes(path: Path) -> list[int]:
    data = path.read_bytes()
    if len(data) < 6:
        fail(f"ICO file too short: {path}")

    reserved, kind, count = struct.unpack_from("<HHH", data, 0)
    if reserved != 0 or kind != 1:
        fail(f"Invalid ICO header in {path}")

    expected_len = 6 + (count * 16)
    if len(data) < expected_len:
        fail(f"ICO directory truncated in {path}")

    sizes: list[int] = []
    for idx in range(count):
        offset = 6 + (idx * 16)
        width, height = struct.unpack_from("<BB", data, offset)
        width = 256 if width == 0 else width
        height = 256 if height == 0 else height
        if width != height:
            fail(f"ICO entry {idx} is not square ({width}x{height}) in {path}")
        sizes.append(width)
    return sizes


def paeth_predictor(a: int, b: int, c: int) -> int:
    p = a + b - c
    pa = abs(p - a)
    pb = abs(p - b)
    pc = abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    if pb <= pc:
        return b
    return c


def decode_png_rgba_alpha_bounds(path: Path) -> tuple[int, int, float, float]:
    data = path.read_bytes()
    if len(data) < 8 or data[:8] != b"\x89PNG\r\n\x1a\n":
        fail(f"Not a PNG file: {path}")

    cursor = 8
    width = height = None
    bit_depth = color_type = None
    idat_chunks: list[bytes] = []

    while cursor + 8 <= len(data):
        length = struct.unpack_from(">I", data, cursor)[0]
        cursor += 4
        chunk_type = data[cursor : cursor + 4]
        cursor += 4
        chunk_data = data[cursor : cursor + length]
        cursor += length
        cursor += 4  # CRC

        if chunk_type == b"IHDR":
            width, height, bit_depth, color_type, comp, filt, interlace = struct.unpack_from(
                ">IIBBBBB", chunk_data, 0
            )
            if comp != 0 or filt != 0 or interlace != 0:
                fail(f"Unsupported PNG encoding options in {path}")
        elif chunk_type == b"IDAT":
            idat_chunks.append(chunk_data)
        elif chunk_type == b"IEND":
            break

    if width is None or height is None or bit_depth is None or color_type is None:
        fail(f"Missing PNG header chunks in {path}")
    if bit_depth != 8:
        fail(f"Unsupported PNG bit depth ({bit_depth}) in {path}")

    channels_by_color_type = {0: 1, 2: 3, 4: 2, 6: 4}
    channels = channels_by_color_type.get(color_type)
    if channels is None:
        fail(f"Unsupported PNG color type ({color_type}) in {path}")

    payload = zlib.decompress(b"".join(idat_chunks))
    bytes_per_pixel = channels
    row_bytes = width * bytes_per_pixel
    expected = (row_bytes + 1) * height
    if len(payload) != expected:
        fail(f"Unexpected decompressed PNG size in {path}")

    rows: list[bytes] = []
    prev_row = bytes(row_bytes)
    pos = 0
    for _ in range(height):
        filt = payload[pos]
        pos += 1
        raw = payload[pos : pos + row_bytes]
        pos += row_bytes

        recon = bytearray(row_bytes)
        if filt == 0:
            recon[:] = raw
        elif filt == 1:
            for i in range(row_bytes):
                left = recon[i - bytes_per_pixel] if i >= bytes_per_pixel else 0
                recon[i] = (raw[i] + left) & 0xFF
        elif filt == 2:
            for i in range(row_bytes):
                recon[i] = (raw[i] + prev_row[i]) & 0xFF
        elif filt == 3:
            for i in range(row_bytes):
                left = recon[i - bytes_per_pixel] if i >= bytes_per_pixel else 0
                up = prev_row[i]
                recon[i] = (raw[i] + ((left + up) // 2)) & 0xFF
        elif filt == 4:
            for i in range(row_bytes):
                left = recon[i - bytes_per_pixel] if i >= bytes_per_pixel else 0
                up = prev_row[i]
                up_left = prev_row[i - bytes_per_pixel] if i >= bytes_per_pixel else 0
                recon[i] = (raw[i] + paeth_predictor(left, up, up_left)) & 0xFF
        else:
            fail(f"Unsupported PNG filter type ({filt}) in {path}")

        rows.append(bytes(recon))
        prev_row = bytes(recon)

    min_x = width
    min_y = height
    max_x = -1
    max_y = -1

    for y, row in enumerate(rows):
        for x in range(width):
            if color_type in (0, 2):
                alpha = 255
            elif color_type == 4:
                alpha = row[(x * 2) + 1]
            else:  # color_type == 6
                alpha = row[(x * 4) + 3]
            if alpha > 0:
                min_x = min(min_x, x)
                min_y = min(min_y, y)
                max_x = max(max_x, x)
                max_y = max(max_y, y)

    if max_x < min_x or max_y < min_y:
        fail(f"PNG appears fully transparent: {path}")

    coverage_w = (max_x - min_x + 1) / width
    coverage_h = (max_y - min_y + 1) / height
    return width, height, coverage_w, coverage_h


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--icons-dir",
        type=Path,
        default=Path("crates/gpui-app/assets/app-icons"),
        help="Directory containing generated icon assets.",
    )
    parser.add_argument(
        "--coverage-threshold",
        type=float,
        default=0.88,
        help="Minimum acceptable non-transparent coverage per axis for 1024 PNG.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    icons_dir: Path = args.icons_dir
    threshold = args.coverage_threshold

    required_files = [icons_dir / "butterpaper-icon.ico", icons_dir / "butterpaper-icon.icns"]
    required_files += [icons_dir / f"butterpaper-icon-{size}.png" for size in EXPECTED_PNG_SIZES]

    missing = [path for path in required_files if not path.exists()]
    if missing:
        fail("Missing icon assets:\n" + "\n".join(str(path) for path in missing))

    ico_sizes = sorted(parse_ico_sizes(icons_dir / "butterpaper-icon.ico"))
    if ico_sizes != EXPECTED_ICO_SIZES:
        fail(
            "ICO size set mismatch. expected="
            f"{EXPECTED_ICO_SIZES} actual={ico_sizes}"
        )

    png_1024 = icons_dir / "butterpaper-icon-1024.png"
    width, height, cov_w, cov_h = decode_png_rgba_alpha_bounds(png_1024)
    if width != 1024 or height != 1024:
        fail(f"Expected 1024x1024 PNG, got {width}x{height}: {png_1024}")
    if cov_w < threshold or cov_h < threshold:
        fail(
            "Icon coverage below threshold "
            f"({threshold:.2f}). got width={cov_w:.3f}, height={cov_h:.3f}"
        )

    print(f"Validated icon assets in {icons_dir}")
    print(f"ICO sizes: {ico_sizes}")
    print(f"1024 PNG alpha coverage: width={cov_w:.3f}, height={cov_h:.3f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
