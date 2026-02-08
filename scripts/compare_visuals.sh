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

EDITOR_SCENARIOS=(
  "editor-empty"
  "editor-loaded-default"
  "editor-fit-page-active"
  "editor-fit-width-active"
)

SETTINGS_SCENARIOS=(
  "appearance"
)

status=0

compare_suite() {
  local suite="$1"
  local required="$2"
  shift 2
  local -a scenarios=("$@")
  local baseline_dir="tests/visual/baselines/${PLATFORM}/${suite}"
  local candidate_dir="tests/visual/candidates/${PLATFORM}/${suite}"

  if [[ "${required}" != "1" && ! -d "${baseline_dir}" && ! -d "${candidate_dir}" ]]; then
    echo "Skipping ${suite} visual comparison for ${PLATFORM} (no baselines/candidates)."
    return
  fi

  for scenario in "${scenarios[@]}"; do
    local baseline="${baseline_dir}/${scenario}.png"
    local candidate="${candidate_dir}/${scenario}.png"

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
      echo "Visual diff detected for ${suite}/${scenario}" >&2
      status=1
    fi
  done
}

compare_suite "editor" "1" "${EDITOR_SCENARIOS[@]}"
compare_suite "settings" "0" "${SETTINGS_SCENARIOS[@]}"

if [[ "${status}" -ne 0 ]]; then
  echo "Visual comparison failed for platform ${PLATFORM}." >&2
  exit "${status}"
fi

echo "Visual comparison passed for platform ${PLATFORM}."
