//! Shared "button-like" styling primitives.
//!
//! This mirrors Zed's approach: one central style contract for all clickable
//! controls (text buttons, icon buttons, segmented triggers, etc.).

use gpui::{px, Pixels, Rems, Rgba, StatefulInteractiveElement, Styled};

use crate::styles::units::rems_from_px;
use crate::ui::color;
use crate::ui::StatefulInteractiveExt;
use crate::Theme;

/// Visual tone for button-like controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonLikeVariant {
    Neutral,
    Accent,
    Ghost,
    Danger,
}

/// Shared button sizing contract used across interactive controls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonSize {
    Large,
    Medium,
    #[default]
    Default,
    Compact,
    None,
}

impl ButtonSize {
    pub fn rems(self) -> Rems {
        match self {
            ButtonSize::Large => rems_from_px(32.0),
            ButtonSize::Medium => rems_from_px(28.0),
            ButtonSize::Default => rems_from_px(22.0),
            ButtonSize::Compact => rems_from_px(18.0),
            ButtonSize::None => rems_from_px(16.0),
        }
    }

    pub fn height_px(self) -> Pixels {
        match self {
            ButtonSize::Large => px(32.0),
            ButtonSize::Medium => px(28.0),
            ButtonSize::Default => px(22.0),
            ButtonSize::Compact => px(18.0),
            ButtonSize::None => px(16.0),
        }
    }

    pub fn horizontal_padding_px(self) -> Pixels {
        match self {
            ButtonSize::Large => px(16.0),
            ButtonSize::Medium => px(12.0),
            ButtonSize::Default => px(8.0),
            ButtonSize::Compact => px(6.0),
            ButtonSize::None => px(0.0),
        }
    }

    pub fn text_size_px(self) -> Pixels {
        match self {
            ButtonSize::Compact | ButtonSize::None => px(12.0),
            ButtonSize::Default | ButtonSize::Medium | ButtonSize::Large => px(14.0),
        }
    }

    pub fn shortcut_text_size_px(self) -> Pixels {
        match self {
            ButtonSize::Compact | ButtonSize::None => px(10.0),
            ButtonSize::Default | ButtonSize::Medium | ButtonSize::Large => px(12.0),
        }
    }

    pub fn icon_size_px(self) -> f32 {
        match self {
            ButtonSize::Large => 20.0,
            ButtonSize::Medium => 18.0,
            ButtonSize::Default => 16.0,
            ButtonSize::Compact => 14.0,
            ButtonSize::None => 12.0,
        }
    }
}

/// Color bundle for button-like controls.
#[derive(Clone, Copy, Debug)]
pub struct ButtonLikeColors {
    pub background: Rgba,
    pub text: Rgba,
    pub border: Rgba,
    pub hover: Rgba,
    pub active: Rgba,
}

/// Shared subtle border used by neutral controls.
pub fn subtle_border(theme: &Theme) -> Rgba {
    color::subtle_border(theme.border)
}

/// Muted text for disabled controls.
pub fn disabled_text(theme: &Theme) -> Rgba {
    color::disabled(theme.text_muted)
}

fn mix_color(left: Rgba, right: Rgba, factor: f32) -> Rgba {
    let t = factor.clamp(0.0, 1.0);
    Rgba {
        r: left.r * (1.0 - t) + right.r * t,
        g: left.g * (1.0 - t) + right.g * t,
        b: left.b * (1.0 - t) + right.b * t,
        a: left.a,
    }
}

/// Standardized colors for a button-like variant.
pub fn variant_colors(variant: ButtonLikeVariant, theme: &Theme) -> ButtonLikeColors {
    match variant {
        ButtonLikeVariant::Neutral => ButtonLikeColors {
            background: theme.elevated_surface,
            text: theme.text,
            border: subtle_border(theme),
            hover: theme.element_hover,
            active: theme.element_selected,
        },
        ButtonLikeVariant::Accent => ButtonLikeColors {
            background: theme.accent,
            text: theme.text_accent,
            border: Rgba {
                r: theme.accent.r,
                g: theme.accent.g,
                b: theme.accent.b,
                a: color::strong_border(theme.accent).a,
            },
            hover: Rgba {
                r: theme.accent.r * 0.9,
                g: theme.accent.g * 0.9,
                b: theme.accent.b * 0.9,
                a: theme.accent.a,
            },
            active: Rgba {
                r: theme.accent.r * 0.8,
                g: theme.accent.g * 0.8,
                b: theme.accent.b * 0.8,
                a: theme.accent.a,
            },
        },
        ButtonLikeVariant::Ghost => ButtonLikeColors {
            background: color::transparent(),
            text: theme.text,
            border: color::transparent(),
            hover: theme.element_hover,
            active: theme.element_selected,
        },
        ButtonLikeVariant::Danger => ButtonLikeColors {
            background: theme.danger,
            text: theme.text_accent,
            border: theme.danger_border,
            hover: mix_color(theme.danger, theme.danger_bg, 0.14),
            active: mix_color(theme.danger, theme.danger_border, 0.22),
        },
    }
}

/// Styling extension for all stateful interactive controls.
pub trait ButtonLikeExt: StatefulInteractiveElement + Styled + Sized {
    fn button_like(self, colors: ButtonLikeColors, radius: Pixels) -> Self {
        self.bg(colors.background)
            .border_1()
            .border_color(colors.border)
            .text_color(colors.text)
            .rounded(radius)
            .interactive_bg(colors.hover, colors.active)
    }

    fn button_like_focus_ring(self, ring_color: Rgba) -> Self {
        let _ = ring_color;
        self
    }
}

impl<T: StatefulInteractiveElement + Styled> ButtonLikeExt for T {}

#[cfg(test)]
mod tests {
    use super::{disabled_text, subtle_border, variant_colors, ButtonLikeVariant, ButtonSize};
    use crate::theme::ThemeColors;
    use gpui::Rgba;

    #[test]
    fn subtle_border_is_lower_alpha_than_base_border() {
        let theme = ThemeColors::fallback_light();
        let subtle = subtle_border(&theme);
        assert!(subtle.a < theme.border.a);
    }

    #[test]
    fn disabled_text_reduces_alpha() {
        let theme = ThemeColors::fallback_dark();
        let disabled = disabled_text(&theme);
        assert!(disabled.a < theme.text_muted.a);
    }

    #[test]
    fn accent_variant_uses_accent_background() {
        let theme = ThemeColors::fallback_light();
        let colors = variant_colors(ButtonLikeVariant::Accent, &theme);
        assert_eq!(colors.background, theme.accent);
    }

    #[test]
    fn danger_variant_uses_theme_danger_tokens() {
        let mut theme = ThemeColors::fallback_dark();
        theme.danger = Rgba { r: 0.8, g: 0.1, b: 0.2, a: 1.0 };
        theme.danger_bg = Rgba { r: 0.95, g: 0.75, b: 0.75, a: 0.4 };
        theme.danger_border = Rgba { r: 0.6, g: 0.1, b: 0.15, a: 0.9 };

        let colors = variant_colors(ButtonLikeVariant::Danger, &theme);
        assert_eq!(colors.background, theme.danger);
        assert_eq!(colors.border, theme.danger_border);
        assert_eq!(colors.text, theme.text_accent);
    }

    #[test]
    fn button_size_matches_zed_ladder() {
        let large: f32 = ButtonSize::Large.height_px().into();
        let medium: f32 = ButtonSize::Medium.height_px().into();
        let default: f32 = ButtonSize::Default.height_px().into();
        let compact: f32 = ButtonSize::Compact.height_px().into();
        let none: f32 = ButtonSize::None.height_px().into();

        assert_eq!(large, 32.0);
        assert_eq!(medium, 28.0);
        assert_eq!(default, 22.0);
        assert_eq!(compact, 18.0);
        assert_eq!(none, 16.0);
    }
}
