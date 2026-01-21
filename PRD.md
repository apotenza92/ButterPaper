# PDF Editor - Phase 2 PRD (Integration & Polish)

Phase 1 created foundational subsystems but the app doesn't work as an integrated PDF editor. This PRD focuses on making it functional and usable.

## Quick Start

```bash
# Build the app
./scripts/build-app.sh

# Run tests
./scripts/run-tests.sh

# Run the app
open "target/release/bundle/osx/PDF Editor.app"
```

## Current State Analysis

### What EXISTS (data structures/logic):
- ✅ Window with Metal rendering
- ✅ PDF loading via PDFium
- ✅ Tile-based page rendering
- ✅ ThumbnailStrip component (not wired up)
- ✅ TextSelection/TextSearchManager (not wired up)
- ✅ ViewportCompositor (not wired up)
- ✅ Annotation data model (no UI)
- ✅ Measurement system (no UI)
- ✅ Cache system (RAM/GPU/disk)
- ✅ Job scheduler

### What DOESN'T WORK:
- ⚠️ PDF pages render but may have display issues (pixel format fixed, needs verification)
- ❌ No menu bar
- ❌ No toolbar  
- ❌ No buttons or clickable UI elements
- ❌ No sidebar/thumbnail view
- ✅ Opening from bundle - **FIXED with build script**
- ❌ None of the subsystems are wired to the UI
- ❌ No way to access any features except keyboard shortcuts

### Unit Tests: ✅ ALL PASSING (476 tests)
All unit tests now pass after fixing:
- Rectangle bounding_box coordinate calculation
- Circle/Ellipse hit testing (interior, not just edge)
- Environment variable test isolation with serial_test

---

## Priority 0: Bundle & Distribution

### Phase 0.1: Fix Bundle ✅ DONE
- [x] Create build script that downloads libpdfium.dylib if missing (`scripts/setup-pdfium.sh`)
- [x] Copy libpdfium.dylib into app bundle during cargo-bundle (`scripts/build-app.sh`)
- [x] Set @rpath correctly so the dylib is found (currently relies on same-dir lookup)
- [ ] Test: `open "PDF Editor.app"` works without terminal
- [ ] Test: Drag PDF onto app icon in Finder opens it

### Phase 0.2: Fix Failing Tests ✅ DONE
- [x] Fix `test_handle_annotation_selection` - fixed Rectangle bounding_box to properly compute min/max
- [x] Fix `test_hit_test_with_multiple_layers` - fixed Circle hit_test to check interior, not just edge
- [x] Fix env var test race conditions - added serial_test for isolation

---

## Priority 1: Make PDFs Actually Visible

### Phase 1.1: Fix PDF Display
- [x] Debug why texture blit doesn't show content (fixed: set MTLStorageMode::Managed for CPU-writable textures)
- [x] Verify RGBA→BGRA conversion is correct (verified: conversion is correct, issue was storage mode)
- [x] Add debug overlay showing texture dimensions
- [x] Test with multiple PDF files (added `--test-load` flag and `scripts/test-multiple-pdfs.sh`)
- [x] **Terminal test:** `cargo run --release -- --test-load test.pdf` outputs `LOAD: OK pages=X time=Xms`

### Phase 1.2: Page Info Overlay
- [x] Show "Page X of Y" in bottom-right corner
- [x] Show current zoom level (e.g., "100%")
- [x] Show loading spinner while rendering
- [x] Use scene graph primitives (Rect + text texture)

### Phase 1.3: Connect Zoom/Pan to Display
- [x] Make scroll wheel zoom update the visible page
- [x] Make mouse drag pan the page
- [x] Add smooth animation for zoom transitions
- [x] **Terminal test:** Log viewport state changes

---

## Keyboard Shortcuts (Bluebeam-Compatible)

**IMPORTANT:** All keyboard shortcuts must match Bluebeam Revu exactly, including single-key shortcuts.
Reference: https://support.bluebeam.com/revu/resources/keyboard-shortcuts.html

**macOS Mapping:** Bluebeam uses Windows conventions (Ctrl). On macOS, map Ctrl → Cmd for standard shortcuts.
For example: Ctrl+S (Windows) → Cmd+S (macOS). Keep Alt as Option.

### Single-Key Tool Shortcuts (Critical - must work without modifiers)
| Key | Tool |
|-----|------|
| V | Select |
| A | Arrow |
| C | Cloud |
| E | Ellipse |
| F | File Attachment |
| G | Snapshot |
| H | Highlight |
| I | Image |
| J | Dynamic Fill |
| K | Cloud+ |
| L | Line |
| M | Measure Tool |
| N | Note |
| P | Pen |
| Q | Callout |
| R | Rectangle |
| S | Stamp |
| T | Text Box |
| U | Underline |
| W | Typewriter |
| X | Add Signature Field |
| Z | Zoom Tool |
| D | Strikethrough |

### Selection & Navigation
| Shortcut | Action |
|----------|--------|
| V | Select tool |
| Shift+V | Pan tool |
| Shift+T | Select Text |
| Shift+O | Lasso select |
| Shift+Z | Toggle Zoom Tool |
| + / = | Zoom In |
| - | Zoom Out |
| Home | First Page |
| End | Last Page |
| Ctrl+Left | Previous Page |
| Ctrl+Right | Next Page |
| Alt+Left | Previous View |
| Alt+Right | Next View |

### View Shortcuts
| Shortcut | Action |
|----------|--------|
| Ctrl+0 | Fit Width |
| Ctrl+8 | Actual Size (100%) |
| Ctrl+9 | Fit Page |
| Ctrl+4 | Single Page Mode |
| Ctrl+5 | Continuous Mode |
| Ctrl+R | Toggle Rulers |
| F11 | Full Screen |
| Ctrl+Enter | Presentation Mode |

### Markup Tool Shortcuts (with modifiers)
| Shortcut | Tool |
|----------|------|
| Shift+C | Arc |
| Shift+L | Dimension |
| Shift+E | Eraser |
| Shift+F | Flag |
| Shift+H | Hyperlink |
| Shift+I | Image from Scanner |
| Shift+N | Polyline |
| Shift+P | Polygon |

### Measurement Shortcuts
| Shortcut | Tool |
|----------|------|
| M | Measure Tool |
| Shift+Alt+L | Length |
| Shift+Alt+A | Area |
| Shift+Alt+P | Perimeter |
| Shift+Alt+C | Count |
| Shift+Alt+D | Diameter |
| Shift+Alt+G | Angle |
| Shift+Alt+U | Radius |
| Shift+Alt+V | Volume |
| Shift+Alt+Q | Polylength |

### Standard Edit Shortcuts
| Shortcut | Action |
|----------|--------|
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| Ctrl+C | Copy |
| Ctrl+V | Paste |
| Ctrl+Shift+V | Paste in Place |
| Ctrl+X | Cut |
| Ctrl+A | Select All |
| Ctrl+Shift+A | Select All Text |
| Del | Delete |

### File Shortcuts
| Shortcut | Action |
|----------|--------|
| Ctrl+O | Open |
| Ctrl+S | Save |
| Ctrl+Shift+S | Save As |
| Ctrl+P | Print |
| Ctrl+W / Ctrl+F4 | Close |
| Ctrl+Shift+W | Close All |
| Ctrl+F | Search |
| F3 | Next Search Result |
| Shift+F3 | Previous Search Result |

### Panel Shortcuts
| Shortcut | Panel |
|----------|-------|
| Alt+T | Thumbnails |
| Alt+B | Bookmarks |
| Alt+L | Markups List |
| Alt+U | Measurements |
| Alt+P | Properties |
| Alt+1 | Search |
| F4 | Navigation Bar |
| Shift+F4 | Hide Panels |

### Mouse Modifiers
- **Spacebar + drag**: Pan without deselecting current tool
- **Ctrl + scroll**: Toggle between zoom and pan
- **Shift + drag**: Constrain to horizontal/vertical/45°
- **Ctrl+Shift + click**: Copy and move in straight line

---

## Priority 2: Native macOS UI

### Phase 2.1: Menu Bar
The app needs a proper macOS menu bar. Use `cocoa` crate for native menus.

- [ ] Create NSMenu with standard structure
- [ ] **File menu:**
  - [ ] Open... (Cmd+O) - working
  - [ ] Open Recent → submenu
  - [ ] Close (Cmd+W)
  - [ ] Save (Cmd+S)
  - [ ] Save As... (Cmd+Shift+S)
  - [ ] Export as PDF...
  - [ ] Export as Images...
- [ ] **Edit menu:**
  - [ ] Undo (Cmd+Z)
  - [ ] Redo (Cmd+Shift+Z)
  - [ ] Copy (Cmd+C)
  - [ ] Select All (Cmd+A)
  - [ ] Find... (Cmd+F)
- [ ] **View menu:**
  - [ ] Zoom In (Cmd+=)
  - [ ] Zoom Out (Cmd+-)
  - [ ] Actual Size (Cmd+0)
  - [ ] Fit Page
  - [ ] Fit Width
  - [ ] Show Thumbnails (Cmd+T)
  - [ ] Show Annotations
- [ ] **Go menu:**
  - [ ] Next Page (→, PgDown)
  - [ ] Previous Page (←, PgUp)
  - [ ] First Page (Home)
  - [ ] Last Page (End)
  - [ ] Go to Page... (Cmd+G)
- [ ] **Window menu:**
  - [ ] Minimize
  - [ ] Zoom
- [ ] **Help menu:**
  - [ ] About PDF Editor

### Phase 2.2: Toolbar
GPU-rendered toolbar at top of window.

- [ ] Create toolbar component using scene graph
- [ ] Add icon rendering (use simple geometric shapes initially)
- [ ] **Navigation section:**
  - [ ] Previous page button (◀)
  - [ ] Page number input field
  - [ ] Next page button (▶)
- [ ] **Zoom section:**
  - [ ] Zoom out button (-)
  - [ ] Zoom dropdown (50%, 75%, 100%, 125%, 150%, 200%)
  - [ ] Zoom in button (+)
  - [ ] Fit page button
  - [ ] Fit width button
- [ ] **Tools section:**
  - [ ] Selection tool (arrow)
  - [ ] Hand/pan tool
  - [ ] Text select tool
  - [ ] Highlight tool
  - [ ] Comment/note tool
  - [ ] Measurement tool
- [ ] Handle button hover states
- [ ] Handle button click events
- [ ] **Terminal test:** Log button clicks

### Phase 2.3: Sidebar (Thumbnail Strip)
Wire up the existing ThumbnailStrip component.

- [ ] Connect ThumbnailStrip to main app
- [ ] Render page thumbnails on left side
- [ ] Highlight current page
- [ ] Click thumbnail to navigate
- [ ] Toggle visibility (Cmd+T)
- [ ] Smooth scroll in sidebar
- [ ] Lazy load thumbnails (visible first)
- [ ] **Terminal test:** Log thumbnail render times

---

## Priority 3: Core Features

### Phase 3.1: Text Selection
Wire up existing TextSelection component.

- [ ] Click and drag to select text
- [ ] Use PDFium text extraction
- [ ] Render selection highlight rectangles
- [ ] Copy to clipboard (Cmd+C)
- [ ] Double-click for word, triple for line
- [ ] Show cursor change on hover over text
- [ ] **Terminal test:** Log selected text to stdout

### Phase 3.2: Search (Cmd+F)
Wire up existing TextSearchManager.

- [ ] Add search bar UI at top of window
- [ ] Search input field
- [ ] Match count display
- [ ] Previous/Next buttons
- [ ] Close button (Esc)
- [ ] Highlight all matches in yellow
- [ ] Highlight current match in orange
- [ ] Scroll to match location
- [ ] Case sensitive toggle
- [ ] **Terminal test:** `--search "term" file.pdf` outputs matches

### Phase 3.3: Annotations
Wire up existing annotation system.

- [ ] Highlight tool (select text, press H)
- [ ] Render highlights from annotation data
- [ ] Comment/note tool (click to place)
- [ ] Render note icons
- [ ] Click note to show popup
- [ ] Freehand drawing tool
- [ ] Render strokes
- [ ] Save annotations to PDF
- [ ] Load annotations from PDF
- [ ] **Terminal test:** `--list-annotations file.pdf` outputs JSON

### Phase 3.4: Measurements
Wire up existing measurement system.

- [ ] Distance measurement tool
- [ ] Click two points to measure
- [ ] Show measurement label
- [ ] Area measurement tool
- [ ] Click polygon points
- [ ] Show area calculation
- [ ] Scale calibration
- [ ] Click known distance, enter real value
- [ ] Apply scale to page
- [ ] Export measurements to CSV
- [ ] **Terminal test:** `--export-measurements file.pdf` outputs CSV

---

## Priority 4: Performance & Polish

### Phase 4.1: Startup Optimization
- [ ] Profile startup time
- [ ] Lazy load subsystems
- [ ] Show splash/loading on cold start
- [ ] Target: <200ms to first frame
- [ ] Target: <500ms to first PDF page visible

### Phase 4.2: Large PDF Handling
- [ ] Test with 500+ page PDFs
- [ ] Test with 100MB+ PDFs
- [ ] Ensure no UI freezes
- [ ] Progressive page loading
- [ ] Memory usage stays bounded

### Phase 4.3: Polish
- [ ] Consistent visual style
- [ ] Dark mode support
- [ ] Retina display support (ProMotion 120Hz)
- [ ] Target: 120fps on ProMotion displays, 60fps minimum
- [ ] Smooth scrolling with no frame drops
- [ ] Proper error messages

---

## Testing Strategy

### Running Tests Now

```bash
# Full test suite (unit + integration + smoke + visual)
./scripts/test-all.sh

# Just unit tests
./scripts/run-tests.sh

# Just GUI smoke tests
./scripts/run-gui-tests.sh
```

### Test Layers

| Layer | Tool | Speed | What it catches |
|-------|------|-------|-----------------|
| Unit | `cargo test` | Fast | Logic bugs, regressions |
| Integration | CLI + stdout | Fast | PDF loading, rendering failures |
| Smoke | AppleScript | Medium | App crashes, window issues |
| Render | Process stability | Medium | Crashes during rendering |

Note: Screenshot-based visual testing requires Screen Recording permission (manual approval in System Preferences). The test script skips screenshots when running non-interactively (e.g., via ralphy).

### Automated Terminal Tests
Each feature should have CLI test mode:

```bash
# Test PDF loading ✅ IMPLEMENTED
cargo run --release -- --test-load file.pdf
# Expected output: "LOAD: OK pages=10 time=50ms"
# Also available: scripts/test-multiple-pdfs.sh for batch testing

# Test page rendering
cargo run --release -- --test-render file.pdf --page 1
# Expected output: "RENDER: OK page=1 size=612x792 time=100ms"

# Test text extraction
cargo run --release -- --extract-text file.pdf --page 1
# Expected output: page text to stdout

# Test search
cargo run --release -- --search "keyword" file.pdf
# Expected output: "FOUND: page=3 count=5"

# Test annotation export
cargo run --release -- --list-annotations file.pdf
# Expected output: JSON array of annotations

# Test measurement export  
cargo run --release -- --export-measurements file.pdf
# Expected output: CSV data
```

### Visual Testing Options

**Option A: CLI render-to-file (no permissions needed)**
Add `--render-to-png` flag that saves rendered page directly to file:
```bash
cargo run --release -- --render-to-png page1.png test.pdf
# Saves rendered page 1 to page1.png without opening window
```

**Option B: Window capture script**
```bash
# Requires Screen Recording permission for Terminal.app
./scripts/capture-window.sh pdf-editor screenshot.png
```

**Option C: Interactive only**
```bash
./scripts/test-all.sh  # Takes screenshots if running in terminal
SKIP_SCREENSHOT=1 ./scripts/test-all.sh  # Skip for automation
```

### Manual Test Checklist
Before each release:
- [ ] Fresh install on clean macOS
- [ ] Open 1-page PDF
- [ ] Open 100-page PDF
- [ ] Open scanned (image-only) PDF
- [ ] Navigate all pages
- [ ] Zoom in/out
- [ ] Select and copy text
- [ ] Search for text
- [ ] Add annotation
- [ ] Save and reopen
- [ ] Export to PDF

---

## Success Criteria

The editor is **"MVP complete"** when:
1. ✅ Opens from Finder (double-click .app)
2. ✅ Opens any valid PDF file
3. ✅ Displays pages correctly
4. ✅ Has working menu bar
5. ✅ Has working toolbar
6. ✅ Has page thumbnails
7. ✅ Text selection works
8. ✅ Search works
9. ✅ Can save annotations
10. ✅ No crashes during normal use

---

## File Structure for New Components

```
crates/
├── app/
│   └── src/
│       ├── main.rs          # Entry point
│       ├── menu.rs          # macOS menu bar
│       └── app_state.rs     # Global app state
├── ui/
│   └── src/
│       ├── toolbar.rs       # NEW: Toolbar component
│       ├── button.rs        # NEW: Clickable button
│       ├── text_input.rs    # NEW: Text input field
│       ├── search_bar.rs    # NEW: Search UI
│       ├── thumbnail.rs     # EXISTS: Wire up
│       ├── compositor.rs    # EXISTS: Wire up
│       └── ...
└── ...
```
