# UI Components API

This document describes the reusable UI components in `src/components/`.

All components accept a `Theme` reference for consistent styling and use constants from `ui::sizes` for consistent sizing.

## Table of Contents

- [Button](#button)
- [IconButton](#iconbutton)
- [Card](#card)
- [Input](#input)
- [Icon](#icon)
- [Tooltip](#tooltip)
- [Toggle Switch](#toggle-switch)
- [Interactive Traits](#interactive-traits)
- [Size Constants](#size-constants)

---

## Button

**File:** `components/button.rs`

Standard button component with variants and sizes.

### Variants

| Variant | Description | Use Case |
|---------|-------------|----------|
| `Default` | Subtle background | Secondary actions |
| `Primary` | Accent color background | Main actions (Save, Submit) |
| `Ghost` | Transparent background | Toolbar actions |
| `Danger` | Red background | Destructive actions (Delete) |

### Sizes

| Size | Height | Padding | Text Size |
|------|--------|---------|-----------|
| `Sm` | 24px | 8px | 12px |
| `Md` | 28px | 12px | 14px |
| `Lg` | 32px | 16px | 14px |

### Functions

#### `button()`

Full control over variant and size.

```rust
use crate::components::{button, ButtonVariant, ButtonSize};

button(
    "save-btn",                    // id
    "Save",                        // label
    ButtonVariant::Primary,        // variant
    ButtonSize::Md,                // size
    theme,                         // &Theme
    cx.listener(|this, _, _, cx| this.save(cx)),  // on_click
)
```

#### `button_default()`

Simplified default button (Default variant, Md size).

```rust
use crate::components::button_default;

button_default("Cancel", theme, |_, _, _| {})
```

#### `button_primary()`

Simplified primary button (Primary variant, Md size).

```rust
use crate::components::button_primary;

button_primary("Submit", theme, |_, _, _| {})
```

---

## IconButton

**File:** `components/icon_button.rs`

Icon-only button for close buttons, navigation, and toolbar actions.

### Sizes

| Size | Container | Icon Size |
|------|-----------|-----------|
| `Sm` | 16x16px | 12px |
| `Md` | 20x20px | 14px |
| `Lg` | 24x24px | 16px |

### Functions

#### `icon_button()`

Standard icon button.

```rust
use crate::components::{icon_button, Icon, IconButtonSize};

icon_button(
    "close-tab-1",           // id
    Icon::Close,             // icon
    IconButtonSize::Sm,      // size
    theme,                   // &Theme
    |_, _, _| println!("clicked"),  // on_click
)
```

#### `icon_button_conditional()`

Icon button with enabled/disabled state.

```rust
use crate::components::{icon_button_conditional, Icon, IconButtonSize};

icon_button_conditional(
    "nav-back",              // id
    Icon::ArrowLeft,         // icon
    IconButtonSize::Lg,      // size
    can_go_back,             // enabled: bool
    theme,                   // &Theme
    |_, _, _| go_back(),     // on_click (only fires when enabled)
)
```

---

## Card

**File:** `components/card.rs`

Container component for consistent panel styling.

### Functions

#### `card()`

Basic card with padding, border, and shadow.

```rust
use crate::components::card;

card(theme, vec![
    div().child("Card content").into_any_element()
])
```

#### `card_header()`

Card header with title and bottom border.

```rust
use crate::components::card_header;

card_header(theme, "Settings")
```

#### `card_with_header()`

Convenience wrapper for card with header.

```rust
use crate::components::card_with_header;

card_with_header(theme, "Appearance", vec![
    div().child("Theme selector").into_any_element(),
    div().child("Font settings").into_any_element(),
])
```

---

## Input

**File:** `components/input.rs`

Text input styling component.

> **Note:** GPUI doesn't have a native text input element. This provides consistent styling for input-like containers. Actual input handling requires GPUI's text input primitives.

### Sizes

| Size | Height | Padding | Text Size |
|------|--------|---------|-----------|
| `Sm` | 24px | SPACE_2 | 12px |
| `Md` | 32px | SPACE_3 | 14px |
| `Lg` | 40px | SPACE_4 | 14px |

### Functions

#### `text_input()`

Full control input with size.

```rust
use crate::components::{text_input, InputSize};

text_input("search-input", "Search...", InputSize::Md, theme)
```

#### `text_input_default()`

Simplified input with default size.

```rust
use crate::components::text_input_default;

text_input_default("search", "Search...", theme)
```

---

## Icon

**File:** `components/icon.rs`

Standardized icon rendering using unicode characters.

### Available Icons

| Icon | Character | Description |
|------|-----------|-------------|
| `Close` | ✕ | Close button |
| `ChevronLeft` | ‹ | Left navigation |
| `ChevronRight` | › | Right navigation |
| `ArrowLeft` | ← | Back navigation |
| `ArrowRight` | → | Forward navigation |
| `Dirty` | • | Unsaved indicator |
| `Settings` | ⚙ | Settings gear |
| `Check` | ✓ | Checkmark |
| `ChevronDown` | ▼ | Dropdown arrow |

### Functions

#### `icon()`

Render an icon with specified size and color.

```rust
use crate::components::{icon, Icon};

icon(Icon::Close, 14.0, theme.text_muted)
```

#### `Icon::as_str()`

Get the unicode character.

```rust
let char = Icon::Check.as_str();  // "✓"
```

---

## Tooltip

**File:** `components/tooltip.rs`

Tooltip component for hover information.

### Usage

```rust
use crate::components::tooltip_builder;

div()
    .id("my-element")
    .tooltip(tooltip_builder("My tooltip text", theme.surface, theme.border))
```

---

## Toggle Switch

**File:** `components/toggle_switch.rs`

Boolean toggle component.

```rust
use crate::components::toggle_switch;

toggle_switch("dark-mode", is_enabled, theme, |value, _, _| {
    set_dark_mode(value)
})
```

---

## Interactive Traits

**File:** `ui.rs`

Extension traits for consistent hover/active states.

### `InteractiveExt`

For elements without an id (hover only).

```rust
use crate::ui::InteractiveExt;

div()
    .hover_bg(theme.element_hover)
```

```rust
div()
    .interactive_text(theme.text_muted, theme.text)  // default, hover
```

### `StatefulInteractiveExt`

For elements with an id (hover AND active states).

```rust
use crate::ui::StatefulInteractiveExt;

div()
    .id("my-button")
    .interactive_bg(theme.element_hover, theme.element_selected)  // hover, active
```

---

## Size Constants

**File:** `ui.rs` (sizes module)

### Spacing Scale (4px base)

| Constant | Value | Use |
|----------|-------|-----|
| `SPACE_0` | 0px | None |
| `SPACE_1` | 4px | xs |
| `SPACE_2` | 8px | sm |
| `SPACE_3` | 12px | md |
| `SPACE_4` | 16px | lg |
| `SPACE_5` | 20px | xl |
| `SPACE_6` | 24px | 2xl |

### Component Heights

| Constant | Value | Use |
|----------|-------|-----|
| `TITLEBAR_HEIGHT` | 32px | Window title bar |
| `TAB_BAR_HEIGHT` | 32px | Tab bar container |
| `TAB_HEIGHT` | 28px | Individual tab |
| `CONTROL_HEIGHT` | 28px | Standard controls |

### Icon Sizes

| Constant | Value |
|----------|-------|
| `ICON_SM` | 16px |
| `ICON_MD` | 20px |
| `ICON_LG` | 24px |

### Border Radius

| Constant | Value |
|----------|-------|
| `RADIUS_SM` | 4px |
| `RADIUS_MD` | 6px |
| `RADIUS_LG` | 8px |

### Usage

```rust
use crate::ui::sizes;

div()
    .p(sizes::SPACE_4)
    .rounded(sizes::RADIUS_MD)
    .h(sizes::CONTROL_HEIGHT)
```

---

## Component Directory Structure

```
components/
├── mod.rs           # Public exports
├── button.rs        # Button with variants
├── card.rs          # Card/panel container
├── divider.rs       # Horizontal/vertical divider
├── dropdown.rs      # Dropdown select
├── icon.rs          # Icon enum and renderer
├── icon_button.rs   # Icon-only button
├── input.rs         # Text input styling
├── nav_item.rs      # Sidebar nav item
├── setting_item.rs  # Settings row
├── tab_bar.rs       # Tab identifiers
├── toggle_switch.rs # Toggle switch
└── tooltip.rs       # Tooltip view
```
