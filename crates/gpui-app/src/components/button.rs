//! Standard button component matching Zed's patterns.

use gpui::{div, prelude::*, ClickEvent, SharedString, Window};

use crate::ui::sizes;
use crate::Theme;

/// Standard button with consistent styling.
///
/// # Example
/// ```ignore
/// button("Save", theme, cx.listener(|this, _, _, cx| {
///     this.save(cx);
/// }))
/// ```
pub fn button<F>(label: impl Into<SharedString>, theme: &Theme, on_click: F) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let surface = theme.surface;
    let border = theme.border;
    let hover = theme.element_hover;

    div()
        .id(SharedString::from(format!("btn-{}", label.into())))
        .h(sizes::CONTROL_HEIGHT)
        .px(sizes::PADDING_LG)
        .flex()
        .items_center()
        .justify_center()
        .bg(surface)
        .border_1()
        .border_color(border)
        .rounded(sizes::RADIUS_SM)
        .cursor_pointer()
        .text_sm()
        .hover(move |s| s.bg(hover))
        .on_click(on_click)
}

/// Primary/accent button with highlighted styling.
pub fn button_primary<F>(
    label: impl Into<SharedString>,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let accent = theme.accent;
    let text = theme.text;

    div()
        .id(SharedString::from(format!("btn-primary-{}", label.into())))
        .h(sizes::CONTROL_HEIGHT)
        .px(sizes::PADDING_LG)
        .flex()
        .items_center()
        .justify_center()
        .bg(accent)
        .text_color(text)
        .rounded(sizes::RADIUS_SM)
        .cursor_pointer()
        .text_sm()
        .font_weight(gpui::FontWeight::MEDIUM)
        .on_click(on_click)
}
