#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

CHANNEL="${1:-stable}"
if [[ "$CHANNEL" != "stable" && "$CHANNEL" != "beta" ]]; then
  echo "usage: $0 [stable|beta]" >&2
  exit 1
fi

if ! command -v xcrun >/dev/null 2>&1; then
  echo "error: xcrun not found; install Xcode command line tools." >&2
  exit 1
fi
if ! command -v rsvg-convert >/dev/null 2>&1; then
  echo "error: rsvg-convert not found; required for icon PNG generation." >&2
  exit 1
fi

# Ensure required PNG/ICO/ICNS assets are up to date first.
if [[ "$CHANNEL" == "beta" ]]; then
  python3 scripts/generate_app_icons.py \
    --variant beta \
    --svg crates/gpui-app/assets/butterpaper-icon-beta.svg
else
  python3 scripts/generate_app_icons.py
fi

ASSET_ROOT="crates/gpui-app/assets/macos"
XCASSETS_DIR="$ASSET_ROOT/Assets.xcassets"
APPICONSET_DIR="$XCASSETS_DIR/AppIcon.appiconset"
TMP_DIR="$(mktemp -d)"
OUT_DIR="$TMP_DIR/build"
PARTIAL_PLIST="$OUT_DIR/partial-info.plist"
trap 'rm -rf "$TMP_DIR"' EXIT

mkdir -p "$APPICONSET_DIR" "$OUT_DIR"

cat > "$XCASSETS_DIR/Contents.json" <<'JSON'
{
  "info" : {
    "author" : "xcode",
    "version" : 1
  }
}
JSON

cat > "$APPICONSET_DIR/Contents.json" <<'JSON'
{
  "images" : [
    { "filename" : "icon_16x16.png", "idiom" : "mac", "scale" : "1x", "size" : "16x16" },
    { "filename" : "icon_16x16@2x.png", "idiom" : "mac", "scale" : "2x", "size" : "16x16" },
    { "filename" : "icon_32x32.png", "idiom" : "mac", "scale" : "1x", "size" : "32x32" },
    { "filename" : "icon_32x32@2x.png", "idiom" : "mac", "scale" : "2x", "size" : "32x32" },
    { "filename" : "icon_128x128.png", "idiom" : "mac", "scale" : "1x", "size" : "128x128" },
    { "filename" : "icon_128x128@2x.png", "idiom" : "mac", "scale" : "2x", "size" : "128x128" },
    { "filename" : "icon_256x256.png", "idiom" : "mac", "scale" : "1x", "size" : "256x256" },
    { "filename" : "icon_256x256@2x.png", "idiom" : "mac", "scale" : "2x", "size" : "256x256" },
    { "filename" : "icon_512x512.png", "idiom" : "mac", "scale" : "1x", "size" : "512x512" },
    { "filename" : "icon_512x512@2x.png", "idiom" : "mac", "scale" : "2x", "size" : "512x512" }
  ],
  "info" : {
    "author" : "xcode",
    "version" : 1
  }
}
JSON

ICON_PREFIX="butterpaper-icon"
if [[ "$CHANNEL" == "beta" ]]; then
  ICON_PREFIX="butterpaper-icon-beta"
fi

cp "crates/gpui-app/assets/app-icons/${ICON_PREFIX}-16.png" "$APPICONSET_DIR/icon_16x16.png"
cp "crates/gpui-app/assets/app-icons/${ICON_PREFIX}-32.png" "$APPICONSET_DIR/icon_16x16@2x.png"
cp "crates/gpui-app/assets/app-icons/${ICON_PREFIX}-32.png" "$APPICONSET_DIR/icon_32x32.png"
cp "crates/gpui-app/assets/app-icons/${ICON_PREFIX}-64.png" "$APPICONSET_DIR/icon_32x32@2x.png"
cp "crates/gpui-app/assets/app-icons/${ICON_PREFIX}-128.png" "$APPICONSET_DIR/icon_128x128.png"
cp "crates/gpui-app/assets/app-icons/${ICON_PREFIX}-256.png" "$APPICONSET_DIR/icon_128x128@2x.png"
cp "crates/gpui-app/assets/app-icons/${ICON_PREFIX}-256.png" "$APPICONSET_DIR/icon_256x256.png"
cp "crates/gpui-app/assets/app-icons/${ICON_PREFIX}-512.png" "$APPICONSET_DIR/icon_256x256@2x.png"
cp "crates/gpui-app/assets/app-icons/${ICON_PREFIX}-512.png" "$APPICONSET_DIR/icon_512x512.png"
cp "crates/gpui-app/assets/app-icons/${ICON_PREFIX}-1024.png" "$APPICONSET_DIR/icon_512x512@2x.png"

xcrun actool \
  --compile "$OUT_DIR" \
  --platform macosx \
  --target-device mac \
  --minimum-deployment-target 13.0 \
  --app-icon AppIcon \
  --output-partial-info-plist "$PARTIAL_PLIST" \
  "$XCASSETS_DIR"

if [[ ! -f "$OUT_DIR/Assets.car" ]]; then
  echo "error: actool did not produce Assets.car" >&2
  exit 1
fi

CAR_NAME="Assets.car"
if [[ "$CHANNEL" == "beta" ]]; then
  CAR_NAME="Assets-beta.car"
fi

cp "$OUT_DIR/Assets.car" "$ASSET_ROOT/$CAR_NAME"

echo "Generated Tahoe-compatible macOS icon catalog:"
echo "  $ASSET_ROOT/$CAR_NAME"
echo
echo "Optional: Add an Icon Composer source at:"
echo "  crates/gpui-app/assets/macos/AppIcon.icon"
echo "and rebuild your native Xcode target for full Tahoe appearance variants."
