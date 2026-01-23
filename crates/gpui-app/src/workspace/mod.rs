//! Workspace management for multi-document, multi-window PDF editing.
//!
//! This module provides the data structures and logic for managing:
//! - Multiple tabs within a window
//! - Multiple windows in the application
//! - User preferences for tab/window behavior
//! - State persistence (save/restore layouts)

mod persistence;
mod types;
mod window_manager;

pub use persistence::{load_preferences, save_preferences};
pub use types::TabPreferences;
