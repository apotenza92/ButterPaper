//! Reusable UI components following Zed's design patterns.
//!
//! All components accept a `Theme` reference for consistent styling
//! and use constants from `ui::sizes` for consistent sizing.

#![allow(dead_code)]
#![allow(unused_imports)]

mod button;
mod card;
mod divider;
mod dropdown;
mod icon;
mod icon_button;
mod input;
mod nav_item;
mod setting_item;
pub mod tab_bar;
mod text_button;
mod toggle_switch;
mod tooltip;

pub use button::{button, button_default, button_primary, ButtonSize, ButtonVariant};
pub use card::{card, card_header, card_with_header};
pub use icon::{icon, Icon};
pub use icon_button::{icon_button, icon_button_conditional, IconButtonSize};
pub use input::{text_input, text_input_default, InputSize};
pub use text_button::{
    text_button, text_button_full, text_button_with_icon, text_button_with_shortcut,
    TextButtonSize,
};
pub use toggle_switch::{checkbox, checkbox_with_label, toggle_switch};
pub use tooltip::tooltip_builder;
