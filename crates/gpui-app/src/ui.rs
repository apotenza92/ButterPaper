//! Shared UI constants and components for consistent styling across the app

#![allow(dead_code)]

use gpui::{
    div, prelude::*, px, InteractiveElement, IntoElement, Rgba, SharedString,
    StatefulInteractiveElement, Styled,
};

#[allow(unused_imports)]
pub use crate::styles::{rems_from_px, DynamicSpacing, TextSize};

/// Standard UI sizing constants
pub mod sizes {
    use gpui::{px, Pixels};

    // ============================================
    // Spacing Scale (base unit: 4px)
    // ============================================
    pub const SPACE_0: Pixels = px(0.0);
    pub const SPACE_1: Pixels = px(4.0); // xs
    pub const SPACE_2: Pixels = px(8.0); // sm
    pub const SPACE_3: Pixels = px(12.0); // md
    pub const SPACE_4: Pixels = px(16.0); // lg
    pub const SPACE_5: Pixels = px(20.0); // xl
    pub const SPACE_6: Pixels = px(24.0); // 2xl

    // ============================================
    // Component Heights
    // ============================================
    /// Title bar height - ALL windows use 32px transparent title bars for consistent mouse automation
    pub const TITLE_BAR_HEIGHT: Pixels = px(32.0);
    /// Alias for spec compatibility
    pub const TITLEBAR_HEIGHT: Pixels = px(32.0);

    /// Tab bar height - content area below titlebar
    pub const TAB_BAR_HEIGHT: Pixels = px(41.0);
    /// Tab height within tab bar
    pub const TAB_HEIGHT: Pixels = px(32.0);
    /// In-window app menu row height.
    pub const MENU_ROW_HEIGHT: Pixels = px(29.0);
    /// Canvas toolbar height above the PDF viewport.
    pub const CANVAS_TOOLBAR_HEIGHT: Pixels = px(40.0);
    /// Left tool rail width.
    pub const TOOL_RAIL_WIDTH: Pixels = px(41.0);
    /// Horizontal inset for toolbar content.
    pub const TOOLBAR_INSET_X: Pixels = px(4.0);
    /// Spacing between left/center/right toolbar zones.
    pub const TOOLBAR_ZONE_GAP: Pixels = px(4.0);
    /// Spacing between controls inside a toolbar cluster.
    pub const TOOLBAR_CLUSTER_INNER_GAP: Pixels = px(4.0);
    /// Top inset used by left tool rail controls.
    pub const TOOL_RAIL_TOP_INSET: Pixels = px(4.0);

    /// Zed-style button ladder heights.
    pub const CONTROL_HEIGHT_NONE: Pixels = px(16.0);
    pub const CONTROL_HEIGHT_COMPACT: Pixels = px(18.0);
    pub const CONTROL_HEIGHT_DEFAULT: Pixels = px(22.0);
    pub const CONTROL_HEIGHT_MEDIUM: Pixels = px(28.0);
    pub const CONTROL_HEIGHT_LG: Pixels = px(32.0);
    /// Legacy alias retained during migration; maps to medium controls.
    pub const CONTROL_HEIGHT: Pixels = CONTROL_HEIGHT_MEDIUM;

    /// Standard dropdown button width - consistent across all dropdowns
    /// Must fit long theme names like "Gruvbox Dark Hard"
    pub const DROPDOWN_WIDTH: Pixels = px(180.0);
    pub const MENU_WIDTH_MIN: Pixels = px(180.0);
    pub const MENU_WIDTH_MAX: Pixels = px(260.0);

    /// Tab metrics
    pub const TAB_MIN_WIDTH: Pixels = px(120.0);
    pub const TAB_MAX_WIDTH: Pixels = px(220.0);
    pub const TAB_CLOSE_SIZE: Pixels = px(18.0);
    pub const TAB_ACTIVE_UNDERLINE_HEIGHT: Pixels = px(2.0);

    /// Toggle/radio/checkbox metrics
    pub const TOGGLE_WIDTH: Pixels = px(44.0);
    pub const TOGGLE_KNOB_SIZE: Pixels = px(18.0);
    pub const TOGGLE_KNOB_OFFSET: Pixels = px(3.0);
    pub const RADIO_SIZE: Pixels = px(16.0);
    pub const RADIO_DOT_SIZE: Pixels = px(7.0);
    pub const CHECKBOX_SIZE: Pixels = px(16.0);

    /// Slider metrics
    pub const SLIDER_BUTTON_SIZE: Pixels = px(24.0);
    pub const SLIDER_TRACK_WIDTH: Pixels = px(140.0);
    pub const SLIDER_TRACK_HEIGHT: Pixels = px(8.0);
    pub const SLIDER_VALUE_MIN_WIDTH: Pixels = px(44.0);
    pub const SLIDER_ICON_SIZE: f32 = 14.0;

    /// Dropdown metrics
    pub const DROPDOWN_MAX_HEIGHT: Pixels = px(240.0);

    /// Editor toolbar metrics
    pub const TOOLBAR_CONTROL_SIZE: Pixels = TAB_HEIGHT;
    /// Min width for zoom combo: supports 5 chars of zoom text (e.g. `1600%`) plus chevron.
    pub const ZOOM_COMBO_MIN_WIDTH: Pixels = px(72.0);
    pub const PAGE_LABEL_MIN_WIDTH: Pixels = px(68.0);
    pub const PAGE_LABEL_HORIZONTAL_PADDING: Pixels = px(4.0);
    pub const TOOLBAR_SEPARATOR_WIDTH: Pixels = px(1.0);
    pub const TOOLBAR_SEPARATOR_HEIGHT: Pixels = px(16.0);

    // ============================================
    // Icon Sizes
    // ============================================
    pub const ICON_SM: Pixels = px(16.0);
    pub const ICON_MD: Pixels = px(20.0);
    pub const ICON_LG: Pixels = px(24.0);

    // ============================================
    // Padding (legacy aliases for SPACE_*)
    // ============================================
    pub const PADDING_SM: Pixels = px(4.0);
    pub const PADDING_MD: Pixels = px(8.0);
    pub const PADDING_LG: Pixels = px(12.0);
    pub const PADDING_XL: Pixels = px(16.0);
    pub const PADDING_2XL: Pixels = px(24.0);
    pub const PADDING_3XL: Pixels = px(32.0);

    // ============================================
    // Gap (layout spacing)
    // ============================================
    pub const GAP_SM: Pixels = px(4.0);
    pub const GAP_MD: Pixels = px(8.0);
    pub const GAP_LG: Pixels = px(16.0);
    pub const GAP_XL: Pixels = px(24.0);

    // ============================================
    // Border Radius
    // ============================================
    pub const RADIUS_SM: Pixels = px(7.0);
    pub const RADIUS_MD: Pixels = px(10.0);
    pub const RADIUS_LG: Pixels = px(14.0);

    // ============================================
    // Stroke / Opacity Tokens
    // ============================================
    /// Alpha multiplier for subtle control borders.
    pub const BORDER_ALPHA_SUBTLE: f32 = 0.45;
    /// Alpha multiplier for stronger border emphasis.
    pub const BORDER_ALPHA_STRONG: f32 = 0.8;
    /// Default disabled content alpha multiplier.
    pub const DISABLED_ALPHA: f32 = 0.7;

    // ============================================
    // Layout Widths
    // ============================================
    /// Sidebar width
    pub const SIDEBAR_WIDTH: Pixels = px(220.0);

    /// Settings content max width (prevents overflow)
    pub const SETTINGS_CONTENT_MAX_WIDTH: Pixels = px(600.0);
}

// ============================================
// Interactive Element Extensions
// ============================================

/// Extension trait for interactive elements with hover states.
/// Provides consistent styling for buttons, links, and other clickable elements.
pub trait InteractiveExt: InteractiveElement + Styled + Sized {
    /// Apply hover background color for interactive elements.
    fn hover_bg(self, hover: Rgba) -> Self {
        self.hover(move |s| s.bg(hover))
    }

    /// Apply default and hover text colors for interactive text elements.
    fn interactive_text(self, default: Rgba, hover: Rgba) -> Self {
        self.text_color(default).hover(move |s| s.text_color(hover))
    }
}

impl<T: InteractiveElement + Styled> InteractiveExt for T {}

/// Extension trait for stateful interactive elements with hover AND active states.
/// Use this for elements that have an id() and support active (pressed) state styling.
pub trait StatefulInteractiveExt: StatefulInteractiveElement + Styled + Sized {
    /// Apply hover and active background colors for interactive elements.
    fn interactive_bg(self, hover: Rgba, active: Rgba) -> Self {
        self.hover(move |s| s.bg(hover)).active(move |s| s.bg(active))
    }
}

impl<T: StatefulInteractiveElement + Styled> StatefulInteractiveExt for T {}

/// Standard text sizes following Zed conventions
pub mod text {
    pub const UI_META: &str = "text_xs";
    pub const UI_BODY: &str = "text_sm";
    pub const UI_ICON_LABEL: &str = "text_base";
    pub const UI_TITLE: &str = "text_xl";
}

/// Semantic typography helpers for app UI surfaces.
pub trait TypographyExt: Styled + Sized {
    /// Body text used by controls, menu labels, and standard UI copy.
    fn text_ui_body(self) -> Self {
        self.text_sm()
    }

    /// Secondary/meta text used by compact labels and shortcuts.
    fn text_ui_meta(self) -> Self {
        self.text_xs()
    }

    /// Icon-like textual glyph sizing in compact UI rows.
    fn text_ui_icon(self) -> Self {
        self.text_base()
    }

    /// Prominent section titles.
    fn text_ui_title(self) -> Self {
        self.text_xl()
    }
}

impl<T: Styled + Sized> TypographyExt for T {}

/// Shared color helpers for standardized control styling.
pub mod color {
    use gpui::Rgba;

    use crate::ui::sizes;

    pub fn with_alpha(color: Rgba, alpha_multiplier: f32) -> Rgba {
        Rgba { r: color.r, g: color.g, b: color.b, a: color.a * alpha_multiplier }
    }

    pub fn subtle_border(color: Rgba) -> Rgba {
        with_alpha(color, sizes::BORDER_ALPHA_SUBTLE)
    }

    pub fn strong_border(color: Rgba) -> Rgba {
        with_alpha(color, sizes::BORDER_ALPHA_STRONG)
    }

    pub fn disabled(color: Rgba) -> Rgba {
        with_alpha(color, sizes::DISABLED_ALPHA)
    }
}

/// Create a centered title bar for transparent titlebar windows.
/// Title is centered horizontally and vertically within the 32px title bar area.
/// Includes a bottom border line using the theme border color.
pub fn title_bar(
    title: impl Into<SharedString>,
    text_color: impl Into<Rgba>,
    border_color: impl Into<Rgba>,
) -> impl IntoElement {
    let text_color = text_color.into();
    let border_color = border_color.into();
    div()
        .h(sizes::TITLE_BAR_HEIGHT)
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .border_b_1()
        .border_color(border_color)
        .child(
            div()
                .text_size(px(14.0))
                .text_color(text_color)
                .mt(px(1.5)) // Optical adjustment
                .child(title.into()),
        )
}
