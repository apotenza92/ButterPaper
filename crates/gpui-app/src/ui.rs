//! Shared UI constants and components for consistent styling across the app

#![allow(dead_code)]

use gpui::{div, prelude::*, px, IntoElement, Rgba, SharedString};

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
    pub const TAB_BAR_HEIGHT: Pixels = px(32.0);
    /// Tab height within tab bar
    pub const TAB_HEIGHT: Pixels = px(28.0);

    /// Standard control heights
    pub const CONTROL_HEIGHT: Pixels = px(28.0);

    /// Standard dropdown button width - consistent across all dropdowns
    /// Must fit long theme names like "Gruvbox Dark Hard"
    pub const DROPDOWN_WIDTH: Pixels = px(180.0);

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
    pub const RADIUS_SM: Pixels = px(4.0);
    pub const RADIUS_MD: Pixels = px(6.0);
    pub const RADIUS_LG: Pixels = px(8.0);

    // ============================================
    // Layout Widths
    // ============================================
    /// Sidebar width
    pub const SIDEBAR_WIDTH: Pixels = px(220.0);

    /// Settings content max width (prevents overflow)
    pub const SETTINGS_CONTENT_MAX_WIDTH: Pixels = px(600.0);
}

/// Standard text sizes following Zed conventions
pub mod text {
    pub const SIZE_XS: &str = "text_xs";
    pub const SIZE_SM: &str = "text_sm";
    pub const SIZE_BASE: &str = "text_base";
    pub const SIZE_LG: &str = "text_lg";
    pub const SIZE_XL: &str = "text_xl";
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
