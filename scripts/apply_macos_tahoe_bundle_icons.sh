#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [[ $# -ne 1 ]]; then
  echo "usage: $0 /path/to/ButterPaper.app" >&2
  exit 1
fi

APP_BUNDLE="$1"
PLIST_PATH="$APP_BUNDLE/Contents/Info.plist"
RESOURCES_DIR="$APP_BUNDLE/Contents/Resources"
ASSETS_CAR="crates/gpui-app/assets/macos/Assets.car"

if [[ ! -d "$APP_BUNDLE" ]]; then
  echo "error: app bundle not found: $APP_BUNDLE" >&2
  exit 1
fi
if [[ ! -f "$PLIST_PATH" ]]; then
  echo "error: Info.plist not found at $PLIST_PATH" >&2
  exit 1
fi

if [[ ! -f "$ASSETS_CAR" ]]; then
  echo "Assets.car missing; generating it now..."
  bash scripts/prepare_macos_tahoe_icons.sh
fi

mkdir -p "$RESOURCES_DIR"
cp "$ASSETS_CAR" "$RESOURCES_DIR/Assets.car"

plutil -replace CFBundleIconName -string "AppIcon" "$PLIST_PATH"
plutil -remove CFBundleIconFile "$PLIST_PATH" >/dev/null 2>&1 || true
plutil -remove CFBundleIconFiles "$PLIST_PATH" >/dev/null 2>&1 || true

if command -v codesign >/dev/null 2>&1; then
  # Ad-hoc re-sign keeps local execution valid after Info.plist/resource edits.
  codesign --force --deep --sign - "$APP_BUNDLE" >/dev/null 2>&1 || true
fi

echo "Patched bundle for Tahoe icon appearance:"
echo "  - Installed Assets.car"
echo "  - Set CFBundleIconName=AppIcon"
echo "  - Removed legacy CFBundleIconFile keys"
