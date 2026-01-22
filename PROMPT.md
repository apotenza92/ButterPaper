# PDF Editor - Migrate UI to egui

## Context
Replace the custom Metal-based UI components with egui for a modern, consistent look with automatic system light/dark mode support.

**Goal**: Keep PDF rendering (Metal/pdfium) but use egui for all UI chrome (toolbar, dialogs, sidebar, search bar).

## Current State
- Custom scene graph + Metal renderer for UI (`crates/ui/`)
- Toolbar, dialogs, thumbnails all hand-drawn with primitives
- Looks dated, inconsistent, hard to maintain
- No automatic dark/light mode

## Target State
- egui handles all UI elements (toolbar, sidebar, dialogs, menus)
- Metal renderer only for PDF page viewport
- Automatic system theme detection (light/dark mode)
- Modern look: rounded corners, proper spacing, hover states
- Easier to modify/extend

---

## Migration Plan

### Phase 1: Add egui + eframe scaffolding
**Status**: [ ] Not Started

1. Add dependencies (already done in `crates/app/Cargo.toml`):
   ```toml
   egui = "0.31"
   eframe = { version = "0.31", default-features = false, features = ["default_fonts", "wgpu"] }
   ```

2. Create new `main_egui.rs` entry point using eframe:
   - Set `follow_system_theme: true` in NativeOptions
   - Implement basic `eframe::App` trait
   - Render empty window with egui top panel (placeholder toolbar)

3. Verify it builds and launches

**Acceptance Criteria**:
- [ ] `cargo build --release` succeeds
- [ ] App launches with egui window
- [ ] Responds to system dark/light mode changes

---

### Phase 2: Migrate Toolbar to egui
**Status**: [ ] Not Started

Replace `crates/ui/src/toolbar.rs` functionality with egui:

1. Create egui top panel with:
   - Navigation: Prev/Next page buttons
   - Page indicator: "Page X of Y" with input field
   - Zoom controls: dropdown + zoom in/out buttons
   - Tools: Select, Hand, Text, Highlight, Comment, Measure, Freedraw

2. Wire up button callbacks to existing app state

3. Style with:
   - Proper icons or text labels
   - Rounded buttons
   - Good spacing (8px, 12px grid)

**Acceptance Criteria**:
- [ ] Toolbar renders in egui
- [ ] All buttons functional
- [ ] Looks modern (not placeholder rectangles)

---

### Phase 3: Migrate Sidebar (Thumbnail Strip) to egui
**Status**: [ ] Not Started

Replace `crates/ui/src/thumbnail.rs`:

1. Create egui side panel (left, ~120px wide)
2. Render scrollable list of page thumbnails
3. Highlight current page
4. Click to navigate

**Acceptance Criteria**:
- [ ] Thumbnail strip in egui side panel
- [ ] Shows actual page previews (may need texture bridge to egui)
- [ ] Click navigates to page

---

### Phase 4: Migrate Dialogs to egui
**Status**: [ ] Not Started

Replace:
- `calibration_dialog.rs` → egui::Window
- `error_dialog.rs` → egui::Window  
- `note_popup.rs` → egui::Window
- `search_bar.rs` → egui top/bottom panel or popup

**Acceptance Criteria**:
- [ ] All dialogs render in egui
- [ ] Modal behavior works
- [ ] Keyboard shortcuts preserved

---

### Phase 5: PDF Viewport Integration
**Status**: [ ] Not Started

The PDF page rendering must stay in Metal. Options:

1. **egui central panel + custom paint callback**: Use `egui::PaintCallback` to render Metal content in the viewport area
2. **Hybrid approach**: egui chrome around a native Metal view (more complex)

Research egui's custom rendering integration with wgpu/Metal.

**Acceptance Criteria**:
- [ ] PDF pages render in viewport
- [ ] Zoom/pan works
- [ ] Annotations render correctly
- [ ] Performance comparable to current

---

### Phase 6: Cleanup
**Status**: [ ] Not Started

1. Remove old UI code (`toolbar.rs`, `thumbnail.rs`, etc. - keep scene graph if needed for PDF overlay)
2. Remove old winit event loop (eframe handles it)
3. Update tests
4. Verify all features work

**Acceptance Criteria**:
- [ ] Clean build, no dead code warnings
- [ ] All tests pass
- [ ] Feature parity with old UI

---

## Key Files

| New | Purpose |
|-----|---------|
| `crates/app/src/main_egui.rs` | New eframe entry point |
| `crates/app/src/ui/` | New egui UI components |

| Old (to migrate/remove) | Purpose |
|-------------------------|---------|
| `crates/ui/src/toolbar.rs` | Custom toolbar |
| `crates/ui/src/thumbnail.rs` | Custom thumbnail strip |
| `crates/ui/src/calibration_dialog.rs` | Custom dialog |
| `crates/ui/src/error_dialog.rs` | Custom dialog |
| `crates/ui/src/search_bar.rs` | Custom search |

---

## Commands

```bash
# Build
cargo build --release 2>&1 | tail -50

# Test
cargo test --release 2>&1 | tail -30

# Run
cargo run --release --package pdf-editor

# Verify theme switching
# Change macOS System Settings → Appearance → Light/Dark and confirm app follows
```

---

## Done When

- [ ] App uses egui for all UI chrome
- [ ] Automatic light/dark mode works
- [ ] PDF viewport renders correctly (Metal)
- [ ] All existing features work (zoom, pan, annotations, search, etc.)
- [ ] Modern look: rounded corners, proper spacing, hover states
- [ ] All tests pass
- [ ] No regressions

---

## Notes

- Start with Phase 1-2 to validate approach before full migration
- Keep old code until new implementation verified
- egui + wgpu should work; Metal interop via wgpu-hal if needed
- Reference: https://docs.rs/egui, https://docs.rs/eframe
