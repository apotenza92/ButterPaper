//! Tab bar component for multi-document interfaces.

use gpui::{div, prelude::*, px, SharedString};
use std::path::PathBuf;
use uuid::Uuid;

use super::tooltip_builder;
use crate::ui::sizes;
use crate::Theme;

/// Unique identifier for a tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TabId(Uuid);

impl TabId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TabId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TabId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A single tab in the tab bar.
#[derive(Clone, Debug)]
pub struct Tab {
    pub id: TabId,
    pub path: PathBuf,
    pub title: String,
    pub is_dirty: bool,
}

impl Tab {
    pub fn new(path: PathBuf) -> Self {
        let title = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string());

        Self {
            id: TabId::new(),
            path,
            title,
            is_dirty: false,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
}

/// Tab bar height constant.
pub const TAB_BAR_HEIGHT: gpui::Pixels = px(32.0);

/// Tab bar component that renders a list of tabs.
///
/// # Example
/// ```ignore
/// TabBar::new(tabs, active_tab_id)
///     .on_select(|tab_id, cx| { ... })
///     .on_close(|tab_id, cx| { ... })
///     .render(theme)
/// ```
pub struct TabBar<SelectFn, CloseFn>
where
    SelectFn: Fn(TabId, &mut gpui::App) + Clone + 'static,
    CloseFn: Fn(TabId, &mut gpui::App) + Clone + 'static,
{
    tabs: Vec<Tab>,
    active_tab_id: Option<TabId>,
    on_select: SelectFn,
    on_close: CloseFn,
}

impl<SelectFn, CloseFn> TabBar<SelectFn, CloseFn>
where
    SelectFn: Fn(TabId, &mut gpui::App) + Clone + 'static,
    CloseFn: Fn(TabId, &mut gpui::App) + Clone + 'static,
{
    pub fn new(
        tabs: Vec<Tab>,
        active_tab_id: Option<TabId>,
        on_select: SelectFn,
        on_close: CloseFn,
    ) -> Self {
        Self {
            tabs,
            active_tab_id,
            on_select,
            on_close,
        }
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let surface = theme.surface;
        let border = theme.border;
        let element_selected = theme.element_selected;
        let element_hover = theme.element_hover;
        let text_muted = theme.text_muted;

        div()
            .id("tab-bar")
            .h(TAB_BAR_HEIGHT)
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .bg(surface)
            .border_b_1()
            .border_color(border)
            .px(sizes::PADDING_SM)
            .gap(px(2.0))
            .children(self.tabs.iter().map(|tab| {
                let is_active = self.active_tab_id == Some(tab.id);
                let tab_id = tab.id;
                let on_select = self.on_select.clone();
                let on_close = self.on_close.clone();

                div()
                    .id(SharedString::from(format!("tab-{}", tab.id)))
                    .h(px(26.0))
                    .px(sizes::PADDING_MD)
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(sizes::GAP_SM)
                    .rounded(sizes::RADIUS_SM)
                    .cursor_pointer()
                    .text_sm()
                    .when(is_active, move |d| d.bg(element_selected))
                    .when(!is_active, move |d| d.hover(move |s| s.bg(element_hover)))
                    .on_click(move |_, _, cx| {
                        on_select(tab_id, cx);
                    })
                    // Tab title with tooltip showing full name
                    .child({
                        let title = tab.title.clone();
                        div()
                            .max_w(px(150.0))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(title)
                    })
                    .tooltip(tooltip_builder(tab.title.clone(), surface, border))
                    // Dirty indicator
                    .when(tab.is_dirty, |d| {
                        d.child(div().text_xs().text_color(text_muted).child("\u{2022}"))
                    })
                    // Close button
                    .child(
                        div()
                            .id(SharedString::from(format!("tab-close-{}", tab.id)))
                            .w(px(16.0))
                            .h(px(16.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(sizes::RADIUS_SM)
                            .text_xs()
                            .text_color(text_muted)
                            .hover(move |s| s.bg(element_hover))
                            .on_click(move |_, _, cx| {
                                on_close(tab_id, cx);
                            })
                            .child("\u{2715}"), // X symbol
                    )
            }))
    }
}
