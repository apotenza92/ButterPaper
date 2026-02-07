//! Icon-only button component for close buttons, navigation, and actions.
//!
//! Provides a standardized icon button following Zed's design patterns with
//! consistent sizing, proper hit areas, and hover/active states.
//!
use gpui::{div, prelude::*, ClickEvent, SharedString, Window};

use crate::components::button_like::{
    disabled_text, subtle_border, variant_colors, ButtonLikeColors, ButtonLikeExt,
    ButtonLikeVariant, ButtonSize,
};
use crate::components::icon::{icon, Icon};
use crate::ui::sizes;
use crate::Theme;

/// Create an icon button with the specified icon, size, and click handler.
///
/// # Arguments
/// * `id` - Unique identifier for the button
/// * `icon_type` - The icon to display
/// * `size` - Button size variant
/// * `theme` - Theme for styling
/// * `on_click` - Click handler
///
/// # Example
/// ```ignore
/// icon_button(
///     "close-tab-1",
///     Icon::Close,
///     ButtonSize::Default,
///     theme,
///     |_, _, _| println!("clicked"),
/// )
/// ```
pub fn icon_button<F>(
    id: impl Into<SharedString>,
    icon_type: Icon,
    size: ButtonSize,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let button_size = size.height_px();
    let icon_size = size.icon_size_px();
    let mut colors = variant_colors(ButtonLikeVariant::Neutral, theme);
    colors.text = theme.text;

    div()
        .id(id.into())
        .w(button_size)
        .h(button_size)
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .button_like(colors, sizes::RADIUS_MD)
        .cursor_pointer()
        .on_click(on_click)
        .child(icon(icon_type, icon_size, colors.text))
}

/// Create an icon button that can be conditionally enabled/disabled.
///
/// When disabled, the button has muted text and no hover effects.
///
/// # Example
/// ```ignore
/// icon_button_conditional(
///     "nav-back",
///     Icon::ArrowLeft,
///     ButtonSize::Large,
///     can_go_back,
///     theme,
///     |_, _, _| go_back(),
/// )
/// ```
pub fn icon_button_conditional<F>(
    id: impl Into<SharedString>,
    icon_type: Icon,
    size: ButtonSize,
    enabled: bool,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let button_size = size.height_px();
    let icon_size = size.icon_size_px();
    let text_color = if enabled { theme.text } else { disabled_text(theme) };
    let mut colors = ButtonLikeColors {
        background: theme.elevated_surface,
        text: text_color,
        border: subtle_border(theme),
        hover: theme.element_hover,
        active: theme.element_selected,
    };
    if !enabled {
        colors.hover = colors.background;
        colors.active = colors.background;
    }

    div()
        .id(id.into())
        .w(button_size)
        .h(button_size)
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .button_like(colors, sizes::RADIUS_MD)
        .when(enabled, move |d| d.cursor_pointer().on_click(on_click))
        .child(icon(icon_type, icon_size, text_color))
}
