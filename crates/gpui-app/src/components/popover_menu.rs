//! Lightweight anchored popover primitive.

use gpui::{div, prelude::*};

use crate::components::ButtonSize;
use crate::ui::sizes;

pub fn popover_menu(
    trigger: impl IntoElement,
    menu: impl IntoElement,
    is_open: bool,
) -> impl IntoElement {
    div().relative().child(trigger).when(is_open, |d| {
        d.child(
            div()
                .absolute()
                .left_0()
                .top(ButtonSize::Medium.height_px() + sizes::SPACE_1)
                .child(menu),
        )
    })
}
