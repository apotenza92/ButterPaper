//! Persistence layer for workspace state and preferences.

#![allow(dead_code)]

use std::path::PathBuf;

use super::types::{EditorWindow, TabPreferences, Workspace};

/// Get the config directory for butterpaper.
fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("butterpaper"))
}

/// Get the path to the preferences file.
fn preferences_path() -> Option<PathBuf> {
    config_dir().map(|p| p.join("preferences.json"))
}

/// Get the path to the layout file.
fn layout_path() -> Option<PathBuf> {
    config_dir().map(|p| p.join("layout.json"))
}

/// Load preferences from disk, or return defaults.
pub fn load_preferences() -> TabPreferences {
    let Some(path) = preferences_path() else {
        return TabPreferences::default();
    };

    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => TabPreferences::default(),
    }
}

/// Save preferences to disk.
pub fn save_preferences(prefs: &TabPreferences) -> std::io::Result<()> {
    let Some(path) = preferences_path() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine config directory",
        ));
    };

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(prefs)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    std::fs::write(path, json)
}

/// Saved window state for restoration.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SavedWindow {
    pub bounds: Option<(i32, i32, u32, u32)>,
    pub tabs: Vec<SavedTab>,
    pub active_tab_index: usize,
}

/// Saved tab state for restoration.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SavedTab {
    pub path: PathBuf,
}

/// Layout state for restoration.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SavedLayout {
    pub windows: Vec<SavedWindow>,
    pub active_window_index: Option<usize>,
}

impl From<&Workspace> for SavedLayout {
    fn from(workspace: &Workspace) -> Self {
        let windows: Vec<SavedWindow> = workspace
            .windows
            .values()
            .map(|w| SavedWindow {
                bounds: w.bounds,
                tabs: w
                    .tab_bar
                    .tabs
                    .iter()
                    .map(|t| SavedTab {
                        path: t.path.clone(),
                    })
                    .collect(),
                active_tab_index: w.tab_bar.active_tab_index,
            })
            .collect();

        let active_window_index = workspace
            .active_window_id
            .and_then(|id| workspace.windows.keys().position(|&k| k == id));

        SavedLayout {
            windows,
            active_window_index,
        }
    }
}

/// Load layout from disk.
pub fn load_layout() -> Option<SavedLayout> {
    let path = layout_path()?;

    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).ok(),
        Err(_) => None,
    }
}

/// Save layout to disk.
pub fn save_layout(workspace: &Workspace) -> std::io::Result<()> {
    let Some(path) = layout_path() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine config directory",
        ));
    };

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let layout = SavedLayout::from(workspace);
    let json = serde_json::to_string_pretty(&layout)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    std::fs::write(path, json)
}

/// Restore workspace from saved layout.
pub fn restore_workspace(layout: &SavedLayout) -> Workspace {
    use super::types::{Tab, TabBar};

    let mut workspace = Workspace::new();

    for (idx, saved_window) in layout.windows.iter().enumerate() {
        // Skip windows with no valid tabs (files that don't exist)
        let tabs: Vec<Tab> = saved_window
            .tabs
            .iter()
            .filter(|t| t.path.exists())
            .map(|t| Tab::new(t.path.clone()))
            .collect();

        if tabs.is_empty() {
            continue;
        }

        let mut tab_bar = TabBar::new();
        for tab in tabs {
            tab_bar.tabs.push(tab);
        }
        // Restore active tab index, clamping to valid range
        tab_bar.active_tab_index = saved_window
            .active_tab_index
            .min(tab_bar.tabs.len().saturating_sub(1));

        let mut window = EditorWindow::new();
        window.tab_bar = tab_bar;
        window.bounds = saved_window.bounds;

        let window_id = window.id;
        workspace.windows.insert(window_id, window);

        // Set active window
        if layout.active_window_index == Some(idx) {
            workspace.active_window_id = Some(window_id);
        }
    }

    // If no active window set, pick the first one
    if workspace.active_window_id.is_none() && !workspace.windows.is_empty() {
        workspace.active_window_id = workspace.windows.keys().next().copied();
    }

    workspace
}
