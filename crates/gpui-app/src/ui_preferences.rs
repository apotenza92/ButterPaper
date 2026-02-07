//! Persistence for app-level UI preferences (appearance/theme).

use std::path::PathBuf;

use gpui::App;

use crate::theme::{AppearanceMode, ThemeSettings};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct UiPreferences {
    pub appearance_mode: AppearanceMode,
    pub theme_settings: ThemeSettings,
}

fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("butterpaper"))
}

fn preferences_path() -> Option<PathBuf> {
    config_dir().map(|p| p.join("ui_preferences.json"))
}

pub fn load_ui_preferences() -> UiPreferences {
    let Some(path) = preferences_path() else {
        return UiPreferences::default();
    };

    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => UiPreferences::default(),
    }
}

pub fn save_ui_preferences(preferences: &UiPreferences) -> std::io::Result<()> {
    let Some(path) = preferences_path() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine config directory",
        ));
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(preferences)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    std::fs::write(path, json)
}

pub fn collect_ui_preferences(cx: &App) -> UiPreferences {
    UiPreferences {
        appearance_mode: cx.try_global::<AppearanceMode>().copied().unwrap_or_default(),
        theme_settings: cx.try_global::<ThemeSettings>().cloned().unwrap_or_default(),
    }
}

pub fn save_ui_preferences_from_app(cx: &App) -> std::io::Result<()> {
    let prefs = collect_ui_preferences(cx);
    save_ui_preferences(&prefs)
}

#[cfg(test)]
mod tests {
    use super::UiPreferences;
    use crate::theme::AppearanceMode;

    #[test]
    fn ui_preferences_json_roundtrip() {
        let prefs =
            UiPreferences { appearance_mode: AppearanceMode::Dark, ..UiPreferences::default() };

        let json = serde_json::to_string(&prefs).expect("serialize prefs");
        let decoded: UiPreferences = serde_json::from_str(&json).expect("deserialize prefs");
        assert_eq!(decoded.appearance_mode, AppearanceMode::Dark);
    }

    #[test]
    fn legacy_density_field_is_ignored_on_load() {
        let legacy = r#"{
            "appearance_mode":"Dark",
            "theme_settings":{"light_theme":"One Light","dark_theme":"One Dark"},
            "ui_density":"Comfortable"
        }"#;
        let decoded: UiPreferences =
            serde_json::from_str(legacy).expect("deserialize legacy prefs");
        assert_eq!(decoded.appearance_mode, AppearanceMode::Dark);
        assert_eq!(decoded.theme_settings.light_theme, "One Light");
        assert_eq!(decoded.theme_settings.dark_theme, "One Dark");
    }
}
