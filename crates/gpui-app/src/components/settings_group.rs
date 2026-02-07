//! Settings section/group primitives.

use gpui::{div, prelude::*, AnyElement, SharedString};

use crate::ui::sizes;
use crate::Theme;

pub fn settings_group(
    title: impl Into<Option<SharedString>>,
    children: Vec<AnyElement>,
    theme: &Theme,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .w_full()
        .when_some(title.into(), |d, title| {
            d.child(div().text_sm().text_color(theme.text_muted).mb(sizes::SPACE_2).child(title))
        })
        .children(children)
}

pub fn settings_panel(children: Vec<AnyElement>, theme: &Theme) -> impl IntoElement {
    div()
        .w_full()
        .bg(theme.elevated_surface)
        .border_1()
        .border_color(theme.border)
        .rounded(sizes::RADIUS_LG)
        .p(sizes::SPACE_3)
        .children(children)
}
