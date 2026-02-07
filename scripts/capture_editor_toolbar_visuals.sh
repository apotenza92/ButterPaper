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
CANDIDATE_DIR="tests/visual/candidates/${PLATFORM}/editor"
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
  local -a command=(cargo run -p butterpaper -- "$@" \
    --window-title ButterPaper \
    --screenshot "${CANDIDATE_DIR}/${output_name}.png" \
    --screenshot-delay "${SCREENSHOT_DELAY_MS:-1600}")

  if [[ "${#RUN_PREFIX[@]}" -gt 0 ]]; then
    "${RUN_PREFIX[@]}" "${command[@]}"
  else
    "${command[@]}"
  fi
}

capture_with_fit() {
  local output_name="$1"
  local fit_mode="$2"
  shift 2
  local -a command=(cargo run -p butterpaper -- "$@" \
    --window-title ButterPaper \
    --screenshot "${CANDIDATE_DIR}/${output_name}.png" \
    --screenshot-delay "${SCREENSHOT_DELAY_MS:-1600}")

  if [[ "${#RUN_PREFIX[@]}" -gt 0 ]]; then
    BUTTERPAPER_VISUAL_FIT_MODE="${fit_mode}" "${RUN_PREFIX[@]}" "${command[@]}"
  else
    BUTTERPAPER_VISUAL_FIT_MODE="${fit_mode}" "${command[@]}"
  fi
}

capture "editor-empty"
capture "editor-loaded-default" tests/fixtures/small.pdf
capture_with_fit "editor-fit-page-active" "page" tests/fixtures/small.pdf
capture_with_fit "editor-fit-width-active" "width" tests/fixtures/small.pdf

echo "Captured editor visuals in ${CANDIDATE_DIR}"
