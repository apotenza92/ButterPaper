# Auto Update System (v1)

## Goal
Provide automatic update checks and one-click updates for ButterPaper across macOS, Windows, and Linux, for both stable and beta channels, without introducing a webview/JS runtime.

## High-Level Architecture
- `crates/update-core`
  - Fetches GitHub Releases metadata.
  - Selects the correct release + asset name for a given channel/platform/arch.
  - No UI and no installation logic.
- `crates/updater` (`butterpaper-updater` binary)
  - Applies updates per platform:
    - macOS: download `.zip`, extract, replace current `.app`, relaunch.
    - Windows: download NSIS installer, run it (optionally silent) after the app exits.
    - Linux: if running as AppImage (APPIMAGE set), replace AppImage and relaunch; otherwise fall back to opening the download URL.
- `crates/gpui-app`
  - Auto-checks in the background (24h interval) and surfaces an in-app update banner.
  - When the user clicks update, it spawns `butterpaper-updater apply ...` and quits.

## Channels
- Stable builds default to `stable`.
- Beta builds (compiled with `--features beta`) default to `beta`.
- Release artifacts are named deterministically and the updater selects by name:
  - Stable prefix: `ButterPaper-...`
  - Beta prefix: `ButterPaper-Beta-...`

## Packaging Requirements
To support in-place updates, packaged artifacts include the updater binary alongside the app:
- macOS: `ButterPaper.app/Contents/MacOS/butterpaper-updater`
- Windows: installed next to `ButterPaper.exe` as `butterpaper-updater.exe`
- Linux: shipped next to the executable inside AppImage and installed as `/usr/bin/butterpaper-updater` for deb/rpm

## Security Notes (future work)
v1 relies on HTTPS + GitHub Releases. For stronger integrity guarantees:
- publish a signed update manifest and verify signatures in the updater.
- consider pinning public keys in `update-core`.

