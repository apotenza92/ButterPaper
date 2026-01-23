//! Text input styling component.
//!
//! Note: GPUI doesn't have a native text input element, so this provides
//! consistent styling for input-like containers. Actual input handling
//! requires GPUI's text input primitives or custom implementation.

use gpui::{div, prelude::*, px, SharedString};

use crate::ui::sizes;
use crate::Theme;

/// Input size variants
#[derive(Clone, Copy, Default)]
pub enum InputSize {
    /// Small: 24px height
    Sm,
    /// Medium: 32px height (default)
    #[default]
    Md,
    /// Large: 40px height
    Lg,
}

/// Create a styled text input container.
///
/// This provides the visual styling for an input field. For actual text input
/// functionality, you'll need to combine this with GPUI's input handling.
///
/// # Example
/// ```ignore
/// text_input("search-input", "Search...", InputSize::Md, theme)
/// ```
pub fn text_input(
    id: impl Into<SharedString>,
    placeholder: impl Into<SharedString>,
    size: InputSize,
    theme: &Theme,
) -> impl IntoElement {
    let placeholder = placeholder.into();
    let (height, px_val, text_size) = size_dimensions(size);

    div()
        .id(id.into())
        .h(height)
        .px(px_val)
        .flex()
        .items_center()
        .bg(theme.elevated_surface)
        .border_1()
        .border_color(theme.border)
        .rounded(sizes::RADIUS_SM)
        .text_size(text_size)
        .text_color(theme.text_muted)
        .child(placeholder)
}

/// Create a styled text input container with default size.
///
/// # Example
/// ```ignore
/// text_input_default("search", "Search...", theme)
/// ```
pub fn text_input_default(
    id: impl Into<SharedString>,
    placeholder: impl Into<SharedString>,
    theme: &Theme,
) -> impl IntoElement {
    text_input(id, placeholder, InputSize::Md, theme)
}

/// Get dimensions for an input size.
fn size_dimensions(size: InputSize) -> (gpui::Pixels, gpui::Pixels, gpui::Pixels) {
    match size {
        InputSize::Sm => (px(24.0), sizes::SPACE_2, px(12.0)),
        InputSize::Md => (px(32.0), sizes::SPACE_3, px(14.0)),
        InputSize::Lg => (px(40.0), sizes::SPACE_4, px(14.0)),
    }
}
