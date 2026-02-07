//! Toggle switch component for boolean settings.

use gpui::{div, prelude::*, px, ClickEvent, SharedString, Window};

use super::{icon, Icon};
use crate::ui::color;
use crate::ui::{sizes, TypographyExt};
use crate::Theme;

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
    let surface = theme.elevated_surface;
    let hover_bg = theme.element_hover;
    let selected_bg = theme.element_selected;
    let check_color = theme.text;
    let subtle_border = color::subtle_border(theme.border);

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
            d.bg(selected_bg).border_color(subtle_border).child(icon(
                Icon::Check,
                CHECK_ICON_SIZE,
                check_color,
            ))
        })
        .when(!checked, move |d| {
            d.bg(surface).border_color(subtle_border).hover(move |s| s.bg(hover_bg))
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
    let surface = theme.elevated_surface;
    let hover_bg = theme.element_hover;
    let selected_bg = theme.element_selected;
    let text_color = theme.text;
    let check_color = theme.text;
    let subtle_border = color::subtle_border(theme.border);

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
                    d.bg(selected_bg).border_color(subtle_border).child(icon(
                        Icon::Check,
                        CHECK_ICON_SIZE,
                        check_color,
                    ))
                })
                .when(!checked, move |d| {
                    d.bg(surface).border_color(subtle_border).hover(move |s| s.bg(hover_bg))
                }),
        )
        .child(div().text_ui_body().text_color(text_color).child(label))
}
