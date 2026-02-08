#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

bash scripts/prepare_macos_tahoe_icons.sh

cargo bundle --format osx --package butterpaper "$@"

APP_NAME="ButterPaper.app"
if [[ -d "target/release/bundle/osx/$APP_NAME" ]]; then
  APP_PATH="target/release/bundle/osx/$APP_NAME"
elif [[ -d "target/debug/bundle/osx/$APP_NAME" ]]; then
  APP_PATH="target/debug/bundle/osx/$APP_NAME"
else
  echo "error: bundled app not found under target/*/bundle/osx/$APP_NAME" >&2
  exit 1
fi

bash scripts/apply_macos_tahoe_bundle_icons.sh "$APP_PATH"

echo "Tahoe-ready bundle:"
echo "  $APP_PATH"
