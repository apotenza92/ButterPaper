# Changelog

## [0.0.2] - 2026-02-10

### Added
- Beta channel app icon variants for macOS, Windows, and Linux (distinct from stable).
- Auto-update system (all platforms) backed by GitHub Releases:
  - In-app periodic update check + “Update and restart” banner.
  - Packaged `butterpaper-updater` helper to download/apply updates and relaunch.
- Changelog-driven GitHub release notes.

### Changed
- Release packaging now builds/embeds beta icons in platform installers/bundles.
- Beta artifacts for macOS and Linux are built with `--features beta` so beta installs track the beta update channel.
- Linux beta deb/rpm now ship as a distinct package/binary/desktop entry (`butterpaper-beta`) so it can coexist with stable.

## [0.0.1] - 2026-02-05

### Added
- Initial GPUI desktop app scaffold and release pipeline.
