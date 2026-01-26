# Agent Guidelines

## CRITICAL LAYOUT RULES (DO NOT VIOLATE)

**NEVER put UI content in the titlebar area.** The titlebar is sacred and must remain clean.

### Titlebar Rules:
1. The 32px titlebar is ONLY for the macOS traffic lights (● ● ●) - nothing else
2. NO text, labels, buttons, tabs, or any UI elements in the titlebar
3. ALL content starts BELOW the 32px titlebar with `pt(px(32.0))` padding
4. The sidebar, tabs, navigation buttons - everything goes BELOW the titlebar

### Correct Window Layout:
```
┌────────────────────────────────────────┐
│ ● ● ●     [empty titlebar - 32px]      │  ← NOTHING here except traffic lights
├─────────┬──────────────────────────────┤
│ Sidebar │  Tab Bar                     │  ← Content starts HERE (below titlebar)
│         ├──────────────────────────────┤
│         │  Viewport                    │
│         │                              │
└─────────┴──────────────────────────────┘
```

### WRONG (Never do this):
```
┌─────────┬──────────────────────────────┐
│ ● ● ● X │  ← → [tabs]                  │  ← WRONG: Content in titlebar
├─────────┼──────────────────────────────┤
```

This is a **hard constraint** - do not attempt to make the UI "look like Zed" by putting content in the titlebar.

---

## Code Philosophy

- **Native first**: Always prefer native platform APIs and official plugin solutions over hacks or workarounds
- **CLI-first architecture**: Design all features to be usable via command line for LLM/automation compatibility
- Embrace each platform's conventions (macOS traffic lights, Windows title bar, etc.)

## Visual Design: Follow Zed's Patterns

**Default to copying Zed's visual design** as a starting point for all UI components. Zed is built with GPUI and provides excellent patterns for native desktop UI.

### Zed UI Component Patterns

**Layout helpers:**
- Use `h_flex()` / `v_flex()` style patterns (horizontal/vertical flex containers)
- `div().flex().flex_row()` for horizontal, `div().flex().flex_col()` for vertical

**Spacing system:**
- `gap_1()` = 4px, `gap_2()` = 8px, `gap_4()` = 16px
- `px_8()` = 32px horizontal padding for content areas
- `py_1()` = 4px, `py_2()` = 8px vertical padding

**Settings UI pattern (see `settings.rs`):**
- Two-column layout: sidebar navigation (200px) + content area
- Setting items: title + description on left, control on right
- Use `justify_between()` to spread label and control apart
- Horizontal dividers between items with `border_b_1().border_color(theme.border)`

**Setting item structure:**
```rust
div()
    .flex().flex_row().justify_between().gap(px(16.0))
    .child(
        div().flex().flex_col().gap(px(2.0))
            .child(div().text_sm().child("Title"))
            .child(div().text_xs().text_color(theme.text_muted).child("Description"))
    )
    .child(control)
```

**Dropdown buttons:**
- Border with `border_1().border_color(theme.border)`
- Rounded corners with `rounded(px(4.0))`
- Height `h(px(28.0))` for controls
- Chevron indicator (▼) in muted color
- Dropdown menu: absolute positioned, shadow, rounded corners

**Toggle/Switch controls:**
- Zed uses `Switch` component with `ToggleState::Selected/Unselected`
- For simple cases, use checkmark (✓) in accent color for selected state

**Text styling:**
- `text_sm()` for primary labels
- `text_xs()` for descriptions and secondary text
- `text_color(theme.text_muted)` for secondary/muted text
- `font_weight(FontWeight::MEDIUM)` for emphasis

**Navigation items:**
- Height `h(px(28.0))`
- Padding `px(px(8.0))`
- Rounded `rounded(px(4.0))`
- Selected state: `bg(theme.element_selected)`
- Hover state: `hover(|s| s.bg(theme.element_hover))`

**Window patterns:**
- Settings windows: `appears_transparent: true` with traffic light positioning
- Traffic lights: `traffic_light_position: Some(point(px(12.0), px(12.0)))`
- Content padding top `pt(px(48.0))` to clear title bar area

### Theme Colors (from `theme.rs`)

The app uses Zed's actual theme colors:
- **One Light / One Dark** - Popular Atom-style themes
- **Sand Light / Sand Dark** - Zed's default neutral themes

Key color properties:
- `background` - Main window background
- `surface` - Panel/sidebar background
- `elevated_surface` - Content area background
- `text` / `text_muted` - Primary and secondary text
- `border` - Border color
- `accent` - Selection and focus color
- `element_hover` / `element_selected` - Interactive element states

## Project Structure

```
crates/
├── gpui-app/      # GPUI native app (GPU-accelerated)
├── render/        # PDF rendering with pdfium

scripts/           # Build scripts
specs/             # Architecture specs
```

## Build Commands

```bash
# Build
./scripts/build-gpui.sh

# Or directly with cargo
cargo build --release -p butterpaper-gpui

# Run with a PDF file
./target/release/butterpaper path/to/file.pdf

# Show help
./target/release/butterpaper --help
```

## Runtime Requirements

- `libpdfium.dylib` must be in the same directory as the binary
- Build script automatically copies it to `target/release/`

## GPUI Development Patterns

**Image rendering (CRITICAL):**
- GPUI requires BGRA pixel format, not RGBA
- Always swap R and B channels before creating RenderImage:
  ```rust
  for pixel in pixels.chunks_exact_mut(4) {
      pixel.swap(0, 2);  // RGBA -> BGRA
  }
  ```

**Creating GPU textures:**
```rust
use smallvec::SmallVec;
let buffer = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(width, height, bgra_pixels)?;
let frame = image::Frame::new(buffer);
let render_image = Arc::new(gpui::RenderImage::new(SmallVec::from_elem(frame, 1)));
// Use with: img(ImageSource::Render(render_image))
```

**Entity API:**
- `Entity::read(cx)` returns `&T` directly
- `Entity::update(cx, |this, cx| { ... })` for mutations

**Async spawning:**
```rust
cx.spawn(|this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| async move {
    this.update(cx, |view, cx| { ... }).ok();
})
.detach();
```

**Actions and keybindings:**
```rust
actions!(app_name, [ActionName, ...]);
cx.bind_keys([KeyBinding::new("cmd-o", ActionName, Some("KeyContext"))]);
// In Render: .on_action(cx.listener(Self::handle_action))
```

**File dialogs:**
```rust
let future = cx.prompt_for_paths(gpui::PathPromptOptions { ... });
// Returns Result<Result<Option<Vec<PathBuf>>, Error>, Canceled>
// Check with: if let Ok(Ok(Some(paths))) = future.await { ... }
```

## Thread Safety Note

PdfDocument (pdfium-render) is NOT Send+Sync due to raw pointers. Cannot use background threads for PDF rendering. Use:
- Aggressive caching (LRU, keyed by page+zoom)
- Deferred rendering (limit renders per frame)
- Virtual scrolling (only render visible pages)

## PDF Render Crate

The `crates/render` crate provides the PDF rendering API:

```rust
use butterpaper_render::PdfDocument;

let doc = PdfDocument::open(&path)?;
let page_count = doc.page_count();  // u16
let page = doc.get_page(index)?;    // PdfPage with .width().value, .height().value
let pixels = doc.render_page_rgba(page_index, width, height)?;  // Vec<u8> RGBA
```

## Testing

```bash
# Test with sample PDF
./target/release/butterpaper test.pdf

# Check help
./target/release/butterpaper --help
```

## Mouse Automation & Coordinate System

**CRITICAL**: ALL GPUI windows in this app use **32px transparent title bars** for consistent mouse automation.

### Window Configuration Standard

All windows MUST be configured with:
```rust
titlebar: Some(TitlebarOptions {
    title: Some("Window Title".into()),
    appears_transparent: true,
    traffic_light_position: Some(point(px(12.0), px(9.0))),
}),
```

All window content MUST have 32px top padding:
```rust
div()
    .pt(ui::sizes::TITLE_BAR_HEIGHT) // 32px
    // ... rest of content
```

### Coordinate System Overview

There are THREE coordinate systems at play:

1. **Screen coordinates**: Absolute position on the display (0,0 = top-left of screen)
2. **Window coordinates (xcap)**: Position reported by xcap - this is the top-left of the **title bar**
3. **Content-area coordinates (GPUI)**: Window-relative coords used by GPUI mouse events - starts at top-left of **content area** (below the 32px title bar padding)

### Coordinate Mapping

With transparent titlebar:
- xcap reports window position as top-left of the entire window (including title bar area)
- xcap captures the full window content including the 32px title bar area
- GPUI coordinates start at (0,0) at the top-left of the window (including title bar)
- Measurements from xcap screenshots are already in GPUI coordinates

### Formula for Clicking Elements

```
screen_x = window_x + gpui_x
screen_y = window_y + gpui_y

(No additional offset needed - xcap and GPUI use the same coordinate origin)
```

### Example

```
Window at (100, 100) from xcap
Element at GPUI (200, 232) - includes 32px title bar in Y
Screen position to click = (100 + 200, 100 + 232) = (300, 332)
```

### Measuring Element Positions

1. Use `xcap` to capture the window (captures content area only, no title bar)
2. Image is at 2x retina resolution, so divide pixel coords by 2 for points
3. These measurements are content-area relative (use directly with GPUI)

### CLI Tools for Mouse Automation

```bash
# List windows with positions
butterpaper --list-windows

# List UI elements with clickable coordinates
butterpaper --list-elements

# Focus a window (required before clicking)
butterpaper --focus --window-title Settings

# Click a UI element by ID
butterpaper --click-element settings.appearance --window-title Settings

# Calibration tool to verify coordinates
./target/release/calibrate
```

### Calibration Tool

The `calibrate` binary creates a window with markers at known positions. Use it to verify coordinate calculations:

1. Run `./target/release/calibrate`
2. Move mouse to a marker using `test-enigo <x> <y>`
3. Take screenshot with cursor: `screencapture -C /tmp/test.png`
4. Verify cursor is on the marker
5. Click and check the "Clicks:" footer shows expected coordinates

### Common Mistakes

- ❌ Forgetting the 32px title bar offset
- ❌ Using xcap window position directly without offset
- ❌ Measuring from screenshots that include the title bar
- ❌ Not focusing the window before clicking (window must be frontmost)

### Platform-Specific Window Focus

- **macOS**: Uses AppleScript with `AXRaise` to bring window to front
- **Windows**: Uses PowerShell with `SetForegroundWindow`
- **Linux**: Uses `xdotool windowactivate` or `wmctrl -i -a`

## Dev Mode & Dynamic Element Tracking

The app has a dev mode (`--dev` flag) for agentic automation that tracks UI element positions dynamically. **This is disabled by default for performance.**

### Design Principles

1. **Opt-in only**: Dev mode features are gated behind `element_registry::is_dev_mode()`
2. **Zero cost when disabled**: No element tracking overhead for regular users
3. **Dynamic over static**: Element positions are captured during render, not hardcoded
4. **Requires running window**: CLI automation commands need the target window open

### Usage

```bash
# Start app with dev mode enabled
./target/release/butterpaper --settings --dev

# Then in another terminal, list elements (requires window to be rendered)
./target/release/butterpaper --list-elements

# Click an element
./target/release/butterpaper --click-element settings.dark_theme --window-title Settings
```

### Adding Trackable Elements

When building new UI components that should be automatable:

```rust
use crate::element_registry::{register_from_bounds, ElementType, is_dev_mode};

// In your component, wrap clickable elements to track bounds:
div()
    .id("my-button")
    .on_children_prepainted(move |bounds, _window, _cx| {
        // Only registers if dev mode is enabled - zero cost otherwise
        register_from_bounds("component.button_id", "Button Name", &bounds, ElementType::Button, "WindowTitle");
    })
    .child(/* button content */)
```

### Element Types

- `ElementType::Button` - Clickable buttons
- `ElementType::Dropdown` - Dropdown selectors  
- `ElementType::NavItem` - Navigation items

### Why This Approach

- **Performance**: Real users don't pay for automation infrastructure
- **Accuracy**: Dynamic tracking means positions are always correct, even after UI changes
- **Maintainability**: No hardcoded coordinates to update when layouts change
