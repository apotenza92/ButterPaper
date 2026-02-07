//! Radio input primitives.

use gpui::{div, prelude::*, px, ClickEvent, SharedString, Window};

use crate::ui::color;
use crate::ui::{sizes, TypographyExt};
use crate::Theme;

const RADIO_SIZE: gpui::Pixels = sizes::RADIO_SIZE;

pub fn radio<F>(
    id: impl Into<SharedString>,
    selected: bool,
    theme: &Theme,
    on_select: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let border = color::subtle_border(theme.border);
    let dot = theme.text;

    div()
        .id(id.into())
        .w(RADIO_SIZE)
        .h(RADIO_SIZE)
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(theme.elevated_surface)
        .border_1()
        .border_color(border)
        .cursor_pointer()
        .hover({
            let hover = theme.element_hover;
            move |s| s.bg(hover)
        })
        .on_click(on_select)
        .when(selected, move |d| {
            d.child(div().w(sizes::RADIO_DOT_SIZE).h(sizes::RADIO_DOT_SIZE).rounded_full().bg(dot))
        })
}

pub fn radio_with_label<F>(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    selected: bool,
    theme: &Theme,
    on_select: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let id = id.into();
    let label = label.into();
    let text = theme.text;

    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .gap(sizes::GAP_SM)
        .cursor_pointer()
        .on_click(on_select)
        .child(radio("radio-inner", selected, theme, |_, _, _| {}))
        .child(div().text_ui_body().text_color(text).child(label))
}
