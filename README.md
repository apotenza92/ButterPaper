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
