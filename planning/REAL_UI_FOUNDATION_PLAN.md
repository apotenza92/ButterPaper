# ButterPaper Real UI Foundation Plan (Post-Scaffold)

## 1. Purpose

This document defines the first **intentional production UI pass** for the Rust-native desktop rewrite.
It replaces the current scaffold-style controls with a consistent, professional reader interface.

Scope here is **reader UX quality**, not annotation authoring.

## 2. UI Quality Bar (v0.1.1)

The UI pass is complete only when all of the following are true:

1. Shell looks and behaves like a coherent desktop PDF tool, not a debug panel.
2. Top tab bar feels browser-like with overflow scroll and visible active state.
3. Toolbar is structured into stable groups (File, View Mode, Navigation, Zoom).
4. Continuous and single-page modes are discoverable and visually obvious.
5. Thumbnail lane has clear active-page highlight and loading/empty states.
6. Keyboard shortcuts, menu labels, and on-screen affordances are consistent.
7. Screenshot baselines across macOS/Windows/Linux pass with deterministic rendering.

## 3. Canonical Shell Layout

Window structure (top to bottom):

1. App title bar integration area (platform-native where available).
2. **Tab strip row** (fixed height, horizontal scroll for overflow).
3. **Toolbar row** (grouped controls with separators).
4. **Main split pane**:
   - left: thumbnail rail (resizable later; fixed width for now),
   - right: document viewport and status footer.

Locked dimensions for the first pass:

1. Tab strip height: `40px`.
2. Toolbar height: `44px`.
3. Thumbnail rail width: `220px`.
4. Inter-group spacing in toolbar: `16px`.
5. Page vertical spacing in continuous mode: `24px`.

## 4. Component Contracts

## 4.1 Tab Strip

1. Tab contains title, active marker, and close affordance.
2. Active tab styling must be distinct in both light and dark themes.
3. Overflow behavior:
   - horizontal scroll is enabled,
   - active tab auto-scrolls into view after activation/open/close.
4. Plus button opens a new welcome tab.

## 4.2 Toolbar Groups

1. File group:
   - `Open…`
   - `Close Tab`
2. Mode group:
   - segmented control: `Continuous`, `Single Page`
3. Navigation group:
   - previous page button
   - page indicator `N / Total`
   - next page button
4. Zoom group:
   - `-`, current zoom value, `+`
   - `100%`, `Fit Width`, `Fit Page`

Rules:

1. Only relevant controls are enabled for current state.
2. Disabled controls still reserve layout space (no reflow jumps).
3. Zoom value always reflects effective zoom (including fit modes).

## 4.3 Thumbnail Rail

1. Each thumbnail cell includes:
   - preview image
   - page label
2. Active page has strong highlight treatment.
3. Loading thumbnails use skeleton placeholders.
4. Clicking thumbnail updates current page in both view modes.

## 4.4 Viewer Surface

1. Continuous mode:
   - virtualized page window
   - stable scroll anchoring on resize/zoom updates
2. Single-page mode:
   - centered page with deterministic bounds navigation
3. Empty/open states:
   - centered primary action and concise help text

## 5. Visual System (GPUI Theme Tokens)

Initial token set (add as constants in UI crate/module):

1. Spacing scale: `4, 8, 12, 16, 20, 24`.
2. Radius scale: `6, 8, 10`.
3. Semantic colors:
   - background
   - surface
   - surface-muted
   - border
   - text-primary
   - text-secondary
   - accent
   - accent-strong
4. Typography levels:
   - `label-sm`
   - `label-md`
   - `body`
   - `title`

Rules:

1. No inline one-off colors in layout code.
2. No ad-hoc padding values outside the token scale.
3. All controls use shared style helpers (`button_primary`, `button_subtle`, `pill_active`, etc.).

## 6. Accessibility + Input Contract

1. Keyboard focus ring visible for all interactive controls.
2. All toolbar actions have keyboard equivalents where applicable.
3. Shortcut contract remains locked:
   - `Cmd/Ctrl+0` -> `100%`
   - `Cmd/Ctrl+9` -> `Fit Page`
   - `Cmd/Ctrl+8` -> `Fit Width`
4. Tab order is deterministic from top-left to bottom-right.
5. Minimum hit target for clickable controls: `32px`.

## 7. Implementation Slices

## Slice A - Design tokens + shared primitives

1. Add `crates/app-shell/src/theme.rs` with semantic tokens.
2. Add reusable styled components/helpers for:
   - segmented controls
   - icon/text buttons
   - toolbar group containers
3. Replace scattered inline spacing values in desktop shell with tokens.

Done when:

1. Desktop shell compiles with no inline literal styling except locked dimensions.
2. Visual snapshot for default shell is approved.

## Slice B - Tab strip + toolbar refactor

1. Implement structured tab item with close button and active state.
2. Add dedicated toolbar group widgets instead of raw button rows.
3. Add proper disabled states and hover/press styling.

Done when:

1. Headless UI tests verify tab lifecycle behavior with the new widgets.
2. Visual snapshots for tab overflow and active tab state are approved.

## Slice C - Thumbnail and viewer presentation polish

1. Add thumbnail skeleton/loading states and active highlight.
2. Improve empty state and loading copy in viewer.
3. Ensure continuous/single-page transitions preserve visual stability.

Done when:

1. Screenshot diffs for open document flows are stable per OS.
2. UX acceptance script passes manually on all three platforms.

## 8. Testing Plan for UI Pass

## 8.1 Functional (non-visual)

1. Headless `gpui_test` scenarios:
   - tab open/close/activate
   - mode switch
   - page next/prev
   - zoom shortcut routing
   - thumbnail click-to-jump

## 8.2 Visual

1. `insta` snapshots for:
   - empty shell
   - 1 tab loaded
   - 20-tab overflow
   - continuous mode with active thumbnail
   - single-page mode
2. OS-specific baseline directories:
   - `tests/visual/snapshots/macos`
   - `tests/visual/snapshots/windows`
   - `tests/visual/snapshots/linux`

## 8.3 Acceptance script

Manual script (release candidate):

1. Open medium PDF.
2. Verify tab creation, active styling, and close behavior.
3. Verify thumbnail selection sync while scrolling.
4. Toggle continuous/single-page and confirm current page preservation.
5. Validate `Cmd/Ctrl+0`, `Cmd/Ctrl+9`, `Cmd/Ctrl+8`.

## 9. Non-Goals

1. Annotation tool palettes.
2. Multi-window/tab tear-off.
3. Theme editor.
4. Collaboration UI.

## 10. Default Style Policy Addendum

For the current desktop shell pass, default GPUI styling is the primary contract:

1. No manual separator widgets for menu/tab/toolbar boundaries.
2. No ad-hoc border construction in desktop shell view code.
3. Prefer built-in GPUI primitives and shared component style helpers for bordered surfaces and controls.
4. Typography policy: default body size plus one shared metadata size for secondary labels/hints.
