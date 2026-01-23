//! Standard button component with variants and sizes.

use gpui::{div, prelude::*, px, ClickEvent, Rgba, SharedString, Window};

use crate::ui::{sizes, StatefulInteractiveExt};
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

/// Button size
#[derive(Clone, Copy, Default)]
pub enum ButtonSize {
    /// Small: 24px height
    Sm,
    /// Medium: 28px height (default)
    #[default]
    Md,
    /// Large: 32px height
    Lg,
}

/// Create a button with the specified variant and size.
///
/// # Example
/// ```ignore
/// button(
///     "save-btn",
///     "Save",
///     ButtonVariant::Primary,
///     ButtonSize::Md,
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
    let (bg, text, hover_bg, active_bg) = variant_colors(variant, theme);
    let (height, px_val, text_size) = size_dimensions(size);

    div()
        .id(id.into())
        .h(height)
        .px(px_val)
        .flex()
        .items_center()
        .justify_center()
        .rounded(sizes::RADIUS_SM)
        .bg(bg)
        .text_color(text)
        .text_size(text_size)
        .cursor_pointer()
        .interactive_bg(hover_bg, active_bg)
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
    button(id, label_str, ButtonVariant::Default, ButtonSize::Md, theme, on_click)
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
    button(id, label_str, ButtonVariant::Primary, ButtonSize::Md, theme, on_click)
}

/// Get colors for a button variant.
fn variant_colors(variant: ButtonVariant, theme: &Theme) -> (Rgba, Rgba, Rgba, Rgba) {
    match variant {
        ButtonVariant::Default => (
            theme.surface,
            theme.text,
            theme.element_hover,
            theme.element_selected,
        ),
        ButtonVariant::Primary => (
            theme.accent,
            theme.text_accent,
            // Darken accent for hover/active - simple approach
            Rgba {
                r: theme.accent.r * 0.9,
                g: theme.accent.g * 0.9,
                b: theme.accent.b * 0.9,
                a: theme.accent.a,
            },
            Rgba {
                r: theme.accent.r * 0.8,
                g: theme.accent.g * 0.8,
                b: theme.accent.b * 0.8,
                a: theme.accent.a,
            },
        ),
        ButtonVariant::Ghost => (
            Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
            theme.text,
            theme.element_hover,
            theme.element_selected,
        ),
        ButtonVariant::Danger => {
            // Use a red-ish color for danger
            let danger = Rgba {
                r: 0.85,
                g: 0.25,
                b: 0.25,
                a: 1.0,
            };
            (
                danger,
                theme.text_accent,
                Rgba {
                    r: danger.r * 0.9,
                    g: danger.g * 0.9,
                    b: danger.b * 0.9,
                    a: danger.a,
                },
                Rgba {
                    r: danger.r * 0.8,
                    g: danger.g * 0.8,
                    b: danger.b * 0.8,
                    a: danger.a,
                },
            )
        }
    }
}

/// Get dimensions for a button size.
fn size_dimensions(size: ButtonSize) -> (gpui::Pixels, gpui::Pixels, gpui::Pixels) {
    match size {
        ButtonSize::Sm => (px(24.0), px(8.0), px(12.0)),
        ButtonSize::Md => (px(28.0), px(12.0), px(14.0)),
        ButtonSize::Lg => (px(32.0), px(16.0), px(14.0)),
    }
}
