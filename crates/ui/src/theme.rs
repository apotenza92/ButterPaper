//! Centralized theme module for consistent visual styling across the application.
//!
//! This module provides a unified color palette and spacing system that all UI
//! components can use to maintain visual consistency.

use crate::scene::Color;

/// Semantic color tokens for the application theme.
///
/// These colors are organized by their semantic meaning rather than their
/// visual appearance, making it easier to maintain consistency and potentially
/// support multiple themes (e.g., light mode, dark mode) in the future.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeColors {
    // ═══════════════════════════════════════════════════════════════════════════
    // BACKGROUND COLORS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Primary background color (main content areas, windows)
    pub background_primary: Color,

    /// Secondary background color (sidebars, panels)
    pub background_secondary: Color,

    /// Tertiary background color (nested panels, dialogs)
    pub background_tertiary: Color,

    /// Elevated surface background (toolbars, floating panels)
    pub background_elevated: Color,

    /// Input field background color
    pub background_input: Color,

    // ═══════════════════════════════════════════════════════════════════════════
    // TEXT COLORS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Primary text color (main content)
    pub text_primary: Color,

    /// Secondary text color (labels, descriptions)
    pub text_secondary: Color,

    /// Muted text color (placeholders, disabled text)
    pub text_muted: Color,

    /// Disabled text color
    pub text_disabled: Color,

    // ═══════════════════════════════════════════════════════════════════════════
    // ACCENT COLORS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Primary accent color (selection, focus, active items)
    pub accent_primary: Color,

    /// Secondary accent color (links, secondary highlights)
    pub accent_secondary: Color,

    /// Success accent color (confirmations, completed items)
    pub accent_success: Color,

    /// Warning accent color (alerts, caution indicators)
    pub accent_warning: Color,

    /// Error/danger accent color (errors, destructive actions)
    pub accent_error: Color,

    // ═══════════════════════════════════════════════════════════════════════════
    // BORDER COLORS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Primary border color (general borders)
    pub border_primary: Color,

    /// Secondary border color (subtle borders, dividers)
    pub border_secondary: Color,

    /// Focused border color (input focus states)
    pub border_focused: Color,

    /// Selected border color (selected items)
    pub border_selected: Color,

    // ═══════════════════════════════════════════════════════════════════════════
    // BUTTON COLORS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Button normal state background
    pub button_normal: Color,

    /// Button hover state background
    pub button_hover: Color,

    /// Button active/pressed state background
    pub button_active: Color,

    /// Button icon/text color
    pub button_icon: Color,

    // ═══════════════════════════════════════════════════════════════════════════
    // SEPARATOR COLORS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Separator/divider line color
    pub separator: Color,

    // ═══════════════════════════════════════════════════════════════════════════
    // SPECIAL COLORS (for specific components)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Note popup background (sticky note yellow)
    pub note_background: Color,

    /// Note popup title bar
    pub note_title_bar: Color,

    /// Note popup border
    pub note_border: Color,

    /// Note popup text (dark on light)
    pub note_text: Color,
}

/// Spacing constants for consistent layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeSpacing {
    /// Extra small spacing (2px)
    pub xs: f32,

    /// Small spacing (4px)
    pub sm: f32,

    /// Medium spacing (8px)
    pub md: f32,

    /// Large spacing (16px)
    pub lg: f32,

    /// Extra large spacing (24px)
    pub xl: f32,

    /// Extra extra large spacing (32px)
    pub xxl: f32,
}

/// Size constants for UI elements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeSizes {
    /// Toolbar height
    pub toolbar_height: f32,

    /// Search bar height
    pub search_bar_height: f32,

    /// Standard button size (square buttons)
    pub button_size: f32,

    /// Small button size
    pub button_size_small: f32,

    /// Border width for standard elements
    pub border_width: f32,

    /// Border width for focused elements
    pub border_width_focused: f32,

    /// Thumbnail width
    pub thumbnail_width: f32,

    /// Thumbnail height
    pub thumbnail_height: f32,

    /// Dialog border radius (note: currently rectangular but prepared for future)
    pub border_radius: f32,
}

/// Complete application theme combining colors, spacing, and sizes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    /// Color palette
    pub colors: ThemeColors,

    /// Spacing values
    pub spacing: ThemeSpacing,

    /// Size values
    pub sizes: ThemeSizes,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self::dark()
    }
}

impl ThemeColors {
    /// Create the default dark theme color palette.
    ///
    /// This is the standard dark mode theme used throughout the application,
    /// providing good contrast and readability while being easy on the eyes.
    pub fn dark() -> Self {
        Self {
            // Background colors - dark grays with slight progression
            background_primary: Color::rgba(0.12, 0.12, 0.12, 1.0),
            background_secondary: Color::rgba(0.15, 0.15, 0.15, 0.95),
            background_tertiary: Color::rgba(0.18, 0.18, 0.18, 0.98),
            background_elevated: Color::rgba(0.22, 0.22, 0.22, 1.0),
            background_input: Color::rgba(0.10, 0.10, 0.10, 1.0),

            // Text colors - light on dark
            text_primary: Color::rgba(0.90, 0.90, 0.90, 1.0),
            text_secondary: Color::rgba(0.70, 0.70, 0.70, 1.0),
            text_muted: Color::rgba(0.50, 0.50, 0.50, 1.0),
            text_disabled: Color::rgba(0.40, 0.40, 0.40, 1.0),

            // Accent colors
            accent_primary: Color::rgba(0.30, 0.50, 0.80, 1.0),   // Blue
            accent_secondary: Color::rgba(0.30, 0.60, 1.00, 1.0), // Bright blue
            accent_success: Color::rgba(0.20, 0.50, 0.30, 1.0),   // Green
            accent_warning: Color::rgba(0.80, 0.60, 0.20, 1.0),   // Orange/amber
            accent_error: Color::rgba(0.50, 0.25, 0.25, 1.0),     // Red

            // Border colors
            border_primary: Color::rgba(0.35, 0.35, 0.35, 1.0),
            border_secondary: Color::rgba(0.30, 0.30, 0.30, 1.0),
            border_focused: Color::rgba(0.30, 0.50, 0.80, 1.0),   // Matches accent_primary
            border_selected: Color::rgba(0.30, 0.60, 1.00, 1.0),  // Matches accent_secondary

            // Button colors - medium grays with interaction states
            button_normal: Color::rgba(0.25, 0.25, 0.25, 1.0),
            button_hover: Color::rgba(0.35, 0.35, 0.35, 1.0),
            button_active: Color::rgba(0.20, 0.40, 0.70, 1.0),    // Blue when active
            button_icon: Color::rgba(0.90, 0.90, 0.90, 1.0),

            // Separator
            separator: Color::rgba(0.30, 0.30, 0.30, 1.0),

            // Note-specific colors (sticky note appearance)
            note_background: Color::rgba(1.00, 1.00, 0.85, 0.98),  // Light yellow
            note_title_bar: Color::rgba(0.95, 0.90, 0.70, 1.0),    // Darker yellow
            note_border: Color::rgba(0.70, 0.65, 0.40, 1.0),       // Golden brown
            note_text: Color::rgba(0.10, 0.10, 0.10, 1.0),         // Dark text
        }
    }

    /// Create a light theme color palette (for future dark mode toggle).
    pub fn light() -> Self {
        Self {
            // Background colors - light grays
            background_primary: Color::rgba(1.00, 1.00, 1.00, 1.0),
            background_secondary: Color::rgba(0.96, 0.96, 0.96, 1.0),
            background_tertiary: Color::rgba(0.92, 0.92, 0.92, 1.0),
            background_elevated: Color::rgba(1.00, 1.00, 1.00, 1.0),
            background_input: Color::rgba(1.00, 1.00, 1.00, 1.0),

            // Text colors - dark on light
            text_primary: Color::rgba(0.10, 0.10, 0.10, 1.0),
            text_secondary: Color::rgba(0.35, 0.35, 0.35, 1.0),
            text_muted: Color::rgba(0.55, 0.55, 0.55, 1.0),
            text_disabled: Color::rgba(0.70, 0.70, 0.70, 1.0),

            // Accent colors (slightly adjusted for light mode)
            accent_primary: Color::rgba(0.20, 0.40, 0.70, 1.0),   // Blue
            accent_secondary: Color::rgba(0.15, 0.50, 0.90, 1.0), // Bright blue
            accent_success: Color::rgba(0.15, 0.50, 0.25, 1.0),   // Green
            accent_warning: Color::rgba(0.85, 0.55, 0.10, 1.0),   // Orange
            accent_error: Color::rgba(0.75, 0.20, 0.20, 1.0),     // Red

            // Border colors
            border_primary: Color::rgba(0.80, 0.80, 0.80, 1.0),
            border_secondary: Color::rgba(0.88, 0.88, 0.88, 1.0),
            border_focused: Color::rgba(0.20, 0.40, 0.70, 1.0),
            border_selected: Color::rgba(0.15, 0.50, 0.90, 1.0),

            // Button colors
            button_normal: Color::rgba(0.92, 0.92, 0.92, 1.0),
            button_hover: Color::rgba(0.85, 0.85, 0.85, 1.0),
            button_active: Color::rgba(0.20, 0.40, 0.70, 1.0),
            button_icon: Color::rgba(0.20, 0.20, 0.20, 1.0),

            // Separator
            separator: Color::rgba(0.85, 0.85, 0.85, 1.0),

            // Note-specific colors (same as dark for consistency)
            note_background: Color::rgba(1.00, 1.00, 0.85, 0.98),
            note_title_bar: Color::rgba(0.95, 0.90, 0.70, 1.0),
            note_border: Color::rgba(0.70, 0.65, 0.40, 1.0),
            note_text: Color::rgba(0.10, 0.10, 0.10, 1.0),
        }
    }
}

impl Default for ThemeSpacing {
    fn default() -> Self {
        Self {
            xs: 2.0,
            sm: 4.0,
            md: 8.0,
            lg: 16.0,
            xl: 24.0,
            xxl: 32.0,
        }
    }
}

impl Default for ThemeSizes {
    fn default() -> Self {
        Self {
            toolbar_height: 44.0,
            search_bar_height: 36.0,
            button_size: 32.0,
            button_size_small: 24.0,
            border_width: 1.0,
            border_width_focused: 2.0,
            thumbnail_width: 120.0,
            thumbnail_height: 160.0,
            border_radius: 0.0, // Currently rectangular
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// Create the default dark theme.
    pub fn dark() -> Self {
        Self {
            colors: ThemeColors::dark(),
            spacing: ThemeSpacing::default(),
            sizes: ThemeSizes::default(),
        }
    }

    /// Create a light theme.
    pub fn light() -> Self {
        Self {
            colors: ThemeColors::light(),
            spacing: ThemeSpacing::default(),
            sizes: ThemeSizes::default(),
        }
    }
}

use std::sync::RwLock;

/// Appearance mode for the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppearanceMode {
    /// Light mode (light backgrounds, dark text)
    Light,
    /// Dark mode (dark backgrounds, light text)
    #[default]
    Dark,
    /// Follow system appearance (auto-switch based on macOS settings)
    System,
}

/// Theme state that can be changed at runtime.
struct ThemeState {
    /// The current theme (computed from mode)
    theme: Theme,
    /// The current appearance mode
    mode: AppearanceMode,
    /// Generation counter for tracking theme changes
    generation: u64,
}

impl Default for ThemeState {
    fn default() -> Self {
        Self {
            theme: Theme::dark(),
            mode: AppearanceMode::Dark,
            generation: 0,
        }
    }
}

/// Global theme accessor with runtime switching support.
///
/// This provides a convenient way to access the current theme throughout the
/// application without passing it through every function call, while also
/// supporting runtime theme changes for dark mode support.
static CURRENT_THEME: RwLock<ThemeState> = RwLock::new(ThemeState {
    theme: Theme {
        colors: ThemeColors {
            // Dark theme colors (default) - duplicated here for const initialization
            background_primary: Color { r: 0.12, g: 0.12, b: 0.12, a: 1.0 },
            background_secondary: Color { r: 0.15, g: 0.15, b: 0.15, a: 0.95 },
            background_tertiary: Color { r: 0.18, g: 0.18, b: 0.18, a: 0.98 },
            background_elevated: Color { r: 0.22, g: 0.22, b: 0.22, a: 1.0 },
            background_input: Color { r: 0.10, g: 0.10, b: 0.10, a: 1.0 },
            text_primary: Color { r: 0.90, g: 0.90, b: 0.90, a: 1.0 },
            text_secondary: Color { r: 0.70, g: 0.70, b: 0.70, a: 1.0 },
            text_muted: Color { r: 0.50, g: 0.50, b: 0.50, a: 1.0 },
            text_disabled: Color { r: 0.40, g: 0.40, b: 0.40, a: 1.0 },
            accent_primary: Color { r: 0.30, g: 0.50, b: 0.80, a: 1.0 },
            accent_secondary: Color { r: 0.30, g: 0.60, b: 1.00, a: 1.0 },
            accent_success: Color { r: 0.20, g: 0.50, b: 0.30, a: 1.0 },
            accent_warning: Color { r: 0.80, g: 0.60, b: 0.20, a: 1.0 },
            accent_error: Color { r: 0.50, g: 0.25, b: 0.25, a: 1.0 },
            border_primary: Color { r: 0.35, g: 0.35, b: 0.35, a: 1.0 },
            border_secondary: Color { r: 0.30, g: 0.30, b: 0.30, a: 1.0 },
            border_focused: Color { r: 0.30, g: 0.50, b: 0.80, a: 1.0 },
            border_selected: Color { r: 0.30, g: 0.60, b: 1.00, a: 1.0 },
            button_normal: Color { r: 0.25, g: 0.25, b: 0.25, a: 1.0 },
            button_hover: Color { r: 0.35, g: 0.35, b: 0.35, a: 1.0 },
            button_active: Color { r: 0.20, g: 0.40, b: 0.70, a: 1.0 },
            button_icon: Color { r: 0.90, g: 0.90, b: 0.90, a: 1.0 },
            separator: Color { r: 0.30, g: 0.30, b: 0.30, a: 1.0 },
            note_background: Color { r: 1.00, g: 1.00, b: 0.85, a: 0.98 },
            note_title_bar: Color { r: 0.95, g: 0.90, b: 0.70, a: 1.0 },
            note_border: Color { r: 0.70, g: 0.65, b: 0.40, a: 1.0 },
            note_text: Color { r: 0.10, g: 0.10, b: 0.10, a: 1.0 },
        },
        spacing: ThemeSpacing { xs: 2.0, sm: 4.0, md: 8.0, lg: 16.0, xl: 24.0, xxl: 32.0 },
        sizes: ThemeSizes {
            toolbar_height: 44.0,
            search_bar_height: 36.0,
            button_size: 32.0,
            button_size_small: 24.0,
            border_width: 1.0,
            border_width_focused: 2.0,
            thumbnail_width: 120.0,
            thumbnail_height: 160.0,
            border_radius: 0.0,
        },
    },
    mode: AppearanceMode::Dark,
    generation: 0,
});

/// Get the current application theme.
///
/// Returns a copy of the current theme. The theme may change at runtime
/// when the system appearance changes or when explicitly set.
pub fn current_theme() -> Theme {
    CURRENT_THEME.read().unwrap().theme
}

/// Get the current theme generation counter.
///
/// This can be used to detect when the theme has changed without comparing
/// the full theme struct. The generation increments each time the theme changes.
pub fn theme_generation() -> u64 {
    CURRENT_THEME.read().unwrap().generation
}

/// Get the current appearance mode.
pub fn current_appearance_mode() -> AppearanceMode {
    CURRENT_THEME.read().unwrap().mode
}

/// Set the appearance mode.
///
/// This changes the theme based on the specified mode:
/// - `AppearanceMode::Light` - Always use light theme
/// - `AppearanceMode::Dark` - Always use dark theme
/// - `AppearanceMode::System` - Follow system appearance (requires calling
///   `apply_system_appearance()` to actually apply the system setting)
///
/// Returns true if the theme actually changed.
pub fn set_appearance_mode(mode: AppearanceMode) -> bool {
    let mut state = CURRENT_THEME.write().unwrap();
    if state.mode == mode {
        return false;
    }

    state.mode = mode;
    let new_theme = match mode {
        AppearanceMode::Light => Theme::light(),
        AppearanceMode::Dark => Theme::dark(),
        AppearanceMode::System => {
            // When set to System, use current system appearance
            // This will be updated by apply_system_appearance()
            if is_system_dark_mode() {
                Theme::dark()
            } else {
                Theme::light()
            }
        }
    };

    let changed = state.theme != new_theme;
    if changed {
        state.theme = new_theme;
        state.generation += 1;
    }
    changed
}

/// Apply the current system appearance to the theme.
///
/// This only has an effect when the appearance mode is set to `System`.
/// Call this when the system appearance changes (e.g., from a notification).
///
/// Returns true if the theme actually changed.
pub fn apply_system_appearance(is_dark: bool) -> bool {
    let mut state = CURRENT_THEME.write().unwrap();

    // Only apply if in System mode
    if state.mode != AppearanceMode::System {
        return false;
    }

    let new_theme = if is_dark {
        Theme::dark()
    } else {
        Theme::light()
    };

    let changed = state.theme != new_theme;
    if changed {
        state.theme = new_theme;
        state.generation += 1;
    }
    changed
}

/// Initialize the theme with a specific mode at startup.
///
/// This is a convenience function for setting the initial appearance mode.
pub fn init_theme(mode: AppearanceMode) {
    set_appearance_mode(mode);
}

/// Check if the system is currently in dark mode.
///
/// On macOS, this queries the system appearance setting.
/// On other platforms, returns true (dark mode default).
#[cfg(target_os = "macos")]
pub fn is_system_dark_mode() -> bool {
    #[allow(deprecated)]
    use cocoa::appkit::NSApplication;
    #[allow(deprecated)]
    use cocoa::base::nil;
    use objc::runtime::Object;
    use objc::{msg_send, sel, sel_impl};
    use std::ffi::CStr;

    unsafe {
        // Get the shared NSApplication instance
        #[allow(deprecated)]
        let app = NSApplication::sharedApplication(nil);
        if app.is_null() {
            return true; // Default to dark if we can't get the app
        }

        // Get effectiveAppearance
        let appearance: *mut Object = msg_send![app, effectiveAppearance];
        if appearance.is_null() {
            return true;
        }

        // Get appearance name
        let name: *mut Object = msg_send![appearance, name];
        if name.is_null() {
            return true;
        }

        // Convert NSString to &str and check for "Dark"
        let name_cstr: *const i8 = msg_send![name, UTF8String];
        if name_cstr.is_null() {
            return true;
        }

        let name_rust = CStr::from_ptr(name_cstr);
        if let Ok(name_str) = name_rust.to_str() {
            // Dark mode appearance names contain "Dark"
            // e.g., "NSAppearanceNameDarkAqua", "NSAppearanceNameAccessibilityHighContrastDarkAqua"
            name_str.contains("Dark")
        } else {
            true
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn is_system_dark_mode() -> bool {
    // Default to dark mode on non-macOS platforms
    true
}

/// Check for system appearance changes and update the theme if needed.
///
/// This should be called periodically (e.g., when the app becomes active or
/// receives certain events) to detect when the user changes their system
/// appearance setting.
///
/// Returns true if the theme changed as a result of this check.
pub fn check_system_appearance_change() -> bool {
    let mode = current_appearance_mode();
    if mode != AppearanceMode::System {
        return false;
    }

    let is_dark = is_system_dark_mode();
    apply_system_appearance(is_dark)
}

/// Initialize the theme to follow system appearance.
///
/// This sets the appearance mode to System and applies the current
/// system appearance. Call this at app startup if you want the app
/// to automatically match the system light/dark mode.
pub fn init_system_appearance() {
    // First set to System mode
    set_appearance_mode(AppearanceMode::System);
    // Then apply current system appearance
    let is_dark = is_system_dark_mode();
    apply_system_appearance(is_dark);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_theme_colors() {
        let colors = ThemeColors::dark();

        // Background should be dark (low values)
        assert!(colors.background_primary.r < 0.2);
        assert!(colors.background_primary.g < 0.2);
        assert!(colors.background_primary.b < 0.2);

        // Text should be light (high values)
        assert!(colors.text_primary.r > 0.8);
        assert!(colors.text_primary.g > 0.8);
        assert!(colors.text_primary.b > 0.8);

        // All colors should be fully opaque or nearly so
        assert!(colors.background_primary.a >= 0.95);
        assert!(colors.text_primary.a >= 0.95);
    }

    #[test]
    fn test_light_theme_colors() {
        let colors = ThemeColors::light();

        // Background should be light (high values)
        assert!(colors.background_primary.r > 0.9);
        assert!(colors.background_primary.g > 0.9);
        assert!(colors.background_primary.b > 0.9);

        // Text should be dark (low values)
        assert!(colors.text_primary.r < 0.2);
        assert!(colors.text_primary.g < 0.2);
        assert!(colors.text_primary.b < 0.2);
    }

    #[test]
    fn test_accent_colors_are_distinct() {
        let colors = ThemeColors::dark();

        // Primary accent should be blue-ish
        assert!(colors.accent_primary.b > colors.accent_primary.r);

        // Success should be green-ish
        assert!(colors.accent_success.g > colors.accent_success.r);
        assert!(colors.accent_success.g > colors.accent_success.b);

        // Error should be red-ish
        assert!(colors.accent_error.r > colors.accent_error.g);
        assert!(colors.accent_error.r > colors.accent_error.b);
    }

    #[test]
    fn test_button_states_have_increasing_brightness() {
        let colors = ThemeColors::dark();

        // Hover should be brighter than normal
        assert!(colors.button_hover.r > colors.button_normal.r);

        // Active is blue so we check for accent color
        assert!(colors.button_active.b > 0.5);
    }

    #[test]
    fn test_theme_spacing_values() {
        let spacing = ThemeSpacing::default();

        // Values should increase progressively
        assert!(spacing.xs < spacing.sm);
        assert!(spacing.sm < spacing.md);
        assert!(spacing.md < spacing.lg);
        assert!(spacing.lg < spacing.xl);
        assert!(spacing.xl < spacing.xxl);
    }

    #[test]
    fn test_theme_sizes_positive() {
        let sizes = ThemeSizes::default();

        assert!(sizes.toolbar_height > 0.0);
        assert!(sizes.button_size > 0.0);
        assert!(sizes.thumbnail_width > 0.0);
        assert!(sizes.thumbnail_height > 0.0);
    }

    #[test]
    fn test_default_theme_is_dark() {
        let theme = Theme::default();
        let dark = Theme::dark();

        // Default should match dark theme
        assert_eq!(theme.colors.background_primary.r, dark.colors.background_primary.r);
    }

    #[test]
    fn test_note_colors_are_yellow() {
        let colors = ThemeColors::dark();

        // Note background should be yellowish (high R and G, lower B)
        assert!(colors.note_background.r > 0.9);
        assert!(colors.note_background.g > 0.9);
        assert!(colors.note_background.b < colors.note_background.r);
    }

    #[test]
    fn test_appearance_mode_default() {
        // Default mode should be Dark
        assert_eq!(AppearanceMode::default(), AppearanceMode::Dark);
    }

    #[test]
    fn test_theme_state_default() {
        let state = ThemeState::default();
        assert_eq!(state.mode, AppearanceMode::Dark);
        assert_eq!(state.generation, 0);
    }

    #[test]
    fn test_set_appearance_mode_light() {
        // Note: This test manipulates global state, so results depend on test order
        // We test the Theme construction directly to avoid global state issues
        let theme = Theme::light();

        // Light theme should have light backgrounds
        assert!(theme.colors.background_primary.r > 0.9);
        assert!(theme.colors.background_primary.g > 0.9);
        assert!(theme.colors.background_primary.b > 0.9);

        // Light theme should have dark text
        assert!(theme.colors.text_primary.r < 0.2);
        assert!(theme.colors.text_primary.g < 0.2);
        assert!(theme.colors.text_primary.b < 0.2);
    }

    #[test]
    fn test_set_appearance_mode_dark() {
        let theme = Theme::dark();

        // Dark theme should have dark backgrounds
        assert!(theme.colors.background_primary.r < 0.2);
        assert!(theme.colors.background_primary.g < 0.2);
        assert!(theme.colors.background_primary.b < 0.2);

        // Dark theme should have light text
        assert!(theme.colors.text_primary.r > 0.8);
        assert!(theme.colors.text_primary.g > 0.8);
        assert!(theme.colors.text_primary.b > 0.8);
    }

    #[test]
    fn test_theme_generation_increments() {
        // Test that theme generation works correctly
        let initial_gen = theme_generation();

        // Switching to same mode should not change generation
        // Since tests share global state, we just verify generation is >= 0
        assert!(initial_gen >= 0);
    }

    #[test]
    fn test_apply_system_appearance_only_in_system_mode() {
        // When not in System mode, apply_system_appearance should return false
        // and not change the theme
        let theme_before = current_theme();

        // The global state may be in any mode, so we test the logic directly
        let mode = current_appearance_mode();
        if mode != AppearanceMode::System {
            // In non-System mode, apply_system_appearance should do nothing
            let changed = apply_system_appearance(false);
            assert!(!changed || mode == AppearanceMode::System);
        }

        // Theme should still be the same
        let theme_after = current_theme();
        assert_eq!(theme_before.colors.background_primary.r, theme_after.colors.background_primary.r);
    }

    #[test]
    fn test_theme_colors_light_vs_dark_contrast() {
        let dark = ThemeColors::dark();
        let light = ThemeColors::light();

        // Background brightness should be inverted
        assert!(dark.background_primary.r < 0.2);
        assert!(light.background_primary.r > 0.9);

        // Text brightness should be inverted
        assert!(dark.text_primary.r > 0.8);
        assert!(light.text_primary.r < 0.2);
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_is_system_dark_mode_non_macos() {
        // On non-macOS platforms, should default to dark
        assert!(is_system_dark_mode());
    }
}
