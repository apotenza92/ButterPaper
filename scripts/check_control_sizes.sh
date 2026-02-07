#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TARGET_FILES=(
  "crates/gpui-app/src/app/editor.rs"
  "crates/gpui-app/src/components/context_menu.rs"
  "crates/gpui-app/src/components/chrome.rs"
  "crates/gpui-app/src/components/dropdown.rs"
  "crates/gpui-app/src/components/nav_item.rs"
  "crates/gpui-app/src/components/popover_menu.rs"
  "crates/gpui-app/src/components/radio.rs"
  "crates/gpui-app/src/components/segmented_control.rs"
  "crates/gpui-app/src/components/slider.rs"
  "crates/gpui-app/src/components/tab.rs"
  "crates/gpui-app/src/components/toggle_switch.rs"
)

echo "Checking for ad-hoc px literals in interactive control surfaces..."
if rg -n 'px\([0-9]+(\.[0-9]+)?\)' "${TARGET_FILES[@]}"; then
  echo
  echo "Found hardcoded px() literals in standardized interactive control files."
  echo "Use shared size tokens/ButtonSize/ui::sizes constants instead."
  exit 1
fi

echo "Checking for deprecated size enums..."
if rg -n '\b(IconButtonSize|TextButtonSize|InputSize)\b' crates/gpui-app/src; then
  echo
  echo "Found deprecated size enums. Use components::ButtonSize."
  exit 1
fi

echo "Checking for legacy ButtonSize variants..."
if rg -n 'ButtonSize::(Sm|Md|Lg)\b' crates/gpui-app/src; then
  echo
  echo "Found legacy ButtonSize variant names. Use Large/Medium/Default/Compact/None."
  exit 1
fi

echo "Control sizing checks passed."
