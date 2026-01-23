//! Icon-only button component for close buttons, navigation, and actions.
//!
//! Provides a standardized icon button with consistent sizing and hover states.

use gpui::{div, prelude::*, ClickEvent, SharedString, Window};

use crate::components::icon::{icon, Icon};
use crate::ui::{sizes, StatefulInteractiveExt};
use crate::Theme;

/// Size variants for icon buttons.
#[derive(Clone, Copy, Default)]
pub enum IconButtonSize {
    /// Small: 16x16px (for inline use like tab close)
    Sm,
    /// Medium: 20x20px
    #[default]
    Md,
    /// Large: 24x24px (for navigation buttons)
    Lg,
}

impl IconButtonSize {
    /// Returns the container size and icon font size for this variant.
    fn dimensions(self) -> (gpui::Pixels, f32) {
        match self {
            IconButtonSize::Sm => (sizes::ICON_SM, 12.0),
            IconButtonSize::Md => (sizes::ICON_MD, 14.0),
            IconButtonSize::Lg => (sizes::ICON_LG, 16.0),
        }
    }
}

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
///     IconButtonSize::Sm,
///     theme,
///     |_, _, _| println!("clicked"),
/// )
/// ```
pub fn icon_button<F>(
    id: impl Into<SharedString>,
    icon_type: Icon,
    size: IconButtonSize,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let (container_size, icon_size) = size.dimensions();
    let text_color = theme.text_muted;
    let hover_bg = theme.element_hover;
    let active_bg = theme.element_selected;

    div()
        .id(id.into())
        .w(container_size)
        .h(container_size)
        .flex()
        .items_center()
        .justify_center()
        .rounded(sizes::RADIUS_SM)
        .cursor_pointer()
        .text_color(text_color)
        .interactive_bg(hover_bg, active_bg)
        .on_click(on_click)
        .child(icon(icon_type, icon_size, text_color))
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
///     IconButtonSize::Lg,
///     can_go_back,
///     theme,
///     |_, _, _| go_back(),
/// )
/// ```
pub fn icon_button_conditional<F>(
    id: impl Into<SharedString>,
    icon_type: Icon,
    size: IconButtonSize,
    enabled: bool,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let (container_size, icon_size) = size.dimensions();
    let text_enabled = theme.text;
    let text_disabled = theme.text_muted;
    let hover_bg = theme.element_hover;
    let active_bg = theme.element_selected;

    let text_color = if enabled { text_enabled } else { text_disabled };

    div()
        .id(id.into())
        .w(container_size)
        .h(container_size)
        .flex()
        .items_center()
        .justify_center()
        .rounded(sizes::RADIUS_SM)
        .text_color(text_color)
        .when(enabled, move |d| {
            d.cursor_pointer()
                .interactive_bg(hover_bg, active_bg)
                .on_click(on_click)
        })
        .child(icon(icon_type, icon_size, text_color))
}
