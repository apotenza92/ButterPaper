//! Core data types for the workspace system.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Unique identifier for a tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabId(Uuid);

impl TabId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TabId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TabId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a tab bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabBarId(Uuid);

impl TabBarId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TabBarId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WindowId(Uuid);

impl WindowId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for WindowId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WindowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A single tab representing an open document.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tab {
    pub id: TabId,
    pub path: PathBuf,
    pub title: String,
    pub is_dirty: bool,
}

impl Tab {
    /// Create a new tab for a file path.
    pub fn new(path: PathBuf) -> Self {
        let title = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string());

        Self {
            id: TabId::new(),
            path,
            title,
            is_dirty: false,
        }
    }

    /// Set a custom title for the tab.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Mark the tab as dirty (has unsaved changes).
    pub fn set_dirty(&mut self, dirty: bool) {
        self.is_dirty = dirty;
    }
}

/// A tab bar containing multiple tabs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TabBar {
    pub id: TabBarId,
    pub tabs: Vec<Tab>,
    pub active_tab_index: usize,
}

impl TabBar {
    /// Create a new empty tab bar.
    pub fn new() -> Self {
        Self {
            id: TabBarId::new(),
            tabs: Vec::new(),
            active_tab_index: 0,
        }
    }

    /// Create a tab bar with an initial tab.
    pub fn with_tab(tab: Tab) -> Self {
        Self {
            id: TabBarId::new(),
            tabs: vec![tab],
            active_tab_index: 0,
        }
    }

    /// Add a new tab and make it active.
    pub fn add_tab(&mut self, tab: Tab) {
        self.tabs.push(tab);
        self.active_tab_index = self.tabs.len() - 1;
    }

    /// Remove a tab by ID. Returns true if the tab was found and removed.
    pub fn remove_tab(&mut self, tab_id: TabId) -> bool {
        if let Some(index) = self.tabs.iter().position(|t| t.id == tab_id) {
            self.tabs.remove(index);
            // Adjust active index if needed
            if self.active_tab_index >= self.tabs.len() && !self.tabs.is_empty() {
                self.active_tab_index = self.tabs.len() - 1;
            }
            true
        } else {
            false
        }
    }

    /// Set the active tab by ID.
    pub fn set_active_tab(&mut self, tab_id: TabId) -> bool {
        if let Some(index) = self.tabs.iter().position(|t| t.id == tab_id) {
            self.active_tab_index = index;
            true
        } else {
            false
        }
    }

    /// Get the currently active tab.
    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab_index)
    }

    /// Get the currently active tab mutably.
    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab_index)
    }

    /// Get the active tab ID.
    pub fn active_tab_id(&self) -> Option<TabId> {
        self.active_tab().map(|t| t.id)
    }

    /// Check if the tab bar is empty.
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// Get the number of tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Navigate to the next tab (wrapping around).
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab_index = (self.active_tab_index + 1) % self.tabs.len();
        }
    }

    /// Navigate to the previous tab (wrapping around).
    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            if self.active_tab_index == 0 {
                self.active_tab_index = self.tabs.len() - 1;
            } else {
                self.active_tab_index -= 1;
            }
        }
    }
}

impl Default for TabBar {
    fn default() -> Self {
        Self::new()
    }
}

/// An editor window containing one or more tab bars.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditorWindow {
    pub id: WindowId,
    pub tab_bar: TabBar,
    #[serde(skip)]
    pub is_focused: bool,
    /// Window bounds for restoration (x, y, width, height).
    pub bounds: Option<(i32, i32, u32, u32)>,
}

impl EditorWindow {
    /// Create a new empty window.
    pub fn new() -> Self {
        Self {
            id: WindowId::new(),
            tab_bar: TabBar::new(),
            is_focused: false,
            bounds: None,
        }
    }

    /// Create a window with an initial tab.
    pub fn with_tab(tab: Tab) -> Self {
        Self {
            id: WindowId::new(),
            tab_bar: TabBar::with_tab(tab),
            is_focused: false,
            bounds: None,
        }
    }

    /// Add a new tab to the window.
    pub fn add_tab(&mut self, tab: Tab) {
        self.tab_bar.add_tab(tab);
    }

    /// Close a tab. Returns true if the window should be closed (no tabs left).
    pub fn close_tab(&mut self, tab_id: TabId) -> bool {
        self.tab_bar.remove_tab(tab_id);
        self.tab_bar.is_empty()
    }

    /// Merge another window's tabs into this window.
    pub fn merge_from(&mut self, other: &EditorWindow) {
        for tab in &other.tab_bar.tabs {
            self.tab_bar.tabs.push(tab.clone());
        }
        // Set the first merged tab as active
        if !other.tab_bar.tabs.is_empty() {
            self.tab_bar.active_tab_index = self.tab_bar.tabs.len() - other.tab_bar.tabs.len();
        }
    }

    /// Get the active tab.
    pub fn active_tab(&self) -> Option<&Tab> {
        self.tab_bar.active_tab()
    }

    /// Get active tab ID.
    pub fn active_tab_id(&self) -> Option<TabId> {
        self.tab_bar.active_tab_id()
    }

    /// Check if window is empty (no tabs).
    pub fn is_empty(&self) -> bool {
        self.tab_bar.is_empty()
    }
}

impl Default for EditorWindow {
    fn default() -> Self {
        Self::new()
    }
}

/// User preferences for tab and window behavior.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TabPreferences {
    /// If true, new PDFs open as tabs in the current window.
    /// If false, new PDFs open in a new window.
    pub prefer_tabs: bool,

    /// If true, allow dragging tabs between windows to merge them.
    pub allow_merge: bool,

    /// If true, always show the tab bar even with a single tab.
    /// If false, hide the tab bar when there's only one tab.
    pub show_tab_bar: bool,
}

impl Default for TabPreferences {
    fn default() -> Self {
        Self {
            prefer_tabs: true,
            allow_merge: true,
            show_tab_bar: true,
        }
    }
}

/// Global workspace state managing all windows.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Workspace {
    pub windows: HashMap<WindowId, EditorWindow>,
    pub active_window_id: Option<WindowId>,
    pub preferences: TabPreferences,
}

impl Workspace {
    /// Create a new empty workspace.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new window to the workspace.
    pub fn add_window(&mut self, window: EditorWindow) -> WindowId {
        let id = window.id;
        self.windows.insert(id, window);
        self.active_window_id = Some(id);
        id
    }

    /// Remove a window from the workspace.
    pub fn remove_window(&mut self, window_id: WindowId) -> Option<EditorWindow> {
        let window = self.windows.remove(&window_id);
        if self.active_window_id == Some(window_id) {
            self.active_window_id = self.windows.keys().next().copied();
        }
        window
    }

    /// Get the active window.
    pub fn active_window(&self) -> Option<&EditorWindow> {
        self.active_window_id.and_then(|id| self.windows.get(&id))
    }

    /// Get the active window mutably.
    pub fn active_window_mut(&mut self) -> Option<&mut EditorWindow> {
        self.active_window_id
            .and_then(|id| self.windows.get_mut(&id))
    }

    /// Get a window by ID.
    pub fn get_window(&self, window_id: WindowId) -> Option<&EditorWindow> {
        self.windows.get(&window_id)
    }

    /// Get a window by ID mutably.
    pub fn get_window_mut(&mut self, window_id: WindowId) -> Option<&mut EditorWindow> {
        self.windows.get_mut(&window_id)
    }

    /// Set the active window.
    pub fn set_active_window(&mut self, window_id: WindowId) {
        if self.windows.contains_key(&window_id) {
            self.active_window_id = Some(window_id);
        }
    }

    /// Check if the workspace is empty (no windows).
    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    /// Get the number of windows.
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }
}
