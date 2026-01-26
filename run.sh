#!/bin/bash
# Build and run the ButterPaper with latest changes
cd "$(dirname "$0")"

# Clean our crates to prevent artifact bloat (keeps dependencies cached)
cargo clean -p butterpaper-gpui -p butterpaper-render --quiet 2>/dev/null

cargo run --manifest-path crates/gpui-app/Cargo.toml --bin butterpaper "$@"
