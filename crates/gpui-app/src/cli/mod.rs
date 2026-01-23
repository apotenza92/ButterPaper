//! CLI module for command-line argument parsing and automation.
//!
//! This module provides:
//! - CLI argument parsing (`CliArgs`, `parse_args`)
//! - Mouse automation (`MouseAction`, `simulate_mouse`)
//! - UI element automation (`list_elements`, `click_element`)

mod args;
mod automation;

#[allow(unused_imports)]
pub use args::{parse_args, CliArgs, MouseAction};
#[allow(unused_imports)]
pub use automation::{click_element, list_elements, simulate_mouse};
