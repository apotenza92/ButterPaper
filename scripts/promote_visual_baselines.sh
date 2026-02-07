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
BASELINE_DIR="tests/visual/baselines/${PLATFORM}/editor"

if [[ ! -d "${CANDIDATE_DIR}" ]]; then
  echo "Candidate directory not found: ${CANDIDATE_DIR}" >&2
  exit 1
fi

mkdir -p "${BASELINE_DIR}"
cp "${CANDIDATE_DIR}"/*.png "${BASELINE_DIR}/"

echo "Promoted ${PLATFORM} editor visuals into ${BASELINE_DIR}"
