//! Toggle switch component for boolean settings.

use gpui::{div, prelude::*, px, ClickEvent, SharedString, Window};

use super::{icon, Icon};
use crate::ui::color;
use crate::ui::{sizes, TypographyExt};
use crate::Theme;

#[derive(Clone, Copy)]
struct CheckboxColors {
    checked_bg: gpui::Rgba,
    checked_border: gpui::Rgba,
    checked_icon: gpui::Rgba,
    unchecked_bg: gpui::Rgba,
    unchecked_border: gpui::Rgba,
    unchecked_hover_bg: gpui::Rgba,
}

fn checkbox_colors(theme: &Theme) -> CheckboxColors {
    CheckboxColors {
        // Inverted checked state: dark fill + light tick in light themes,
        // and light fill + dark tick in dark themes.
        checked_bg: theme.text,
        checked_border: color::strong_border(theme.text),
        checked_icon: theme.background,
        // Neutral unchecked state.
        unchecked_bg: theme.elevated_surface,
        unchecked_border: color::strong_border(theme.text_muted),
        unchecked_hover_bg: theme.element_hover,
    }
}

/// Toggle switch for boolean values.
///
/// # Example
/// ```ignore
/// toggle_switch(
///     "dark-mode",
///     enabled,
///     theme,
///     cx.listener(|this, _, _, cx| {
///         this.toggle_dark_mode(cx);
///     }),
/// )
/// ```
pub fn toggle_switch<F>(
    id: impl Into<SharedString>,
    enabled: bool,
    theme: &Theme,
    on_toggle: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let accent = theme.accent;
    let surface = theme.elevated_surface;
    let text = theme.text;
    let subtle_border = color::subtle_border(theme.border);

    // Switch dimensions
    let switch_width = sizes::TOGGLE_WIDTH;
    let switch_height = sizes::CONTROL_HEIGHT_COMPACT;
    let knob_size = sizes::TOGGLE_KNOB_SIZE;
    let knob_offset_off = sizes::TOGGLE_KNOB_OFFSET;
    let knob_offset_on = switch_width - knob_size - sizes::TOGGLE_KNOB_OFFSET;

    div()
        .id(id.into())
        .w(switch_width)
        .h(switch_height)
        .flex()
        .items_center()
        .rounded(sizes::RADIUS_MD)
        .cursor_pointer()
        .border_1()
        .border_color(subtle_border)
        .when(enabled, move |d| d.bg(accent))
        .when(!enabled, move |d| d.bg(surface))
        .on_click(on_toggle)
        .child(
            div()
                .w(knob_size)
                .h(knob_size)
                .rounded_full()
                .bg(text)
                .when(enabled, move |d| d.ml(knob_offset_on))
                .when(!enabled, move |d| d.ml(knob_offset_off)),
        )
}

/// Checkbox size in pixels.
const CHECKBOX_SIZE: gpui::Pixels = sizes::CHECKBOX_SIZE;
/// Checkmark icon size
const CHECK_ICON_SIZE: f32 = 12.0;

/// Zed-style checkbox without label.
pub fn checkbox<F>(
    id: impl Into<SharedString>,
    checked: bool,
    theme: &Theme,
    on_toggle: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let colors = checkbox_colors(theme);

    div()
        .id(id.into())
        .w(CHECKBOX_SIZE)
        .h(CHECKBOX_SIZE)
        .flex()
        .items_center()
        .justify_center()
        .rounded(sizes::RADIUS_MD)
        .cursor_pointer()
        .border_1()
        .when(checked, move |d| {
            d.bg(colors.checked_bg).border_color(colors.checked_border).child(icon(
                Icon::Check,
                CHECK_ICON_SIZE,
                colors.checked_icon,
            ))
        })
        .when(!checked, move |d| {
            d.bg(colors.unchecked_bg)
                .border_color(colors.unchecked_border)
                .hover(move |s| s.bg(colors.unchecked_hover_bg))
        })
        .on_click(on_toggle)
}

/// Zed-style checkbox with label.
pub fn checkbox_with_label<F>(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    checked: bool,
    theme: &Theme,
    on_toggle: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let id = id.into();
    let label = label.into();
    let colors = checkbox_colors(theme);
    let text_color = theme.text;

    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .gap(sizes::GAP_SM)
        .cursor_pointer()
        .on_click(on_toggle)
        .child(
            div()
                .w(CHECKBOX_SIZE)
                .h(CHECKBOX_SIZE)
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(sizes::RADIUS_MD)
                .border_1()
                .when(checked, move |d| {
                    d.bg(colors.checked_bg).border_color(colors.checked_border).child(icon(
                        Icon::Check,
                        CHECK_ICON_SIZE,
                        colors.checked_icon,
                    ))
                })
                .when(!checked, move |d| {
                    d.bg(colors.unchecked_bg)
                        .border_color(colors.unchecked_border)
                        .hover(move |s| s.bg(colors.unchecked_hover_bg))
                }),
        )
        .child(div().text_ui_body().text_color(text_color).child(label))
}

#[cfg(test)]
mod tests {
    use super::checkbox_colors;
    use crate::theme::ThemeColors;
    use crate::ui::color;

    #[test]
    fn checkbox_checked_state_uses_neutral_contrast_palette() {
        let theme = ThemeColors::fallback_dark();
        let colors = checkbox_colors(&theme);
        assert_eq!(colors.checked_bg, theme.text);
        assert_eq!(colors.checked_icon, theme.background);
        assert_eq!(colors.checked_border, color::strong_border(theme.text));
    }

    #[test]
    fn checkbox_unchecked_state_uses_neutral_palette() {
        let theme = ThemeColors::fallback_light();
        let colors = checkbox_colors(&theme);
        assert_eq!(colors.unchecked_bg, theme.elevated_surface);
        assert_eq!(colors.unchecked_border, color::strong_border(theme.text_muted));
        assert_eq!(colors.unchecked_hover_bg, theme.element_hover);
    }
}
