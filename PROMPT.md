# ButterPaper: Multi-Tab Enhancement Specification

## Overview

Enhance the ButterPaper's tab bar to support:
1. Drag and drop PDF files to open in new tabs
2. A "+" button to create new welcome tabs
3. Double-click in empty tab bar area to create new welcome tabs
4. Proper tab bar scrolling with many tabs open

## Reference: Zed Editor

Use **Zed editor** as the behavioral reference for tab styling and interactions. This app uses GPUI (Zed's UI framework), so we should match Zed's patterns where applicable:

- **Drag and drop**: Files dropped anywhere on window open in new tabs
- **Double-click empty area**: Creates new tab
- **Tab overflow**: Horizontal scroll, no tab truncation, smooth scrolling
- **Visual style**: Already matches Zed's elevated active tab, bordered inactive tabs

**Differences from Zed** (intentional):
- **"+" button**: We ARE adding a "+" button after the last tab (Zed doesn't have this, but we want it for discoverability)
- **Welcome tab**: Opens a welcome screen tab, not a blank buffer

When in doubt about styling or interaction feel, follow what Zed does.

## Codebase Context

- **Framework**: GPUI (Zed's UI framework in Rust)
- **Main editor**: `crates/gpui-app/src/app/editor.rs` - contains `PdfEditor` struct and tab management
- **Tab data**: `crates/gpui-app/src/app/document.rs` - contains `DocumentTab` struct
- **Components**: `crates/gpui-app/src/components/` - UI components including icons

## Current State

- Tabs stored in `Vec<DocumentTab>` with `active_tab_index`
- Tab bar has horizontal scrolling via `.overflow_x_scroll()`
- File opening via `open_file(path: PathBuf, cx)` method
- Welcome screen shown when no tabs open (inline in render)
- **No drag/drop file handling exists**

## Requirements

### 1. Welcome Tab Concept

Create a "welcome tab" that shows the welcome screen content (Open File button) but as a tab:

- Add `LoadingState::Welcome` variant to represent welcome tabs
- Change `DocumentTab.path` from `PathBuf` to `Option<PathBuf>` (None = welcome tab)
- Add `DocumentTab::is_welcome()` helper method
- Welcome tabs display "Welcome" as title
- When opening a file from a welcome tab, convert it to a document tab (reuse the tab)

### 2. Drag and Drop File Handling

Add support for dropping PDF files onto the editor window:

- Import `ExternalPaths` from gpui
- Add `.on_drop::<ExternalPaths>()` handler to the main editor div
- Filter for `.pdf` files only (case-insensitive)
- Call `open_file()` for each dropped PDF
- Multiple files can be dropped at once

### 3. "+" Button in Tab Bar

Add a "+" button after the last tab:

- Use `Icon::Plus` with `icon_button()` component
- Clicking creates a new welcome tab via `new_tab()` method
- Style: ghost/subtle variant, same height as tabs
- Position: immediately after the last tab in the tab bar
- When tabs overflow and scroll, the "+" button should remain at the end of the tab list (scrolls with tabs)

### 4. Double-Click in Empty Tab Bar Area

The empty space after tabs (the fill div with bottom border) should:

- Respond to double-click to create a new welcome tab
- Use `.on_mouse_down()` and check for `click_count == 2`
- Call `new_tab()` method

### 5. Tab Bar Overflow Behavior

Verify and ensure:
- Tab bar scrolls horizontally when tabs exceed width
- All tabs maintain full width (no truncation)
- "+" button scrolls with the tabs
- Native scrollbar appears when needed

## Implementation Order

1. Update `document.rs`: Add `LoadingState::Welcome`, make `path` optional, add `is_welcome()` method
2. Update `editor.rs`: Add `create_welcome_tab()` and `new_tab()` methods
3. Update `open_file()`: Reuse welcome tabs when opening files
4. Add `handle_file_drop()` method and `.on_drop::<ExternalPaths>()` handler
5. Add "+" button to tab bar rendering
6. Add double-click handler to empty tab bar area
7. Test with multiple tabs to verify scrolling

## GPUI API Reference

```rust
// File drop handling
.on_drop::<ExternalPaths>(|paths, window, cx| {
    // paths.paths() returns &[PathBuf]
})

// Double-click detection
.on_mouse_down(MouseButton::Left, |event, window, cx| {
    if event.click_count == 2 {
        // Handle double-click
    }
})

// Icon button (ghost variant for subtle appearance)
icon_button("new-tab", Icon::Plus, IconButtonSize::Md, theme, |_, _, cx| {
    // Handler
})
```

## Verification

Build and test:
```bash
cargo build -p butterpaper-gpui
cargo run -p butterpaper-gpui
```

Test scenarios:
1. Launch app - should show welcome screen (no tabs)
2. Drag a PDF file onto the window - should open in new tab
3. Drag a second PDF - should open in another tab
4. Click "+" button - should create welcome tab
5. Double-click empty tab bar area - should create welcome tab
6. Open many files - tab bar should scroll horizontally
7. From welcome tab, open file - tab should convert to document tab

## Completion Signal

When all requirements are implemented, tests pass, and the app builds successfully:

LOOP_COMPLETE
