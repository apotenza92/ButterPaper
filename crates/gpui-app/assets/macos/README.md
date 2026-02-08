# macOS Icon Assets

- `Assets.car` is the compiled macOS asset catalog used for Tahoe icon appearance integration.
- `Assets.xcassets/AppIcon.appiconset/` is generated from `../butterpaper-icon.svg`.
- Optional: place `AppIcon.icon` (from Apple Icon Composer) in this directory for native Tahoe variant authoring in an Xcode target.

Regenerate with:

```bash
bash scripts/prepare_macos_tahoe_icons.sh
```
