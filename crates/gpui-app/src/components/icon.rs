//! Standardized icon component for consistent icon rendering.
//!
//! Uses semantic icon names that map to embedded SVG assets.

use gpui::{prelude::*, px, svg, IntoElement, Rgba};

use crate::icons::IconName;

pub type Icon = IconName;

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
    svg().w(px(size)).h(px(size)).flex_none().path(icon.path()).text_color(color)
}
