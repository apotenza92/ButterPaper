//! Standardized tab item primitive.

use gpui::{div, prelude::*, px, ClickEvent, SharedString, Window};

use crate::components::tab_bar::TabId;
use crate::components::{icon, tooltip_builder, Icon};
use crate::ui::color;
use crate::ui::{sizes, TypographyExt};
use crate::Theme;

#[derive(Clone, Debug)]
pub struct TabItemData {
    pub id: TabId,
    pub title: SharedString,
    pub is_active: bool,
    pub is_dirty: bool,
    pub is_closable: bool,
}

impl TabItemData {
    pub fn new(
        id: TabId,
        title: impl Into<SharedString>,
        is_active: bool,
        is_dirty: bool,
        is_closable: bool,
    ) -> Self {
        Self { id, title: title.into(), is_active, is_dirty, is_closable }
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
    let subtle_border = color::subtle_border(theme.border);
    let inactive_bg = theme.elevated_surface;
    let hover_bg = theme.element_hover;
    let active_bg = color::with_alpha(theme.element_selected, 0.88);
    let pressed_bg = theme.element_selected;
    let close_hover_bg = theme.element_hover;
    let close_pressed_bg = theme.element_selected;
    let close_icon_color = if data.is_active { theme.text } else { theme.text_muted };

    div()
        .id(SharedString::from(format!("tab-{}", data.id)))
        .group("tab")
        .relative()
        .h(sizes::TOOLBAR_CONTROL_SIZE)
        .mr(sizes::SPACE_1)
        .flex_shrink_0()
        .min_w(sizes::TAB_MIN_WIDTH)
        .pl(sizes::SPACE_2)
        .pr(sizes::SPACE_1)
        .flex()
        .flex_row()
        .items_center()
        .rounded(sizes::RADIUS_SM)
        .cursor_pointer()
        .text_ui_body()
        .bg(if data.is_active { active_bg } else { inactive_bg })
        .border_1()
        .border_color(subtle_border)
        .text_color(if data.is_active { theme.text } else { theme.text_muted })
        .active(move |s| s.bg(pressed_bg))
        .when(!data.is_active, {
            let hover_text = theme.text;
            move |d| d.hover(move |s| s.text_color(hover_text).bg(hover_bg))
        })
        .on_click(on_select)
        .child(
            div()
                .h_full()
                .flex()
                .items_center()
                .flex_1()
                .min_w_0()
                .pr(sizes::SPACE_1)
                .whitespace_nowrap()
                .child(data.title),
        )
        .when(data.is_dirty, {
            let text_muted = theme.text_muted;
            move |d| d.child(div().mr(sizes::SPACE_1).child(icon(Icon::Dirty, 10.0, text_muted)))
        })
        .when(data.is_closable, |d| {
            d.child(
                div()
                    .ml_auto()
                    .ml(sizes::SPACE_1)
                    .when(!data.is_active, |d| d.opacity(0.).group_hover("tab", |s| s.opacity(1.0)))
                    .child(
                        div()
                            .id(SharedString::from(format!("tab-close-{}", data.id)))
                            .w(px(18.0))
                            .h(px(18.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(4.0))
                            .hover({
                                let hover_text = theme.text;
                                move |s| s.bg(close_hover_bg).text_color(hover_text)
                            })
                            .active({
                                let pressed_text = theme.text;
                                move |s| s.bg(close_pressed_bg).text_color(pressed_text)
                            })
                            .text_color(close_icon_color)
                            .tooltip(tooltip_builder(
                                "Close tab",
                                theme.surface,
                                theme.border,
                                theme.text,
                            ))
                            .on_click(on_close)
                            .child(icon(Icon::Close, 13.0, close_icon_color)),
                    ),
            )
        })
}
