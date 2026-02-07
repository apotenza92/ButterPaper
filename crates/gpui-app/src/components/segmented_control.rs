//! Segmented control primitive for compact mutually-exclusive options.

use gpui::{div, prelude::*, ClickEvent, SharedString, Window};

use crate::components::button_like::subtle_border;
use crate::components::ButtonSize;
use crate::ui::{sizes, TypographyExt};
use crate::Theme;

#[derive(Clone, Debug)]
pub struct SegmentOption {
    pub value: String,
    pub label: String,
}

impl SegmentOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self { value: value.into(), label: label.into() }
    }

    pub fn simple(value: impl Into<String>) -> Self {
        let value = value.into();
        Self { value: value.clone(), label: value }
    }
}

pub fn segmented_control<F>(
    id: impl Into<SharedString>,
    options: Vec<SegmentOption>,
    selected: impl Into<String>,
    theme: &Theme,
    on_change: F,
) -> impl IntoElement
where
    F: Fn(&str, &mut gpui::App) + Clone + 'static,
{
    let id = id.into();
    let selected = selected.into();
    let border = subtle_border(theme);

    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .bg(theme.surface)
        .border_1()
        .border_color(border)
        .rounded(sizes::RADIUS_MD)
        .p(sizes::SPACE_1)
        .gap(sizes::SPACE_1)
        .children(options.into_iter().map(move |option| {
            let is_selected = option.value == selected;
            let value = option.value.clone();
            let on_change = on_change.clone();

            div()
                .id(SharedString::from(format!("segment-{}", option.value)))
                .h(ButtonSize::Medium.height_px())
                .px(sizes::PADDING_LG)
                .flex()
                .items_center()
                .justify_center()
                .rounded(sizes::RADIUS_SM)
                .text_ui_body()
                .cursor_pointer()
                .bg(if is_selected { theme.elevated_surface } else { theme.surface })
                .text_color(if is_selected { theme.text } else { theme.text_muted })
                .hover({
                    let hover = theme.element_hover;
                    move |s| s.bg(hover)
                })
                .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut gpui::App| {
                    on_change(&value, cx);
                })
                .child(option.label)
        }))
}
