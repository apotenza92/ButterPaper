//! Dropdown component for selecting from a list of options.

#![allow(clippy::type_complexity)]

use gpui::{deferred, div, prelude::*, px, SharedString};

use super::{icon, Icon};
use crate::ui::sizes;
use crate::Theme;

/// A single option in a dropdown menu.
#[derive(Clone, Debug)]
pub struct DropdownOption {
    pub value: String,
    pub label: String,
}

impl DropdownOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }

    /// Create option where value equals label.
    pub fn simple(label: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            value: label.clone(),
            label,
        }
    }
}

/// Dropdown select component.
///
/// # Example
/// ```ignore
/// Dropdown::new("theme-dropdown")
///     .options(vec![
///         DropdownOption::simple("Light"),
///         DropdownOption::simple("Dark"),
///     ])
///     .selected("Dark")
///     .on_select(|value, cx| {
///         // handle selection
///     })
///     .render(is_open, theme, cx)
/// ```
pub struct Dropdown<F>
where
    F: Fn(&str, &mut gpui::App) + Clone + 'static,
{
    id: SharedString,
    options: Vec<DropdownOption>,
    selected: String,
    on_select: F,
    on_toggle: Option<Box<dyn Fn(&mut gpui::App) + 'static>>,
}

impl<F> Dropdown<F>
where
    F: Fn(&str, &mut gpui::App) + Clone + 'static,
{
    pub fn new(id: impl Into<SharedString>, on_select: F) -> Self {
        Self {
            id: id.into(),
            options: Vec::new(),
            selected: String::new(),
            on_select,
            on_toggle: None,
        }
    }

    pub fn options(mut self, options: Vec<DropdownOption>) -> Self {
        self.options = options;
        self
    }

    pub fn selected(mut self, value: impl Into<String>) -> Self {
        self.selected = value.into();
        self
    }

    pub fn on_toggle<G>(mut self, on_toggle: G) -> Self
    where
        G: Fn(&mut gpui::App) + 'static,
    {
        self.on_toggle = Some(Box::new(on_toggle));
        self
    }

    pub fn render(self, is_open: bool, theme: &Theme) -> impl IntoElement {
        let surface = theme.surface;
        let border = theme.border;
        let hover = theme.element_hover;
        let text_muted = theme.text_muted;
        let accent = theme.accent;
        let max_height = px(240.0);

        let selected = self.selected.clone();
        let current_label = self
            .options
            .iter()
            .find(|o| o.value == self.selected)
            .map(|o| o.label.clone())
            .unwrap_or_else(|| self.selected.clone());

        let id = self.id.clone();
        let on_toggle = self.on_toggle;

        div()
            .relative()
            .w_full()
            .h(sizes::CONTROL_HEIGHT)
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .pl(sizes::PADDING_LG)
            .pr(px(10.0))
            .bg(surface)
            .border_1()
            .border_color(border)
            .rounded(sizes::RADIUS_SM)
            .cursor_pointer()
            .hover(move |s| s.bg(hover))
            .id(id.clone())
            .on_click(move |_, _, cx| {
                if let Some(ref toggle) = on_toggle {
                    toggle(cx);
                }
            })
            .child(
                div()
                    .text_sm()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(current_label),
            )
            .child(
                div()
                    .ml(sizes::GAP_SM)
                    .child(icon(Icon::ChevronDown, 10.0, text_muted)),
            )
            .when(is_open, |d| {
                let options = self.options.clone();
                let on_select = self.on_select.clone();
                let selected = selected.clone();

                d.child(
                    div()
                        .absolute()
                        .left_0()
                        .top(sizes::CONTROL_HEIGHT + px(4.0))
                        .child(
                            deferred(
                                div()
                                    .occlude()
                                    .min_w(sizes::DROPDOWN_WIDTH)
                                    .max_h(max_height)
                                    .overflow_hidden()
                                    .bg(surface)
                                    .border_1()
                                    .border_color(border)
                                    .rounded(sizes::RADIUS_MD)
                                    .shadow_lg()
                                    .py(sizes::PADDING_SM)
                                    .children(options.iter().map(|opt| {
                                        let is_selected = opt.value == selected;
                                        let on_select = on_select.clone();
                                        let value = opt.value.clone();

                                        div()
                                            .id(SharedString::from(format!("opt-{}", opt.value)))
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .justify_between()
                                            .h(sizes::CONTROL_HEIGHT)
                                            .px(sizes::PADDING_LG)
                                            .mx(sizes::PADDING_SM)
                                            .rounded(sizes::RADIUS_SM)
                                            .cursor_pointer()
                                            .text_sm()
                                            .hover(move |s| s.bg(hover))
                                            .on_click(move |_, _, cx| {
                                                on_select(&value, cx);
                                            })
                                            .child(opt.label.clone())
                                            .when(is_selected, |d| {
                                                d.child(icon(Icon::Check, 14.0, accent))
                                            })
                                    })),
                            )
                            .with_priority(1),
                        ),
                )
            })
    }
}
