//! Toggle switch component for boolean settings.

use gpui::{div, prelude::*, px, ClickEvent, SharedString, Window};

use super::{icon, Icon};
use crate::ui::sizes;
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
    let surface = theme.surface;
    let border = theme.border;
    let text = theme.text;

    // Switch dimensions
    let switch_width = px(44.0);
    let switch_height = sizes::ICON_LG;
    let knob_size = px(18.0);
    let knob_offset_off = px(3.0);
    let knob_offset_on = switch_width - knob_size - px(3.0);

    div()
        .id(id.into())
        .w(switch_width)
        .h(switch_height)
        .flex()
        .items_center()
        .rounded(sizes::RADIUS_MD)
        .cursor_pointer()
        .border_1()
        .border_color(border)
        .when(enabled, move |d| d.bg(accent))
        .when(!enabled, move |d| d.bg(surface))
        .on_click(on_toggle)
        .child(
            div()
                .w(knob_size)
                .h(knob_size)
                .rounded(px(9.0))
                .bg(text)
                .when(enabled, move |d| d.ml(knob_offset_on))
                .when(!enabled, move |d| d.ml(knob_offset_off)),
        )
}

/// Simple checkbox toggle.
pub fn checkbox<F>(
    id: impl Into<SharedString>,
    checked: bool,
    theme: &Theme,
    on_toggle: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let accent = theme.accent;
    let surface = theme.surface;
    let border = theme.border;
    let text = theme.text;

    div()
        .id(id.into())
        .w(sizes::ICON_MD)
        .h(sizes::ICON_MD)
        .flex()
        .items_center()
        .justify_center()
        .rounded(sizes::RADIUS_SM)
        .cursor_pointer()
        .border_1()
        .border_color(border)
        .when(checked, move |d| d.bg(accent))
        .when(!checked, move |d| d.bg(surface))
        .on_click(on_toggle)
        .when(checked, move |d| {
            d.child(icon(Icon::Check, 14.0, text))
        })
}
