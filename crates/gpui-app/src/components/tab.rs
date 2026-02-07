//! Standardized tab item primitive.

use gpui::{div, prelude::*, px, ClickEvent, SharedString, Window};

use crate::components::tab_bar::TabId;
use crate::components::{icon, Icon};
use crate::ui::color;
use crate::ui::sizes;
use crate::Theme;

#[derive(Clone, Debug)]
pub struct TabItemData {
    pub id: TabId,
    pub title: SharedString,
    pub is_active: bool,
    pub is_dirty: bool,
}

impl TabItemData {
    pub fn new(id: TabId, title: impl Into<SharedString>, is_active: bool, is_dirty: bool) -> Self {
        Self { id, title: title.into(), is_active, is_dirty }
    }
}

pub fn tab_item<FSelect, FClose>(
    data: TabItemData,
    theme: &Theme,
    on_select: FSelect,
    on_close: FClose,
) -> impl IntoElement
where
    FSelect: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    FClose: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let subtle_border = color::with_alpha(theme.border, 0.35);
    let transparent = gpui::Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    let inactive_bg = transparent;
    let active_bg = theme.elevated_surface;
    let close_icon_color = if data.is_active { theme.text } else { theme.text_muted };

    div()
        .id(SharedString::from(format!("tab-{}", data.id)))
        .group("tab")
        .relative()
        .h_full()
        .min_w(sizes::TAB_MIN_WIDTH)
        .max_w(sizes::TAB_MAX_WIDTH)
        .px(sizes::SPACE_2)
        .flex()
        .flex_row()
        .items_center()
        .gap(sizes::SPACE_1)
        .cursor_pointer()
        .text_sm()
        .bg(if data.is_active { active_bg } else { inactive_bg })
        .border_r_1()
        .border_color(subtle_border)
        .text_color(if data.is_active { theme.text } else { theme.text_muted })
        .when(!data.is_active, {
            let hover_text = theme.text;
            let hover_bg = theme.element_hover;
            move |d| d.hover(move |s| s.text_color(hover_text).bg(hover_bg))
        })
        .on_click(on_select)
        .child(
            div()
                .min_w_0()
                .flex_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .child(data.title),
        )
        .when(data.is_dirty, {
            let text_muted = theme.text_muted;
            move |d| d.child(icon(Icon::Dirty, 10.0, text_muted))
        })
        .child(
            div()
                .when(!data.is_active, |d| d.opacity(0.).group_hover("tab", |s| s.opacity(1.0)))
                .child(
                    div()
                        .id(SharedString::from(format!("tab-close-{}", data.id)))
                        .w(sizes::TAB_CLOSE_SIZE)
                        .h(sizes::TAB_CLOSE_SIZE)
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_sm()
                        .hover(move |s| s.bg(theme.element_selected))
                        .on_click(on_close)
                        .child(icon(Icon::Close, 11.0, close_icon_color)),
                ),
        )
        .when(data.is_active, |d| {
            d.child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .h(sizes::TAB_ACTIVE_UNDERLINE_HEIGHT)
                    .bg(theme.accent),
            )
        })
}
