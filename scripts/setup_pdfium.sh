#!/usr/bin/env bash
set -euo pipefail

# Downloads a compatible PDFium runtime for local ButterPaper development.
# Default version matches pdfium-render's default API target.
PDFIUM_VERSION="${PDFIUM_VERSION:-7543}"
RELEASE_TAG="chromium/${PDFIUM_VERSION}"

OS="$(uname -s)"
ARCH="$(uname -m)"

asset=""
platform_dir=""

case "${OS}" in
  Darwin)
    case "${ARCH}" in
      arm64|aarch64)
        asset="pdfium-mac-arm64.tgz"
        platform_dir="macos-aarch64"
        ;;
      x86_64)
        asset="pdfium-mac-x64.tgz"
        platform_dir="macos-x86_64"
        ;;
      *)
        echo "Unsupported macOS architecture: ${ARCH}" >&2
        exit 1
        ;;
    esac
    ;;
  Linux)
    case "${ARCH}" in
      x86_64)
        asset="pdfium-linux-x64.tgz"
        platform_dir="linux-x86_64"
        ;;
      aarch64|arm64)
        asset="pdfium-linux-arm64.tgz"
        platform_dir="linux-aarch64"
        ;;
      *)
        echo "Unsupported Linux architecture: ${ARCH}" >&2
        exit 1
        ;;
    esac
    ;;
  MINGW*|MSYS*|CYGWIN*|Windows_NT)
    case "${ARCH}" in
      x86_64|AMD64)
        asset="pdfium-win-x64.tgz"
        platform_dir="windows-x86_64"
        ;;
      arm64|aarch64)
        asset="pdfium-win-arm64.tgz"
        platform_dir="windows-aarch64"
        ;;
      *)
        echo "Unsupported Windows architecture: ${ARCH}" >&2
        exit 1
        ;;
    esac
    ;;
  *)
    echo "Unsupported OS: ${OS}" >&2
    exit 1
    ;;
esac

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_dir="${repo_root}/third_party/pdfium/${platform_dir}"
archive_path="${out_dir}/${asset}"

mkdir -p "${out_dir}"

url="https://github.com/bblanchon/pdfium-binaries/releases/download/${RELEASE_TAG//\//%2F}/${asset}"

echo "Downloading ${url}"
curl -fL "${url}" -o "${archive_path}"

echo "Extracting to ${out_dir}"
tar -xzf "${archive_path}" -C "${out_dir}"
rm -f "${archive_path}"

case "${OS}" in
  Darwin)
    lib_path="${out_dir}/lib/libpdfium.dylib"
    ;;
  Linux)
    lib_path="${out_dir}/lib/libpdfium.so"
    ;;
  *)
    lib_path="${out_dir}/bin/pdfium.dll"
    ;;
esac

if [[ ! -f "${lib_path}" ]]; then
  echo "PDFium download succeeded, but expected runtime not found at ${lib_path}" >&2
  exit 1
fi

echo "Installed PDFium runtime: ${lib_path}"
echo "ButterPaper will auto-discover this library on startup."
