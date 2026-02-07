//! Standard button component with variants and sizes.

use gpui::{div, prelude::*, ClickEvent, SharedString, Window};

use crate::components::button_like::{
    variant_colors, ButtonLikeExt, ButtonLikeVariant, ButtonSize,
};
use crate::ui::sizes;
use crate::Theme;

/// Button visual variant
#[derive(Clone, Copy, Default)]
pub enum ButtonVariant {
    /// Default button with subtle background
    #[default]
    Default,
    /// Primary/accent button for main actions
    Primary,
    /// Ghost button with transparent background
    Ghost,
    /// Danger button for destructive actions
    Danger,
}

/// Create a button with the specified variant and size.
///
/// # Example
/// ```ignore
/// button(
///     "save-btn",
///     "Save",
///     ButtonVariant::Primary,
///     ButtonSize::Medium,
///     theme,
///     cx.listener(|this, _, _, cx| this.save(cx)),
/// )
/// ```
pub fn button<F>(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
    size: ButtonSize,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let label = label.into();
    let colors = variant_colors(variant.to_button_like_variant(), theme);
    let (height, px_val, text_size) = size_dimensions(size);

    div()
        .id(id.into())
        .h(height)
        .px(px_val)
        .flex()
        .items_center()
        .justify_center()
        .button_like(colors, sizes::RADIUS_MD)
        .text_size(text_size)
        .cursor_pointer()
        .on_click(on_click)
        .child(label)
}

/// Simplified button with default variant and size.
///
/// # Example
/// ```ignore
/// button_default("Save", theme, |_, _, _| {})
/// ```
pub fn button_default<F>(
    label: impl Into<SharedString>,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let label_str: SharedString = label.into();
    let id = format!("btn-{}", label_str);
    button(id, label_str, ButtonVariant::Default, ButtonSize::Medium, theme, on_click)
}

/// Simplified primary button with default size.
///
/// # Example
/// ```ignore
/// button_primary("Submit", theme, |_, _, _| {})
/// ```
pub fn button_primary<F>(
    label: impl Into<SharedString>,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let label_str: SharedString = label.into();
    let id = format!("btn-primary-{}", label_str);
    button(id, label_str, ButtonVariant::Primary, ButtonSize::Medium, theme, on_click)
}

impl ButtonVariant {
    fn to_button_like_variant(self) -> ButtonLikeVariant {
        match self {
            ButtonVariant::Default => ButtonLikeVariant::Neutral,
            ButtonVariant::Primary => ButtonLikeVariant::Accent,
            ButtonVariant::Ghost => ButtonLikeVariant::Ghost,
            ButtonVariant::Danger => ButtonLikeVariant::Danger,
        }
    }
}

/// Get dimensions for a button size.
fn size_dimensions(size: ButtonSize) -> (gpui::Pixels, gpui::Pixels, gpui::Pixels) {
    (size.height_px(), size.horizontal_padding_px(), size.text_size_px())
}

#[cfg(test)]
mod tests {
    use super::size_dimensions;
    use crate::components::ButtonSize;

    #[test]
    fn dimensions_follow_shared_button_size_contract() {
        let (h, px_val, text) = size_dimensions(ButtonSize::Large);
        let h: f32 = h.into();
        let px_val: f32 = px_val.into();
        let text: f32 = text.into();
        assert_eq!(h, 32.0);
        assert_eq!(px_val, 16.0);
        assert_eq!(text, 14.0);

        let (h, px_val, text) = size_dimensions(ButtonSize::Compact);
        let h: f32 = h.into();
        let px_val: f32 = px_val.into();
        let text: f32 = text.into();
        assert_eq!(h, 18.0);
        assert_eq!(px_val, 6.0);
        assert_eq!(text, 12.0);
    }
}
