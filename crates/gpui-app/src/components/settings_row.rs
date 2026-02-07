//! Standardized settings row primitive.

use gpui::{div, prelude::*, SharedString};

use crate::ui::{sizes, TypographyExt};
use crate::Theme;

pub fn settings_row(
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    control: impl IntoElement,
    theme: &Theme,
) -> impl IntoElement {
    let title = title.into();
    let description = description.into();

    div()
        .flex()
        .flex_row()
        .w_full()
        .items_center()
        .gap(sizes::GAP_LG)
        .py(sizes::PADDING_XL)
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(sizes::GAP_SM)
                .child(div().text_ui_body().child(title))
                .child(div().text_ui_body().text_color(theme.text_muted).child(description)),
        )
        .child(
            div()
                .flex()
                .justify_end()
                .items_center()
                .flex_shrink_0()
                .min_w(sizes::DROPDOWN_WIDTH)
                .child(control),
        )
}
