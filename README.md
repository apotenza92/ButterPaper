# ButterPaper

<img src="crates/gpui-app/assets/butterpaper-icon.svg" alt="ButterPaper icon" width="160" />

ButterPaper brings the classic tracing-paper workflow to PDFs. Layer ideas, mark up drawings, and iterate without losing the original intent.

## Why Butter Paper

Butter paper was the thin, translucent paper architects and drafties used long before digital tools. It was made for tracing and layering ideas over drawings without touching the originals. ButterPaper brings that same workflow into a fast, clean, open-source app.

## Build

```bash
./scripts/build-gpui.sh
```

Or with cargo:

```bash
cargo build --release -p butterpaper-gpui
```

## Run

```bash
./target/release/butterpaper path/to/file.pdf
```

## macOS bundle

```bash
./scripts/build-app.sh
open ./target/release/bundle/osx/ButterPaper.app
```

## Requirements

- `libpdfium.dylib` must be in the same directory as the binary
- The build script copies it to `target/release/`
