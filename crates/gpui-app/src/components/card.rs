//! Card component for consistent panel/container styling.

use gpui::{div, prelude::*, AnyElement};

use crate::ui::sizes;
use crate::Theme;

/// Create a card container with standard padding, border, and shadow.
///
/// # Example
/// ```ignore
/// card(theme, vec![
///     div().child("Card content").into_any_element()
/// ])
/// ```
pub fn card(theme: &Theme, children: Vec<AnyElement>) -> impl IntoElement {
    div()
        .p(sizes::SPACE_4)
        .bg(theme.surface)
        .border_1()
        .border_color(theme.border)
        .rounded(sizes::RADIUS_MD)
        .shadow_sm()
        .children(children)
}

/// Create a card header with title and bottom border.
///
/// # Example
/// ```ignore
/// card_header(theme, "Settings")
/// ```
pub fn card_header(theme: &Theme, title: impl Into<String>) -> impl IntoElement {
    div()
        .pb(sizes::SPACE_3)
        .mb(sizes::SPACE_3)
        .border_b_1()
        .border_color(theme.border)
        .text_size(gpui::px(16.0))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(theme.text)
        .child(title.into())
}

/// Create a card with a header.
///
/// Convenience wrapper for common card-with-header pattern.
///
/// # Example
/// ```ignore
/// card_with_header(theme, "Appearance", vec![
///     div().child("Content").into_any_element()
/// ])
/// ```
pub fn card_with_header(
    theme: &Theme,
    title: impl Into<String>,
    children: Vec<AnyElement>,
) -> impl IntoElement {
    let mut all_children = vec![card_header(theme, title).into_any_element()];
    all_children.extend(children);
    card(theme, all_children)
}
