# Tahoe Icon Pipeline Notes

## What Changed

- Added a generated cross-platform icon set at `crates/gpui-app/assets/app-icons/`.
- Added macOS asset-catalog generation (`Assets.car`) via:
  - `scripts/prepare_macos_tahoe_icons.sh`
- Added bundle patching for Tahoe icon appearance keys/resources via:
  - `scripts/apply_macos_tahoe_bundle_icons.sh`
- Added Windows executable icon embedding in `crates/gpui-app/build.rs`.
- Kept `.icns` fallback for non-Tahoe macOS flows.

## Runtime Behavior

- On macOS 26+ (Tahoe), runtime Dock icon override is disabled in `crates/gpui-app/src/macos.rs`.
- Reason: runtime `setApplicationIconImage` forces a static icon and can bypass system appearance icon styling.

## Packaging Contract

- Source of truth: `crates/gpui-app/assets/butterpaper-icon.svg`
- Generated artifacts:
  - `crates/gpui-app/assets/app-icons/butterpaper-icon.icns`
  - `crates/gpui-app/assets/app-icons/butterpaper-icon.ico`
  - `crates/gpui-app/assets/app-icons/butterpaper-icon-*.png`
  - `crates/gpui-app/assets/macos/Assets.car`

## Tahoe-Specific Constraint

- Full appearance variants in Tahoe depend on Apple Icon Composer assets in an Xcode-native packaging path.
- This repo now supports the native `Assets.car` + `CFBundleIconName` route and avoids runtime icon override on Tahoe.
