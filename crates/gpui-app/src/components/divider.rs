//! Horizontal divider component.

use gpui::{div, prelude::*};

use crate::ui::sizes;
use crate::Theme;

/// Horizontal divider line with optional vertical padding.
///
/// # Example
/// ```ignore
/// div()
///     .child(content_above)
///     .child(divider(theme))
///     .child(content_below)
/// ```
pub fn divider(theme: &Theme) -> impl IntoElement {
    let border = theme.border;

    div().w_full().h_0().my(sizes::PADDING_MD).border_t_1().border_color(border)
}

/// Divider with no vertical margin (just the line).
pub fn divider_tight(theme: &Theme) -> impl IntoElement {
    let border = theme.border;

    div().w_full().h_0().border_t_1().border_color(border)
}
