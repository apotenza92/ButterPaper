# ButterPaper Initial Build + Migration Plan (GPUI)

## 1. Summary

This plan defines the first production slice of ButterPaper as a Rust-native desktop app with a CLI-first foundation.  
It preserves key UX from the old product while moving to a clean `GPUI` stack and a cross-platform automated test pipeline.

Design source of truth for old behavior remains `../ButterPaper-history` (read-only reference).
Execution checklist for this plan lives in `planning/INITIAL_BUILD_CHECKLIST.md`.
UI polish execution details live in `planning/REAL_UI_FOUNDATION_PLAN.md`.

## 2. Success Criteria for Initial Build (v0.1)

`v0.1` is successful only if all items below are shipped:

1. Open PDF from CLI and desktop UI.
2. Browser-style tab strip at top of window with horizontal overflow scrolling.
3. Left thumbnail rail with fast page preview generation and click-to-jump.
4. Continuous scroll mode with smooth reading on medium/large PDFs.
5. Single-page mode with correct page centering.
6. Zoom UX parity:
   - `Fit Page`
   - `Fit Width`
   - manual zoom percent
   - visible zoom indicator (for example `100%`)
7. Shortcut contract:
   - `Cmd/Ctrl+0` -> `100%` (Actual Size)
   - `Cmd/Ctrl+9` -> `Fit Page`
   - `Cmd/Ctrl+8` -> `Fit Width`
8. Headless UI test coverage + screenshot regression coverage on macOS, Windows, Linux.

## 3. UX Contract (Locked for v0.1)

## 3.1 Window layout

1. Top row: browser-like tab strip.
2. Main body:
   - left thumbnail sidebar
   - center document viewport
3. Empty state: centered `Open File` action.

## 3.2 Tab system

1. Tabs are horizontally scrollable when overflowed.
2. Tab order is stable and deterministic.
3. Active tab is always visible after open/switch/close.
4. `+` creates a new empty tab.
5. Closing active tab activates nearest surviving tab.

## 3.3 Viewer modes

1. `Continuous`: vertical scrolling across all pages.
2. `SinglePage`: one page centered at a time, next/prev navigation.
3. Mode switch preserves logical position (same page remains selected).

## 3.4 Zoom UX

1. Toolbar exposes:
   - zoom out
   - zoom percent display/input
   - zoom in
   - fit width
   - fit page
2. `Cmd/Ctrl+0` sets manual `100%`.
3. `Cmd/Ctrl+9` sets `Fit Page`.
4. `Cmd/Ctrl+8` sets `Fit Width`.
5. Menu hints and runtime behavior must always match.

## 3.5 PDF navigation

1. Thumbnail click jumps to page.
2. Current page highlight follows viewport.
3. Arrow/PageUp/PageDown navigation works in both viewer modes.

## 4. Architecture for Initial Build

## 4.1 Workspace targets

1. `apps/desktop` - native app bootstrap and window lifecycle.
2. `crates/cli` - CLI entrypoints (`open`, `info`, `render-thumb`, `version`).
3. `crates/app-shell` - menu, tab strip, toolbar, settings panel, action routing.
4. `crates/doc-model` - session/document/tab/preferences state + pure reducers.
5. `crates/viewer-core` - viewport math, mode switching, visible range, zoom fit calculations.
6. `crates/pdf-engine` - renderer trait + backend adapter.
7. `crates/storage` - preferences/session persistence.

## 4.2 Core contracts

## `doc-model`

1. `SessionState`
2. `TabState`
3. `Preferences`
4. Reducers for tab lifecycle, document open/close, mode + zoom actions

## `viewer-core`

1. `ViewportState`
2. `ViewMode` (`Continuous`, `SinglePage`)
3. `ZoomMode` (`Percent`, `FitPage`, `FitWidth`)
4. Fit calculation functions:
   - `fit_width_percent(viewport_px, page_px, dpr)`
   - `fit_page_percent(viewport_px, page_px, dpr)`

## `pdf-engine`

1. `open(path|bytes) -> DocumentHandle`
2. `page_count(handle) -> u32`
3. `page_size(handle, page) -> Size`
4. `render_page(handle, page, scale, clip) -> RgbaImage`
5. `render_thumbnail(handle, page, target_size) -> RgbaImage`
6. `close(handle)`

## 5. Initial Build Phases (Detailed)

## Phase 0 - Foundation + test harness (2-3 days)

1. Create missing crates in workspace.
2. Add unified dev tooling:
   - `cargo fmt`
   - `cargo clippy`
   - `cargo nextest`
3. Add test dependencies:
   - `assert_cmd` for CLI
   - `gpui_test` for headless UI interaction
   - `insta` for snapshot and screenshot approval

Exit criteria:

1. `cargo nextest run` passes with smoke tests.
2. CI matrix runs on macOS, Windows, Linux.

## Phase 1 - CLI-first vertical slice (3-5 days)

1. Implement CLI commands:
   - `butterpaper-cli open <file>`
   - `butterpaper-cli info <file>`
   - `butterpaper-cli render-thumb <file> --page <n>`
2. Wire `open` command to launch desktop app with file path.
3. Add deterministic JSON output for `info`.

Exit criteria:

1. CLI commands work cross-platform.
2. CLI tests fully pass in CI.

## Phase 2 - Desktop shell + top tab strip (4-6 days)

1. Implement window shell, menu bar, toolbar.
2. Implement top tab strip with horizontal overflow behavior.
3. Open-file flow from picker and drag-drop.
4. Route keyboard shortcuts and menu commands through `app-shell`.

Exit criteria:

1. Multiple PDFs open as tabs.
2. Tab strip remains usable with overflow.
3. Shortcut contract (`0`, `9`, `8`) is enforced.

## Phase 3 - Viewer core + render pipeline (6-10 days)

1. Implement continuous viewer with virtualization.
2. Implement single-page viewer mode.
3. Implement fit width/page/percent calculations in `viewer-core`.
4. Render queue:
   - visible pages first
   - preload neighbor pages
   - cancel stale jobs on fast scroll/zoom

Exit criteria:

1. Continuous and single-page views are both functional.
2. Fit modes produce correct page geometry.
3. Page/thumbnail navigation stays synchronized.

## Phase 4 - Thumbnail lane + performance pass (4-6 days)

1. Add thumbnail cache and background thumbnail generation.
2. Add tile/page cache with LRU eviction.
3. Enforce frame budget and avoid blocking UI thread.
4. Add perf smoke tests and thresholds.

Exit criteria:

1. Thumbnails load progressively and remain responsive.
2. 100+ page documents remain smooth in continuous mode.
3. Memory remains bounded by cache limits.

## Phase 5 - Stabilization + release gating (3-5 days)

1. Finalize settings persistence and restore behavior.
2. Complete screenshot baselines per OS.
3. Add release checklist and packaging smoke tests.

Exit criteria:

1. All required tests pass on all platforms.
2. v0.1 release candidate is reproducible from clean checkout.

## 6. Testing Platform (Clean, Single Stack)

## 6.1 Test layers

1. Unit tests:
   - reducers, geometry math, zoom mapping, cache logic
2. CLI functional tests:
   - command behavior and JSON contract
3. Headless UI tests:
   - tab operations, open flow, mode switch, zoom actions
4. Visual regression tests:
   - `gpui_test` screenshots with `insta` snapshot review

## 6.2 Determinism rules for screenshot tests

1. Fixed viewport size.
2. Fixed scale factor (DPR) per test profile.
3. Fixed fonts and theme in test mode.
4. Animations disabled or stepped deterministically.
5. Separate baseline snapshots per OS.

## 6.3 CI gates

1. `cargo fmt --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo nextest run --workspace`
4. `cargo test -p app-shell --features screenshot-tests` (or equivalent visual suite)

## 7. Required Test Scenarios

1. `Cmd/Ctrl+0` sets `zoom_mode=Percent` and `zoom_percent=100` in both viewer modes.
2. `Cmd/Ctrl+9` sets `zoom_mode=FitPage` and fit geometry is valid.
3. `Cmd/Ctrl+8` sets `zoom_mode=FitWidth` and fit geometry is valid.
4. Menu shortcut hints match runtime key routing.
5. Open PDF -> thumbnails appear -> click thumbnail -> selected page changes.
6. Continuous mode scroll updates current page index correctly.
7. Single-page mode next/prev works without page desync.
8. Tab overflow scenario still allows switching and close actions.
9. CLI `info` output remains stable JSON contract.

## 8. Assumptions and Defaults

1. CLI-first means first green milestone is CLI + backend + tests before full UI parity.
2. No web runtime or webview is used in production binaries.
3. Rendering backend remains behind `pdf-engine` trait so backend can evolve without shell churn.
4. v0.1 excludes full annotation editing and collaboration.
- Theme set in first pass includes a minimal light/dark pair; expanded catalog can follow.

## 9. Non-Goals for This Migration Document

- Final annotation architecture.
- Collaboration/multiplayer workflows.
- Cloud sync and accounts.

These are handled after reader and shell parity are complete.
