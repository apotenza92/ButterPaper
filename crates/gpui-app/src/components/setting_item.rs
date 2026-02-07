//! Setting item component for settings panels.

use gpui::{div, prelude::*};

use crate::ui::{sizes, TypographyExt};
use crate::Theme;

/// Setting item with title, description, and control.
///
/// Layout: | Title + Description | Control |
///
/// # Example
/// ```ignore
/// setting_item(
///     "Dark Mode",
///     "Enable dark theme for the application",
///     toggle_switch(enabled, theme, on_change),
///     theme,
/// )
/// ```
pub fn setting_item(
    title: impl Into<gpui::SharedString>,
    description: impl Into<gpui::SharedString>,
    control: impl IntoElement,
    theme: &Theme,
) -> impl IntoElement {
    let text_muted = theme.text_muted;
    let border = theme.border;

    div()
        .flex()
        .flex_row()
        .w_full()
        .items_center()
        .gap(sizes::GAP_LG)
        .py(sizes::PADDING_XL)
        .border_b_1()
        .border_color(border)
        // Label column - takes remaining space
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(sizes::GAP_SM)
                .child(div().text_ui_body().child(title.into()))
                .child(div().text_ui_body().text_color(text_muted).child(description.into())),
        )
        // Control column - fixed width
        .child(div().flex_shrink_0().w(sizes::DROPDOWN_WIDTH).child(control))
}

/// Compact setting item without description.
pub fn setting_item_compact(
    title: impl Into<gpui::SharedString>,
    control: impl IntoElement,
    theme: &Theme,
) -> impl IntoElement {
    let border = theme.border;

    div()
        .flex()
        .flex_row()
        .w_full()
        .items_center()
        .justify_between()
        .gap(sizes::GAP_LG)
        .py(sizes::PADDING_MD)
        .border_b_1()
        .border_color(border)
        .child(div().text_ui_body().child(title.into()))
        .child(div().flex_shrink_0().child(control))
}
