//! Text button component for buttons with text labels.
//!
//! Provides a minimal text button with optional leading icon and keyboard shortcut display.
//! Ideal for welcome screens, menus, and contextual actions.

use gpui::{div, prelude::*, px, ClickEvent, SharedString, Window};

use crate::components::icon::{icon, Icon};
use crate::ui::{sizes, StatefulInteractiveExt};
use crate::Theme;

/// Size variants for text buttons.
#[derive(Clone, Copy, Default)]
pub enum TextButtonSize {
    /// Small: text_xs, minimal padding
    Sm,
    /// Medium: text_sm, standard padding (default)
    #[default]
    Md,
    /// Large: text_base, generous padding
    Lg,
}

impl TextButtonSize {
    /// Returns (text_size_px, padding_y, padding_x, icon_size).
    fn dimensions(self) -> (gpui::Pixels, gpui::Pixels, gpui::Pixels, f32) {
        match self {
            TextButtonSize::Sm => (px(12.0), px(4.0), px(8.0), 12.0),
            TextButtonSize::Md => (px(14.0), px(8.0), px(12.0), 14.0),
            TextButtonSize::Lg => (px(16.0), px(10.0), px(16.0), 16.0),
        }
    }
}

/// Create a text button with the specified label, optional icon, and optional shortcut.
///
/// # Arguments
/// * `id` - Unique identifier for the button
/// * `label` - Text label to display
/// * `size` - Button size variant
/// * `theme` - Theme for styling
/// * `on_click` - Click handler
///
/// # Example
/// ```ignore
/// text_button(
///     "open-file-btn",
///     "Open File",
///     TextButtonSize::Md,
///     theme,
///     |_, _, _| println!("clicked"),
/// )
/// ```
pub fn text_button<F>(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    size: TextButtonSize,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let label = label.into();
    let (text_size, py, px_val, _icon_size) = size.dimensions();

    let text_color = theme.text;
    let hover_bg = theme.element_hover;
    let active_bg = theme.element_selected;

    div()
        .id(id.into())
        .flex()
        .flex_row()
        .items_center()
        .gap(sizes::GAP_SM)
        .py(py)
        .px(px_val)
        .rounded(sizes::RADIUS_SM)
        .cursor_pointer()
        .text_size(text_size)
        .text_color(text_color)
        .interactive_bg(hover_bg, active_bg)
        .on_click(on_click)
        .child(label)
}

/// Create a text button with a leading icon.
///
/// # Arguments
/// * `id` - Unique identifier for the button
/// * `icon_type` - Icon to display before the label
/// * `label` - Text label to display
/// * `size` - Button size variant
/// * `theme` - Theme for styling
/// * `on_click` - Click handler
///
/// # Example
/// ```ignore
/// text_button_with_icon(
///     "settings-btn",
///     Icon::Settings,
///     "Settings",
///     TextButtonSize::Md,
///     theme,
///     |_, _, _| println!("clicked"),
/// )
/// ```
pub fn text_button_with_icon<F>(
    id: impl Into<SharedString>,
    icon_type: Icon,
    label: impl Into<SharedString>,
    size: TextButtonSize,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let label = label.into();
    let (text_size, py, px_val, icon_size) = size.dimensions();

    let text_color = theme.text;
    let icon_color = theme.text_muted;
    let hover_bg = theme.element_hover;
    let active_bg = theme.element_selected;

    div()
        .id(id.into())
        .flex()
        .flex_row()
        .items_center()
        .gap(sizes::GAP_SM)
        .py(py)
        .px(px_val)
        .rounded(sizes::RADIUS_SM)
        .cursor_pointer()
        .text_size(text_size)
        .text_color(text_color)
        .interactive_bg(hover_bg, active_bg)
        .on_click(on_click)
        .child(icon(icon_type, icon_size, icon_color))
        .child(label)
}

/// Create a text button with a keyboard shortcut displayed.
///
/// The shortcut is displayed to the right of the label in muted text.
///
/// # Arguments
/// * `id` - Unique identifier for the button
/// * `label` - Text label to display
/// * `shortcut` - Keyboard shortcut text (e.g., "⌘O", "Ctrl+S")
/// * `size` - Button size variant
/// * `theme` - Theme for styling
/// * `on_click` - Click handler
///
/// # Example
/// ```ignore
/// text_button_with_shortcut(
///     "open-file-btn",
///     "Open File",
///     "⌘O",
///     TextButtonSize::Md,
///     theme,
///     |_, _, _| println!("clicked"),
/// )
/// ```
pub fn text_button_with_shortcut<F>(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    shortcut: impl Into<SharedString>,
    size: TextButtonSize,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let label = label.into();
    let shortcut = shortcut.into();
    let (text_size, py, px_val, _icon_size) = size.dimensions();

    let text_color = theme.text;
    let shortcut_color = theme.text_muted;
    let hover_bg = theme.element_hover;
    let active_bg = theme.element_selected;

    div()
        .id(id.into())
        .flex()
        .flex_row()
        .items_center()
        .gap(sizes::GAP_SM)
        .py(py)
        .px(px_val)
        .rounded(sizes::RADIUS_SM)
        .cursor_pointer()
        .text_size(text_size)
        .text_color(text_color)
        .interactive_bg(hover_bg, active_bg)
        .on_click(on_click)
        .child(label)
        .child(
            div()
                .ml(sizes::GAP_LG)
                .text_size(px(12.0))
                .text_color(shortcut_color)
                .child(shortcut),
        )
}

/// Create a text button with both a leading icon and keyboard shortcut.
///
/// # Arguments
/// * `id` - Unique identifier for the button
/// * `icon_type` - Icon to display before the label
/// * `label` - Text label to display
/// * `shortcut` - Keyboard shortcut text (e.g., "⌘O", "Ctrl+S")
/// * `size` - Button size variant
/// * `theme` - Theme for styling
/// * `on_click` - Click handler
///
/// # Example
/// ```ignore
/// text_button_full(
///     "open-file-btn",
///     Icon::Plus,
///     "Open File",
///     "⌘O",
///     TextButtonSize::Md,
///     theme,
///     |_, _, _| println!("clicked"),
/// )
/// ```
pub fn text_button_full<F>(
    id: impl Into<SharedString>,
    icon_type: Icon,
    label: impl Into<SharedString>,
    shortcut: impl Into<SharedString>,
    size: TextButtonSize,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let label = label.into();
    let shortcut = shortcut.into();
    let (text_size, py, px_val, icon_size) = size.dimensions();

    let text_color = theme.text;
    let icon_color = theme.text_muted;
    let shortcut_color = theme.text_muted;
    let hover_bg = theme.element_hover;
    let active_bg = theme.element_selected;

    div()
        .id(id.into())
        .flex()
        .flex_row()
        .items_center()
        .gap(sizes::GAP_SM)
        .py(py)
        .px(px_val)
        .rounded(sizes::RADIUS_SM)
        .cursor_pointer()
        .text_size(text_size)
        .text_color(text_color)
        .interactive_bg(hover_bg, active_bg)
        .on_click(on_click)
        .child(icon(icon_type, icon_size, icon_color))
        .child(label)
        .child(
            div()
                .ml(sizes::GAP_LG)
                .text_size(px(12.0))
                .text_color(shortcut_color)
                .child(shortcut),
        )
}
