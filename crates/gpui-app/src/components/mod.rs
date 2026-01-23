//! Reusable UI components following Zed's design patterns.
//!
//! All components accept a `Theme` reference for consistent styling
//! and use constants from `ui::sizes` for consistent sizing.

#![allow(dead_code)]

mod button;
mod divider;
mod dropdown;
mod nav_item;
mod setting_item;
pub mod tab_bar;
mod toggle_switch;
mod tooltip;

pub use toggle_switch::toggle_switch;
pub use tooltip::tooltip_builder;
