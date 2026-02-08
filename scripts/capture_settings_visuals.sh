#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

detect_platform() {
  local os
  os="$(uname -s)"
  case "$os" in
    Darwin) echo "darwin" ;;
    Linux) echo "linux" ;;
    MINGW*|MSYS*|CYGWIN*|Windows_NT) echo "windows" ;;
    *)
      echo "Unsupported platform: ${os}" >&2
      exit 1
      ;;
  esac
}

PLATFORM="${VISUAL_PLATFORM:-$(detect_platform)}"
CANDIDATE_DIR="tests/visual/candidates/${PLATFORM}/settings"
mkdir -p "${CANDIDATE_DIR}"

if [[ "${SKIP_PDFIUM_SETUP:-0}" != "1" ]]; then
  ./scripts/setup_pdfium.sh
fi

declare -a RUN_PREFIX=()
if [[ "${PLATFORM}" == "linux" ]] && command -v xvfb-run >/dev/null 2>&1; then
  RUN_PREFIX=(xvfb-run -a)
fi

capture() {
  local output_name="$1"
  shift
  local -a command=(cargo run -p butterpaper -- \
    --settings \
    --window-title Settings \
    --screenshot "${CANDIDATE_DIR}/${output_name}.png" \
    --screenshot-delay "${SCREENSHOT_DELAY_MS:-1800}" \
    "$@")

  if [[ "${#RUN_PREFIX[@]}" -gt 0 ]]; then
    "${RUN_PREFIX[@]}" "${command[@]}"
  else
    "${command[@]}"
  fi
}

# settings-appearance
capture "appearance"

echo "Captured settings visuals in ${CANDIDATE_DIR}"
