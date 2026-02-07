//! Lightweight anchored popover primitive.

use gpui::{deferred, div, prelude::*, Pixels};

use crate::components::ButtonSize;
use crate::ui::sizes;

pub fn popover_menu(
    trigger: impl IntoElement,
    menu: impl IntoElement,
    is_open: bool,
    y_offset: Pixels,
) -> impl IntoElement {
    div().relative().child(trigger).when(is_open, |d| {
        d.child(
            div()
                .absolute()
                .left_0()
                .top(ButtonSize::Medium.height_px() + sizes::SPACE_1 + y_offset)
                .child(deferred(div().occlude().child(menu)).with_priority(1)),
        )
    })
}
