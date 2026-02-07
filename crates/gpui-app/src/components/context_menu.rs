//! Lightweight in-app context menu primitive.

use gpui::{div, prelude::*, ClickEvent, SharedString, Window};

use crate::components::ButtonSize;
use crate::ui::sizes;
use crate::Theme;

#[derive(Clone, Debug)]
pub struct ContextMenuItem {
    pub value: String,
    pub label: String,
    pub enabled: bool,
    pub checked: bool,
}

impl ContextMenuItem {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self { value: value.into(), label: label.into(), enabled: true, checked: false }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.enabled = !disabled;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
}

pub fn context_menu<F>(
    id: impl Into<SharedString>,
    items: Vec<ContextMenuItem>,
    theme: &Theme,
    on_select: F,
) -> impl IntoElement
where
    F: Fn(&str, &mut Window, &mut gpui::App) + Clone + 'static,
{
    let id = id.into();

    div()
        .id(id)
        .min_w(sizes::MENU_WIDTH_MIN)
        .max_w(sizes::MENU_WIDTH_MAX)
        .bg(theme.elevated_surface)
        .border_1()
        .border_color(theme.border)
        .rounded(sizes::RADIUS_MD)
        .shadow_lg()
        .py(sizes::SPACE_1)
        .children(items.into_iter().map(move |item| {
            let enabled = item.enabled;
            let value = item.value.clone();
            let on_select = on_select.clone();

            div()
                .id(SharedString::from(format!("ctx-item-{}", item.value)))
                .h(ButtonSize::Medium.height_px())
                .px(sizes::SPACE_3)
                .mx(sizes::SPACE_1)
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .rounded(sizes::RADIUS_SM)
                .text_sm()
                .text_color(if enabled { theme.text } else { theme.text_muted })
                .when(enabled, |d| {
                    d.cursor_pointer().hover({
                        let hover = theme.element_hover;
                        move |s| s.bg(hover)
                    })
                })
                .on_click(move |_: &ClickEvent, window: &mut Window, cx: &mut gpui::App| {
                    if enabled {
                        on_select(&value, window, cx);
                    }
                })
                .child(item.label)
                .when(item.checked, |d| d.child(div().text_sm().child("✓")))
        }))
}
