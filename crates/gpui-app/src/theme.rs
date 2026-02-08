//! Theme system with support for loading themes from JSON
//!
//! Themes are loaded from JSON files in Zed's theme format.
//! Built-in themes are pulled from Zed's repository:
//! https://github.com/zed-industries/zed/tree/main/assets/themes
//!
//! The app auto-updates themes from GitHub and caches them locally.
//! Embedded themes serve as fallback when offline.

#![allow(dead_code)]

use gpui::{App, Global, Rgba, SharedString, Window, WindowAppearance};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::theme_updater;

/// User's preferred appearance mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppearanceMode {
    Light,
    Dark,
    #[default]
    System,
}

impl Global for AppearanceMode {}

impl AppearanceMode {
    /// Resolve the effective appearance based on mode and system setting
    pub fn resolve(&self, window_appearance: WindowAppearance) -> WindowAppearance {
        match self {
            AppearanceMode::Light => WindowAppearance::Light,
            AppearanceMode::Dark => WindowAppearance::Dark,
            AppearanceMode::System => window_appearance,
        }
    }
}

/// Get the current theme based on appearance mode and user's theme selection
pub fn current_theme(window: &Window, cx: &App) -> Theme {
    let mode = cx.try_global::<AppearanceMode>().copied().unwrap_or_default();
    let settings = cx.try_global::<ThemeSettings>().cloned().unwrap_or_default();
    let appearance = mode.resolve(window.appearance());
    let registry = theme_registry();

    match appearance {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => {
            registry.get_colors(&settings.dark_theme, true)
        }
        WindowAppearance::Light | WindowAppearance::VibrantLight => {
            registry.get_colors(&settings.light_theme, false)
        }
    }
}

/// Built-in theme JSON files (embedded as fallback)
const ONE_THEME_JSON: &str = include_str!("../assets/themes/one.json");

/// Theme family containing light and dark variants
#[derive(Debug, Clone, Deserialize)]
pub struct ThemeFamily {
    pub name: String,
    pub author: String,
    pub themes: Vec<ThemeDefinition>,
}

/// A single theme definition from JSON
#[derive(Debug, Clone, Deserialize)]
pub struct ThemeDefinition {
    pub name: String,
    pub appearance: String,
    pub style: ThemeStyle,
}

/// Theme style colors from JSON
#[derive(Debug, Clone, Deserialize)]
pub struct ThemeStyle {
    // Borders
    pub border: Option<String>,
    #[serde(rename = "border.variant")]
    pub border_variant: Option<String>,
    #[serde(rename = "border.focused")]
    pub border_focused: Option<String>,

    // Surfaces
    pub background: Option<String>,
    #[serde(rename = "surface.background")]
    pub surface_background: Option<String>,
    #[serde(rename = "elevated_surface.background")]
    pub elevated_surface_background: Option<String>,

    // Elements
    #[serde(rename = "element.background")]
    pub element_background: Option<String>,
    #[serde(rename = "element.hover")]
    pub element_hover: Option<String>,
    #[serde(rename = "element.active")]
    pub element_active: Option<String>,
    #[serde(rename = "element.selected")]
    pub element_selected: Option<String>,

    // Text
    pub text: Option<String>,
    #[serde(rename = "text.muted")]
    pub text_muted: Option<String>,
    #[serde(rename = "text.accent")]
    pub text_accent: Option<String>,

    // Semantic colors
    #[serde(rename = "error")]
    pub error: Option<String>,
    #[serde(rename = "error.background")]
    pub error_background: Option<String>,
    #[serde(rename = "error.border")]
    pub error_border: Option<String>,

    // Editor
    #[serde(rename = "editor.background")]
    pub editor_background: Option<String>,
    #[serde(rename = "editor.foreground")]
    pub editor_foreground: Option<String>,

    // All other fields captured here for future use
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

/// Parsed theme colors ready for use
#[derive(Clone, Copy)]
pub struct ThemeColors {
    pub background: Rgba,
    pub surface: Rgba,
    pub elevated_surface: Rgba,
    pub text: Rgba,
    pub text_muted: Rgba,
    pub border: Rgba,
    pub accent: Rgba,
    pub text_accent: Rgba,
    pub element: Rgba,
    pub element_hover: Rgba,
    pub element_selected: Rgba,
    pub danger: Rgba,
    pub danger_bg: Rgba,
    pub danger_border: Rgba,
}

/// Registry of available themes
pub struct ThemeRegistry {
    families: Vec<ThemeFamily>,
}

impl ThemeRegistry {
    /// Create a new registry with built-in themes
    pub fn new() -> Self {
        let mut registry = Self { families: Vec::new() };
        registry.load_builtin_themes();
        registry
    }

    fn load_builtin_themes(&mut self) {
        // Theme sources: (name, cache_key, embedded_fallback)
        let theme_sources = [("One", "one", ONE_THEME_JSON)];

        for (name, cache_key, embedded) in theme_sources {
            // Try cached version first (auto-updated from GitHub)
            let json =
                theme_updater::load_cached_theme(cache_key).unwrap_or_else(|| embedded.to_string());

            match serde_json::from_str::<ThemeFamily>(&json) {
                Ok(family) => self.families.push(family),
                Err(e) => eprintln!("Failed to parse {} theme: {}", name, e),
            }
        }
    }

    /// Reload themes from cache (called after an update)
    pub fn reload(&mut self) {
        self.families.clear();
        self.load_builtin_themes();
    }

    /// Get all available theme names
    pub fn theme_names(&self) -> Vec<(SharedString, SharedString)> {
        self.families
            .iter()
            .flat_map(|f| {
                f.themes.iter().map(|t| {
                    (SharedString::from(t.name.clone()), SharedString::from(t.appearance.clone()))
                })
            })
            .collect()
    }

    /// Find a theme by name
    pub fn get_theme(&self, name: &str) -> Option<&ThemeDefinition> {
        self.families.iter().flat_map(|f| f.themes.iter()).find(|t| t.name == name)
    }

    /// Get light themes
    pub fn light_themes(&self) -> Vec<&str> {
        self.families
            .iter()
            .flat_map(|f| f.themes.iter())
            .filter(|t| t.appearance == "light")
            .map(|t| t.name.as_str())
            .collect()
    }

    /// Get dark themes
    pub fn dark_themes(&self) -> Vec<&str> {
        self.families
            .iter()
            .flat_map(|f| f.themes.iter())
            .filter(|t| t.appearance == "dark")
            .map(|t| t.name.as_str())
            .collect()
    }

    /// Get theme colors by name, with fallback
    pub fn get_colors(&self, name: &str, is_dark: bool) -> ThemeColors {
        self.get_theme(name).map(|t| t.to_colors()).unwrap_or_else(|| {
            if is_dark {
                ThemeColors::fallback_dark()
            } else {
                ThemeColors::fallback_light()
            }
        })
    }
}

/// User's selected themes - stored globally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSettings {
    pub light_theme: String,
    pub dark_theme: String,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self { light_theme: "One Light".to_string(), dark_theme: "One Dark".to_string() }
    }
}

impl Global for ThemeSettings {}

impl Default for ThemeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeDefinition {
    /// Convert to usable theme colors
    pub fn to_colors(&self) -> ThemeColors {
        let style = &self.style;

        // For Gruvbox themes, editor.background has the meaningful hard/soft contrast difference
        // (e.g., #1d2021 vs #32302f), while elevated_surface.background is nearly identical.
        // Prefer editor.background for the main content area to show theme differences.
        let editor_bg = parse_color(style.editor_background.as_deref());
        let elevated_bg = parse_color(style.elevated_surface_background.as_deref());
        let danger = parse_color(style.error.as_deref()).unwrap_or(rgba(0xd14d5b, 1.0));
        let danger_bg =
            parse_color(style.error_background.as_deref()).unwrap_or(with_alpha(danger, 0.16));
        let danger_border =
            parse_color(style.error_border.as_deref()).unwrap_or(with_alpha(danger, 0.62));

        ThemeColors {
            background: parse_color(style.background.as_deref()).unwrap_or(rgba(0x282c33, 1.0)),
            surface: parse_color(style.surface_background.as_deref())
                .unwrap_or(rgba(0x2f343e, 1.0)),
            // Use editor.background if available - this is where Gruvbox hard/soft differences are
            elevated_surface: editor_bg.or(elevated_bg).unwrap_or(rgba(0x282c33, 1.0)),
            text: parse_color(style.text.as_deref()).unwrap_or(rgba(0xdce0e5, 1.0)),
            text_muted: parse_color(style.text_muted.as_deref()).unwrap_or(rgba(0xa9afbc, 1.0)),
            border: parse_color(style.border.as_deref()).unwrap_or(rgba(0x464b57, 1.0)),
            // Use focused border as the global accent source so interactive emphasis
            // follows the theme's focus contract (mode-specific by design).
            accent: parse_color(style.border_focused.as_deref())
                .or_else(|| parse_color(style.text_accent.as_deref()))
                .unwrap_or(rgba(0x47679e, 1.0)),
            text_accent: rgba(0xffffff, 1.0),
            element: parse_color(style.element_background.as_deref())
                .unwrap_or(rgba(0x2e343e, 1.0)),
            element_hover: parse_color(style.element_hover.as_deref())
                .unwrap_or(rgba(0x363c46, 1.0)),
            element_selected: parse_color(style.element_selected.as_deref())
                .or_else(|| parse_color(style.element_active.as_deref()))
                .unwrap_or(rgba(0x454a56, 1.0)),
            danger,
            danger_bg,
            danger_border,
        }
    }
}

impl ThemeColors {
    /// Fallback light theme if loading fails
    pub fn fallback_light() -> Self {
        Self {
            background: rgba(0xdcdcdd, 1.0),
            surface: rgba(0xebebec, 1.0),
            elevated_surface: rgba(0xfafafa, 1.0),
            text: rgba(0x242529, 1.0),
            text_muted: rgba(0x58585a, 1.0),
            border: rgba(0xc9c9ca, 1.0),
            accent: rgba(0x7d82e8, 1.0),
            text_accent: rgba(0xffffff, 1.0),
            element: rgba(0xebebec, 1.0),
            element_hover: rgba(0xdfdfe0, 1.0),
            element_selected: rgba(0xd0d0d1, 1.0),
            danger: rgba(0xd36151, 1.0),
            danger_bg: rgba(0xfbdfd9, 1.0),
            danger_border: rgba(0xf6c6bd, 1.0),
        }
    }

    /// Fallback dark theme if loading fails
    pub fn fallback_dark() -> Self {
        Self {
            background: rgba(0x3b414d, 1.0),
            surface: rgba(0x2f343e, 1.0),
            elevated_surface: rgba(0x282c33, 1.0),
            text: rgba(0xdce0e5, 1.0),
            text_muted: rgba(0xa9afbc, 1.0),
            border: rgba(0x464b57, 1.0),
            accent: rgba(0x47679e, 1.0),
            text_accent: rgba(0xffffff, 1.0),
            element: rgba(0x2e343e, 1.0),
            element_hover: rgba(0x363c46, 1.0),
            element_selected: rgba(0x454a56, 1.0),
            danger: rgba(0xd07277, 1.0),
            danger_bg: rgba(0xd07277, 0.102),
            danger_border: rgba(0x4c2b2c, 1.0),
        }
    }
}

/// Parse a hex color string like "#RRGGBBAA" or "#RRGGBB"
fn parse_color(s: Option<&str>) -> Option<Rgba> {
    let s = s?.trim_start_matches('#');
    if s.len() < 6 {
        return None;
    }

    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    let a = if s.len() >= 8 { u8::from_str_radix(&s[6..8], 16).ok()? } else { 255 };

    Some(Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: a as f32 / 255.0,
    })
}

/// Helper to create Rgba from hex and alpha
fn rgba(hex: u32, alpha: f32) -> Rgba {
    Rgba {
        r: ((hex >> 16) & 0xFF) as f32 / 255.0,
        g: ((hex >> 8) & 0xFF) as f32 / 255.0,
        b: (hex & 0xFF) as f32 / 255.0,
        a: alpha,
    }
}

fn with_alpha(color: Rgba, alpha_multiplier: f32) -> Rgba {
    Rgba { r: color.r, g: color.g, b: color.b, a: color.a * alpha_multiplier }
}

// Backwards compatibility alias
pub type Theme = ThemeColors;

/// Global theme registry instance
static THEME_REGISTRY: std::sync::OnceLock<ThemeRegistry> = std::sync::OnceLock::new();

/// Get the global theme registry
pub fn theme_registry() -> &'static ThemeRegistry {
    THEME_REGISTRY.get_or_init(ThemeRegistry::new)
}

#[cfg(test)]
mod tests {
    use super::{rgba, ThemeDefinition};

    fn parse_theme(style_json: &str) -> ThemeDefinition {
        let json = format!(r#"{{"name":"Test","appearance":"dark","style":{style_json}}}"#);
        serde_json::from_str(&json).expect("parse test theme")
    }

    fn assert_rgba_eq(left: gpui::Rgba, right: gpui::Rgba) {
        assert!((left.r - right.r).abs() < 0.0001);
        assert!((left.g - right.g).abs() < 0.0001);
        assert!((left.b - right.b).abs() < 0.0001);
        assert!((left.a - right.a).abs() < 0.0001);
    }

    #[test]
    fn to_colors_prefers_error_tokens_for_danger_palette() {
        let theme = parse_theme(
            r##"{
                "error":"#112233ff",
                "error.background":"#445566cc",
                "error.border":"#778899ff"
            }"##,
        );
        let colors = theme.to_colors();
        assert_rgba_eq(colors.danger, rgba(0x112233, 1.0));
        assert_rgba_eq(colors.danger_bg, rgba(0x445566, 0.8));
        assert_rgba_eq(colors.danger_border, rgba(0x778899, 1.0));
    }

    #[test]
    fn to_colors_derives_danger_background_and_border_when_missing() {
        let theme = parse_theme(r##"{"error":"#334455ff"}"##);
        let colors = theme.to_colors();
        assert_rgba_eq(colors.danger, rgba(0x334455, 1.0));
        assert_rgba_eq(colors.danger_bg, rgba(0x334455, 0.16));
        assert_rgba_eq(colors.danger_border, rgba(0x334455, 0.62));
    }
}
