# ButterPaper v0.1 Implementation Checklist

Reference: `planning/MIGRATION_UI_UX_PLAN.md`

## Current Progress (2026-02-05)

- Completed in-repo implementation: PR0 baseline normalization structure, PR1 foundation/test platform, PR2 CLI vertical slice, and initial PR3 shell scaffolding.
- Validated locally with:
  - `cargo fmt --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo test --workspace --all-features`
  - `cargo nextest run --workspace --all-features`

## Estimation Notes

- Estimates are in focused engineering days (`1d ~= 6-7 hours`).
- Estimates include implementation + tests in the same task.
- Totals assume one engineer; parallel work can reduce calendar time.

## Milestone Overview

| Milestone | Goal | Estimated Effort |
| --- | --- | --- |
| M0 | Foundation + test platform | 5.0d |
| M1 | CLI-first vertical slice | 6.0d |
| M2 | Desktop shell + top tab bar | 6.5d |
| M3 | Viewer modes + zoom contract | 7.5d |
| M4 | Thumbnails + performance hardening | 6.0d |
| M5 | CI/release gates + stabilization | 4.0d |
| M6 | UI foundation polish pass | 4.0d |
|  | **Total** | **39.0d** |

## M0 - Foundation + Test Platform

| ID | Crate/Path | Task | Est | Depends On | Done When |
| --- | --- | --- | --- | --- | --- |
| M0-1 | `Cargo.toml` | Add workspace members: `crates/cli`, `crates/pdf-engine`, `crates/storage` | 0.5d | - | Workspace builds with all members |
| M0-2 | repo root | Add `rustfmt.toml`, clippy config, and standard test aliases | 0.5d | M0-1 | `fmt`, `clippy`, and tests run consistently |
| M0-3 | repo root | Add `nextest` config and profiles for local/CI | 0.5d | M0-1 | `cargo nextest run --workspace` passes on smoke set |
| M0-4 | test deps | Add `assert_cmd`, `gpui_test`, `insta`, screenshot feature flags | 0.5d | M0-3 | Test crates compile with all test deps |
| M0-5 | `tests/fixtures` | Add deterministic PDF fixture set (small, medium, large) | 1.0d | M0-4 | Fixtures are versioned and used by tests |
| M0-6 | `crates/doc-model` | Expand base state types (session/tab/preferences) with pure reducers | 1.5d | M0-1 | Reducer unit tests cover open/close/switch/update paths |
| M0-7 | `crates/viewer-core` | Add viewport/zoom math primitives and fit functions | 0.5d | M0-1 | Geometry tests pass for fit width/page edge cases |

## M1 - CLI-First Vertical Slice

| ID | Crate/Path | Task | Est | Depends On | Done When |
| --- | --- | --- | --- | --- | --- |
| M1-1 | `crates/pdf-engine` | Define `PdfEngine` trait, handle types, and error model | 0.5d | M0-1 | Trait and errors are stable and documented |
| M1-2 | `crates/pdf-engine` | Implement first backend adapter (PDFium) for open/page metadata/render | 2.0d | M1-1 | Can open PDFs and render a page/thumbnail in tests |
| M1-3 | `crates/cli` | Implement command parser and subcommands: `open`, `info`, `render-thumb`, `version` | 1.0d | M1-1 | `--help` documents all commands |
| M1-4 | `crates/cli` | Add deterministic JSON output contract for `info` | 0.5d | M1-3 | Snapshot test locks JSON schema and fields |
| M1-5 | `apps/desktop` + `crates/cli` | Wire `open` command to launch desktop app with file argument | 0.5d | M1-3 | `butterpaper-cli open <file>` opens file in desktop app |
| M1-6 | `tests/cli` | Add CLI contract tests with `assert_cmd` + fixtures | 1.0d | M1-3 | CLI suite green on macOS/Windows/Linux |
| M1-7 | `crates/pdf-engine` | Add failure-path tests (missing file, encrypted PDF, invalid data) | 0.5d | M1-2 | Error handling is explicit and tested |

## M2 - Desktop Shell + Top Tab Bar

| ID | Crate/Path | Task | Est | Depends On | Done When |
| --- | --- | --- | --- | --- | --- |
| M2-1 | `crates/app-shell` | Implement main layout: top tabs, left thumbnail rail, center viewport area | 1.0d | M0-6 | Empty and loaded states render correctly |
| M2-2 | `crates/app-shell` | Implement browser-style tab model actions (new/close/switch/replace welcome tab) | 1.5d | M2-1 | Tab reducer tests cover all transitions |
| M2-3 | `crates/app-shell` | Implement horizontal overflow behavior in tab strip | 1.0d | M2-2 | 20+ tabs remain navigable and active tab stays visible |
| M2-4 | `crates/app-shell` | Add file-open flows: menu, shortcut, drag-drop | 1.0d | M2-1, M1-2 | Any path opens a tab with document metadata |
| M2-5 | `crates/app-shell` | Implement view menu + shortcut hints with runtime mapping parity | 0.5d | M2-1 | Menu hints always match actual command routing |
| M2-6 | `tests/ui` | Add headless UI tests for tab behavior with `gpui_test` | 1.0d | M2-2 | UI tests verify tab lifecycle and overflow scenarios |
| M2-7 | `tests/visual` | Add first shell screenshots with `gpui_test` + `insta` | 0.5d | M2-1 | Baseline snapshots approved for all OS targets |

## M3 - Viewer Modes + Zoom Contract

| ID | Crate/Path | Task | Est | Depends On | Done When |
| --- | --- | --- | --- | --- | --- |
| M3-1 | `crates/viewer-core` | Implement continuous scroll state machine (visible range + current page detection) | 1.5d | M0-7 | Scroll math tests pass with variable page heights |
| M3-2 | `crates/viewer-core` | Implement single-page mode state machine | 1.0d | M3-1 | Next/prev and bounds behavior is deterministic |
| M3-3 | `crates/viewer-core` | Implement mode switching while preserving logical page position | 0.5d | M3-1, M3-2 | Mode-switch tests confirm same target page |
| M3-4 | `crates/app-shell` + `crates/doc-model` | Wire zoom controls (in/out, percent input, fit width, fit page) | 1.0d | M0-6, M0-7 | Toolbar and keyboard produce expected zoom state |
| M3-5 | `crates/app-shell` | Enforce shortcut contract: `0`=100%, `9`=Fit Page, `8`=Fit Width | 0.5d | M3-4 | Cross-mode shortcut tests pass |
| M3-6 | `crates/app-shell` + `crates/pdf-engine` | Integrate render loop with cancelable jobs and priority (visible first) | 2.0d | M1-2, M3-1 | Fast scroll does not block UI; stale renders canceled |
| M3-7 | `tests/ui` + `tests/visual` | Add UI tests for continuous/single-page behavior and zoom shortcuts | 1.0d | M3-1 to M3-5 | Tests validate both modes + shortcut geometry correctness |

## M4 - Thumbnails + Performance Hardening

| ID | Crate/Path | Task | Est | Depends On | Done When |
| --- | --- | --- | --- | --- | --- |
| M4-1 | `crates/viewer-core` | Add thumbnail generation queue and caching policy | 1.5d | M3-6 | Thumbnails appear progressively without UI stalls |
| M4-2 | `crates/app-shell` | Wire thumbnail rail interactions and selection sync with viewport | 1.0d | M4-1, M3-1 | Click thumbnail jumps page; active page highlight stays synced |
| M4-3 | `crates/viewer-core` | Add page/tile LRU cache limits and eviction instrumentation | 1.0d | M3-6 | Memory bounded by configured cache caps |
| M4-4 | `crates/viewer-core` | Add prefetch policy (neighbor pages) and stale job cancellation metrics | 1.0d | M3-6 | Prefetch improves nav smoothness with no starvation |
| M4-5 | `tests/perf` | Add performance smoke suite (first paint, scroll throughput proxy, memory guardrail) | 1.5d | M4-1 to M4-4 | Perf suite is deterministic and gating in CI |

## M5 - CI/Release Gates + Stabilization

| ID | Crate/Path | Task | Est | Depends On | Done When |
| --- | --- | --- | --- | --- | --- |
| M5-1 | `.github/workflows` | Add matrix CI for macOS, Windows, Linux with nextest + visual tests | 1.0d | M0-3, M2-7 | All suites run per platform |
| M5-2 | `tests/visual` | Split baseline snapshots per platform and document update flow | 0.5d | M2-7 | Snapshot diffs are reviewable and deterministic |
| M5-3 | `crates/storage` | Implement preferences/session persistence with schema versioning | 1.0d | M0-6 | Restart preserves settings and last-session state |
| M5-4 | repo root | Add release smoke checklist and packaging verification script | 0.5d | M5-1 | RC build process is scripted and repeatable |
| M5-5 | `planning/` | Add `v0.1` release readiness report template | 0.5d | M5-1 to M5-4 | Team can record pass/fail against exit criteria |
| M5-6 | workspace | Final bugfix + stabilization buffer | 0.5d | all | All blocker issues resolved before `v0.1` tag |

## M6 - UI Foundation Polish (v0.1.1)

Reference: `planning/REAL_UI_FOUNDATION_PLAN.md`

| ID | Crate/Path | Task | Est | Depends On | Done When |
| --- | --- | --- | --- | --- | --- |
| M6-1 | `crates/app-shell` | Add shared UI theme tokens and style helpers | 1.0d | M2, M3 | No ad-hoc styling in shell layout |
| M6-2 | `apps/desktop` | Replace scaffold toolbar with grouped production controls | 1.0d | M6-1 | Toolbar has stable groups and disabled states |
| M6-3 | `apps/desktop` | Upgrade tab strip (active visuals, per-tab close, overflow affordances) | 1.0d | M6-1 | Tab UX matches browser-like contract |
| M6-4 | `apps/desktop` + `tests/visual` | Thumbnail rail and viewer empty/loading state polish + new snapshots | 1.0d | M6-2, M6-3 | Visual baselines approved per OS |

## Explicit Acceptance Checklist (Must Be Green Before v0.1 Tag)

- CLI:
  - `open`, `info`, `render-thumb`, `version` are stable and tested.
- Tabs:
  - top bar, overflow scrolling, deterministic activation behavior.
- Viewer:
  - continuous mode and single-page mode both complete.
  - fit width/page and manual zoom percent work and are visible.
- Shortcuts:
  - `Cmd/Ctrl+0` -> `100%`
  - `Cmd/Ctrl+9` -> `Fit Page`
  - `Cmd/Ctrl+8` -> `Fit Width`
- Navigation:
  - thumbnail click-to-jump and viewport-to-thumbnail sync.
- Tests:
  - `nextest`, CLI contracts, headless UI flows, screenshot snapshots all green on macOS/Windows/Linux.
