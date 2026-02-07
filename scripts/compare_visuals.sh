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
BASELINE_DIR="tests/visual/baselines/${PLATFORM}/editor"
CANDIDATE_DIR="tests/visual/candidates/${PLATFORM}/editor"

SCENARIOS=(
  "editor-empty"
  "editor-loaded-default"
  "editor-fit-page-active"
  "editor-fit-width-active"
)

status=0

for scenario in "${SCENARIOS[@]}"; do
  baseline="${BASELINE_DIR}/${scenario}.png"
  candidate="${CANDIDATE_DIR}/${scenario}.png"

  if [[ ! -f "${candidate}" ]]; then
    echo "Missing candidate image: ${candidate}" >&2
    status=1
    continue
  fi

  if [[ ! -f "${baseline}" ]]; then
    echo "Missing baseline image: ${baseline}" >&2
    status=1
    continue
  fi

  if ! cmp -s "${baseline}" "${candidate}"; then
    echo "Visual diff detected for ${scenario}" >&2
    status=1
  fi
done

if [[ "${status}" -ne 0 ]]; then
  echo "Visual comparison failed for platform ${PLATFORM}." >&2
  exit "${status}"
fi

echo "Visual comparison passed for platform ${PLATFORM}."
