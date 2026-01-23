//! Navigation item component for sidebars.

use gpui::{div, prelude::*, ClickEvent, SharedString, Window};

use crate::ui::sizes;
use crate::Theme;

/// Navigation item for sidebar menus.
///
/// # Example
/// ```ignore
/// nav_item("Settings", true, theme, cx.listener(|this, _, _, cx| {
///     this.navigate_to_settings(cx);
/// }))
/// ```
pub fn nav_item<F>(
    label: impl Into<SharedString>,
    selected: bool,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let element_selected = theme.element_selected;
    let element_hover = theme.element_hover;

    div()
        .id(SharedString::from(format!("nav-{}", label.into())))
        .h(sizes::CONTROL_HEIGHT)
        .px(sizes::PADDING_MD)
        .flex()
        .items_center()
        .rounded(sizes::RADIUS_SM)
        .cursor_pointer()
        .text_sm()
        .when(selected, move |d| d.bg(element_selected))
        .when(!selected, move |d| d.hover(move |s| s.bg(element_hover)))
        .on_click(on_click)
}

/// Navigation item with icon.
pub fn nav_item_with_icon<F>(
    icon: impl Into<SharedString>,
    label: impl Into<SharedString>,
    selected: bool,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let label: SharedString = label.into();
    let element_selected = theme.element_selected;
    let element_hover = theme.element_hover;

    div()
        .id(SharedString::from(format!("nav-{}", label.clone())))
        .h(sizes::CONTROL_HEIGHT)
        .px(sizes::PADDING_MD)
        .flex()
        .items_center()
        .gap(sizes::GAP_MD)
        .rounded(sizes::RADIUS_SM)
        .cursor_pointer()
        .text_sm()
        .when(selected, move |d| d.bg(element_selected))
        .when(!selected, move |d| d.hover(move |s| s.bg(element_hover)))
        .on_click(on_click)
        .child(div().text_base().child(icon.into()))
        .child(label)
}
