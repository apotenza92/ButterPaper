# ButterPaper

ButterPaper is being rebuilt as a Rust-native desktop PDF app.

## Stack

- UI: `gpui`
- Core: Rust workspace crates (`render`, `gpui-app`)
- Testing: GPUI app tests (`#[gpui::test]`) + workspace checks

## Workspace commands

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Run

```bash
./scripts/setup_pdfium.sh
cargo run -p butterpaper -- path/to/file.pdf
```

If PDF loading still fails, point directly at your runtime library:

```bash
BUTTERPAPER_PDFIUM_LIB=/absolute/path/to/libpdfium.dylib cargo run -p butterpaper -- path/to/file.pdf
```

## App Icons (macOS, Windows, Linux)

ButterPaper uses `crates/gpui-app/assets/butterpaper-icon.svg` as the source of truth.

Generate platform icon assets:

```bash
python3 scripts/generate_app_icons.py
```

Generated outputs:

- `crates/gpui-app/assets/app-icons/butterpaper-icon.icns` (macOS)
- `crates/gpui-app/assets/app-icons/butterpaper-icon.ico` (Windows)
- `crates/gpui-app/assets/app-icons/butterpaper-icon-*.png` (Linux desktop sizes)

Notes:

- `rsvg-convert` is required for PNG rendering.
- `iconutil` is used for `.icns` generation when available (macOS).
- Icon artwork is intentionally platform-neutral (no baked rounded container) so each OS can apply
  its own icon shape treatment.

Validate icon outputs (recommended for local checks and CI):

```bash
python3 scripts/validate_app_icons.py
```

### macOS Tahoe Appearance Icons

For Tahoe-style icon appearance integration (`CFBundleIconName` + `Assets.car`):

```bash
bash scripts/prepare_macos_tahoe_icons.sh
```

After building a `.app` bundle (for example via `cargo bundle --format osx`), patch it:

```bash
bash scripts/apply_macos_tahoe_bundle_icons.sh /path/to/ButterPaper.app
```

This enables the native macOS icon catalog path used by Appearance icon settings.
For full Tahoe visual variants, provide an Icon Composer source (`AppIcon.icon`) in a native Xcode macOS target.

Important behavior on Tahoe:

- `cargo run -p butterpaper` is dev-mode execution and may not reflect final Appearance icon styling.
- Appearance-accurate validation should use a bundled app with `Assets.car` + `CFBundleIconName`.
- Use `bash scripts/bundle_macos_tahoe.sh --release` for that end-to-end flow.

One-step Tahoe bundle flow:

```bash
bash scripts/bundle_macos_tahoe.sh --release
```
