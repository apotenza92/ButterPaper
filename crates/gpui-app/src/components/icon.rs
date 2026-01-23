//! Standardized icon component for consistent icon rendering.
//!
//! Provides an Icon enum with common icons used throughout the app,
//! replacing inline unicode characters with a reusable abstraction.

use gpui::{div, prelude::*, px, IntoElement, Rgba};

/// Common icons used throughout the application.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    /// Close button (✕)
    Close,
    /// Left chevron (‹)
    ChevronLeft,
    /// Right chevron (›)
    ChevronRight,
    /// Left arrow (←)
    ArrowLeft,
    /// Right arrow (→)
    ArrowRight,
    /// Dirty/unsaved indicator (•)
    Dirty,
    /// Settings gear (⚙)
    Settings,
    /// Checkmark (✓)
    Check,
    /// Down arrow for dropdowns (▼)
    ChevronDown,
    /// Plus sign for add/new actions (+)
    Plus,
}

impl Icon {
    /// Returns the unicode character for this icon.
    pub fn as_str(&self) -> &'static str {
        match self {
            Icon::Close => "\u{2715}",
            Icon::ChevronLeft => "\u{2039}",
            Icon::ChevronRight => "\u{203A}",
            Icon::ArrowLeft => "\u{2190}",
            Icon::ArrowRight => "\u{2192}",
            Icon::Dirty => "\u{2022}",
            Icon::Settings => "\u{2699}",
            Icon::Check => "\u{2713}",
            Icon::ChevronDown => "\u{25BC}",
            Icon::Plus => "+",
        }
    }
}

/// Creates an icon element with the specified size and color.
///
/// # Arguments
/// * `icon` - The icon to render
/// * `size` - Font size in pixels
/// * `color` - Text color for the icon
///
/// # Example
/// ```ignore
/// icon(Icon::Close, 14.0, theme.text_muted)
/// ```
pub fn icon(icon: Icon, size: f32, color: Rgba) -> impl IntoElement {
    div()
        .text_size(px(size))
        .text_color(color)
        .child(icon.as_str())
}
