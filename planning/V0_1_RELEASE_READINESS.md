# ButterPaper v0.1 Release Readiness

Date:
Owner:

## Build/Test Gates

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo nextest run --workspace --all-features`
- [ ] `cargo test -p app-shell --test headless_ui --all-features`

## Feature Gates

- [ ] `butterpaper-cli open <file>` works
- [ ] `butterpaper-cli info <file>` outputs stable JSON
- [ ] `butterpaper-cli render-thumb <file> --page <n>` writes PNG
- [ ] top tab bar + overflow behavior validated
- [ ] shortcut contract validated (`0` / `9` / `8`)

## Performance Gates

- [ ] first-page render threshold met
- [ ] scroll throughput threshold met
- [ ] cache memory bound threshold met

## Platform Matrix

- [ ] macOS
- [ ] Windows
- [ ] Linux

## Notes
