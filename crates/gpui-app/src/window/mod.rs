//! Window management module for listing, focusing, and capturing windows.
//!
//! This module provides cross-platform window operations:
//! - `list_windows`: List all capturable windows
//! - `focus_window`: Bring a window to the front
//! - `schedule_screenshot`, `capture_window`: Screenshot capture

mod manager;
mod screenshot;

#[allow(unused_imports)]
pub use manager::{focus_window, list_windows};
#[allow(unused_imports)]
pub use screenshot::{capture_window, schedule_screenshot};
