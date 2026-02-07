//! Reusable UI components following Zed's design patterns.
//!
//! All components accept a `Theme` reference for consistent styling
//! and use constants from `ui::sizes` for consistent sizing.

#![allow(dead_code)]
#![allow(unused_imports)]

mod button;
mod button_like;
mod card;
mod chrome;
mod context_menu;
mod divider;
mod dropdown;
mod icon;
mod icon_button;
mod input;
mod nav_item;
mod popover_menu;
mod radio;
mod scrollbar;
mod segmented_control;
mod setting_item;
mod settings_group;
mod settings_row;
mod slider;
mod tab;
pub mod tab_bar;
mod text_button;
mod toggle_switch;
mod tooltip;

pub use button::{button, button_default, button_primary, ButtonVariant};
pub use button_like::ButtonSize;
pub use card::{card, card_header, card_with_header};
pub use chrome::{chrome_control_shell, chrome_control_size, chrome_icon_button};
pub use context_menu::{context_menu, ContextMenuItem};
pub use dropdown::{Dropdown, DropdownOption};
pub use icon::{icon, Icon};
pub use icon_button::{icon_button, icon_button_conditional};
pub use input::{text_input, text_input_default};
pub use nav_item::{nav_item, nav_item_with_icon};
pub use popover_menu::popover_menu;
pub use radio::{radio, radio_with_label};
pub use scrollbar::{scrollbar_gutter, ScrollbarController};
pub use segmented_control::{segmented_control, SegmentOption};
pub use settings_group::{settings_group, settings_panel};
pub use settings_row::settings_row;
pub use slider::slider;
pub use tab::{tab_item, TabItemData};
pub use text_button::{
    text_button, text_button_full, text_button_with_icon, text_button_with_shortcut,
};
pub use toggle_switch::{checkbox, checkbox_with_label, toggle_switch};
pub use tooltip::tooltip_builder;
