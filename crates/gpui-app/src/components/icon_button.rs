//! Icon-only button component for close buttons, navigation, and actions.
//!
//! Provides a standardized icon button following Zed's design patterns with
//! consistent sizing, proper hit areas, and hover/active states.
//!
//! # Size Reference (matching Zed button sizes)
//! - Sm: 24px button, 14px icon - for tab close buttons, dense UIs
//! - Md: 28px button, 16px icon - default, general purpose
//! - Lg: 32px button, 18px icon - navigation, toolbars, prominent actions

use gpui::{div, prelude::*, px, ClickEvent, Pixels, SharedString, Window};

use crate::components::icon::{icon, Icon};
use crate::ui::{sizes, StatefulInteractiveExt};
use crate::Theme;

/// Size variants for icon buttons, matching standard Zed button sizes.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum IconButtonSize {
    /// Small: 24x24px button with 14px icon
    Sm,
    /// Medium: 28x28px button with 16px icon (default)
    #[default]
    Md,
    /// Large: 32x32px button with 18px icon
    Lg,
}

impl IconButtonSize {
    /// Returns the button container size (square).
    pub fn button_size(self) -> Pixels {
        match self {
            IconButtonSize::Sm => px(24.0),
            IconButtonSize::Md => px(28.0),
            IconButtonSize::Lg => px(32.0),
        }
    }

    /// Returns the icon size for this button size.
    pub fn icon_size(self) -> f32 {
        match self {
            IconButtonSize::Sm => 14.0,
            IconButtonSize::Md => 16.0,
            IconButtonSize::Lg => 18.0,
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
    let button_size = size.button_size();
    let icon_size = size.icon_size();
    let text_color = theme.text_muted;
    let hover_bg = theme.element_hover;
    let active_bg = theme.element_selected;

    div()
        .id(id.into())
        .w(button_size)
        .h(button_size)
        .flex()
        .flex_shrink_0()
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
    let button_size = size.button_size();
    let icon_size = size.icon_size();
    let text_enabled = theme.text;
    let text_disabled = theme.text_muted;
    let hover_bg = theme.element_hover;
    let active_bg = theme.element_selected;

    let text_color = if enabled { text_enabled } else { text_disabled };

    div()
        .id(id.into())
        .w(button_size)
        .h(button_size)
        .flex()
        .flex_shrink_0()
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
