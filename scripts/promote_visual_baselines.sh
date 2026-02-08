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

promote_suite() {
  local suite="$1"
  local candidate_dir="tests/visual/candidates/${PLATFORM}/${suite}"
  local baseline_dir="tests/visual/baselines/${PLATFORM}/${suite}"

  if [[ ! -d "${candidate_dir}" ]]; then
    echo "Skipping ${suite} baseline promotion: ${candidate_dir} not found."
    return
  fi

  mkdir -p "${baseline_dir}"
  cp "${candidate_dir}"/*.png "${baseline_dir}/"
  echo "Promoted ${PLATFORM} ${suite} visuals into ${baseline_dir}"
}

promote_suite "editor"
promote_suite "settings"
