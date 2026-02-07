//! Text button component for buttons with text labels.
//!
//! Provides a minimal text button with optional leading icon and keyboard shortcut display.
//! Ideal for welcome screens, menus, and contextual actions.

use gpui::{div, prelude::*, ClickEvent, SharedString, Window};

use crate::components::button_like::{
    variant_colors, ButtonLikeExt, ButtonLikeVariant, ButtonSize,
};
use crate::components::icon::{icon, Icon};
use crate::ui::sizes;
use crate::Theme;

fn base_text_button<F>(
    id: impl Into<SharedString>,
    size: ButtonSize,
    theme: &Theme,
    on_click: F,
) -> gpui::Stateful<gpui::Div>
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let (height, px_val, text_size, _icon_size) = dimensions(size);
    let colors = variant_colors(ButtonLikeVariant::Neutral, theme);

    div()
        .id(id.into())
        .h(height)
        .flex()
        .flex_row()
        .items_center()
        .gap(sizes::GAP_SM)
        .px(px_val)
        .button_like(colors, sizes::RADIUS_MD)
        .cursor_pointer()
        .text_size(text_size)
        .on_click(on_click)
}

/// Returns (height, padding_x, text_size, icon_size).
fn dimensions(size: ButtonSize) -> (gpui::Pixels, gpui::Pixels, gpui::Pixels, f32) {
    (size.height_px(), size.horizontal_padding_px(), size.text_size_px(), size.icon_size_px())
}

fn shortcut_text_size(size: ButtonSize) -> gpui::Pixels {
    size.shortcut_text_size_px()
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::{dimensions, shortcut_text_size};
    use crate::components::ButtonSize;

    #[test]
    fn text_button_dimensions_align_with_button_size() {
        let (h, px_val, text, icon) = dimensions(ButtonSize::Medium);
        let h: f32 = h.into();
        let px_val: f32 = px_val.into();
        let text: f32 = text.into();
        assert_eq!(h, 28.0);
        assert_eq!(px_val, 12.0);
        assert_eq!(text, 14.0);
        assert_eq!(icon, 18.0);
    }

    #[test]
    fn compact_shortcuts_use_smaller_text() {
        let compact: f32 = shortcut_text_size(ButtonSize::Compact).into();
        let large: f32 = shortcut_text_size(ButtonSize::Large).into();
        assert_eq!(compact, 10.0);
        assert_eq!(large, 12.0);
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
///     ButtonSize::Medium,
///     theme,
///     |_, _, _| println!("clicked"),
/// )
/// ```
pub fn text_button<F>(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    size: ButtonSize,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let label = label.into();
    base_text_button(id, size, theme, on_click).child(label)
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
///     ButtonSize::Medium,
///     theme,
///     |_, _, _| println!("clicked"),
/// )
/// ```
pub fn text_button_with_icon<F>(
    id: impl Into<SharedString>,
    icon_type: Icon,
    label: impl Into<SharedString>,
    size: ButtonSize,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let label = label.into();
    let (_, _, _, icon_size) = dimensions(size);
    let icon_color = theme.text_muted;
    base_text_button(id, size, theme, on_click)
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
///     ButtonSize::Medium,
///     theme,
///     |_, _, _| println!("clicked"),
/// )
/// ```
pub fn text_button_with_shortcut<F>(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    shortcut: impl Into<SharedString>,
    size: ButtonSize,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let label = label.into();
    let shortcut = shortcut.into();
    let shortcut_color = theme.text_muted;
    let shortcut_size = shortcut_text_size(size);
    base_text_button(id, size, theme, on_click).child(label).child(
        div().ml(sizes::GAP_LG).text_size(shortcut_size).text_color(shortcut_color).child(shortcut),
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
///     ButtonSize::Medium,
///     theme,
///     |_, _, _| println!("clicked"),
/// )
/// ```
pub fn text_button_full<F>(
    id: impl Into<SharedString>,
    icon_type: Icon,
    label: impl Into<SharedString>,
    shortcut: impl Into<SharedString>,
    size: ButtonSize,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let label = label.into();
    let shortcut = shortcut.into();
    let (_, _, _, icon_size) = dimensions(size);
    let icon_color = theme.text_muted;
    let shortcut_color = theme.text_muted;
    let shortcut_size = shortcut_text_size(size);
    base_text_button(id, size, theme, on_click)
        .child(icon(icon_type, icon_size, icon_color))
        .child(label)
        .child(
            div()
                .ml(sizes::GAP_LG)
                .text_size(shortcut_size)
                .text_color(shortcut_color)
                .child(shortcut),
        )
}
