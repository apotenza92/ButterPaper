//! Text input styling component.
//!
//! Note: GPUI doesn't have a native text input element, so this provides
//! consistent styling for input-like containers. Actual input handling
//! requires GPUI's text input primitives or custom implementation.

use gpui::{div, prelude::*, SharedString};

use crate::components::button_like::ButtonSize;
use crate::ui::color;
use crate::ui::sizes;
use crate::Theme;

/// Create a styled text input container.
///
/// This provides the visual styling for an input field. For actual text input
/// functionality, you'll need to combine this with GPUI's input handling.
///
/// # Example
/// ```ignore
/// text_input("search-input", "Search...", ButtonSize::Medium, theme)
/// ```
pub fn text_input(
    id: impl Into<SharedString>,
    placeholder: impl Into<SharedString>,
    size: ButtonSize,
    theme: &Theme,
) -> impl IntoElement {
    let placeholder = placeholder.into();
    let (height, px_val, text_size) = size_dimensions(size);
    let subtle_border = color::subtle_border(theme.border);

    div()
        .id(id.into())
        .h(height)
        .px(px_val)
        .flex()
        .items_center()
        .bg(theme.elevated_surface)
        .border_1()
        .border_color(subtle_border)
        .rounded(sizes::RADIUS_MD)
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
    text_input(id, placeholder, ButtonSize::Medium, theme)
}

/// Get dimensions for an input size.
fn size_dimensions(size: ButtonSize) -> (gpui::Pixels, gpui::Pixels, gpui::Pixels) {
    (size.height_px(), size.horizontal_padding_px(), size.text_size_px())
}

#[cfg(test)]
mod tests {
    use super::size_dimensions;
    use crate::components::ButtonSize;

    #[test]
    fn input_dimensions_match_shared_button_sizes() {
        let (h, px_val, text) = size_dimensions(ButtonSize::Default);
        let h: f32 = h.into();
        let px_val: f32 = px_val.into();
        let text: f32 = text.into();

        assert_eq!(h, 22.0);
        assert_eq!(px_val, 8.0);
        assert_eq!(text, 14.0);
    }
}
