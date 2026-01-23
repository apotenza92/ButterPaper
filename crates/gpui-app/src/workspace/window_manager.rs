//! Window management operations for the workspace.

#![allow(dead_code)]

use std::path::PathBuf;

use super::types::{EditorWindow, Tab, TabId, WindowId, Workspace};

/// Open a PDF file according to user preferences.
///
/// If `prefer_tabs` is true and there's an active window, adds a new tab.
/// Otherwise, creates a new window.
pub fn open_pdf(workspace: &mut Workspace, path: PathBuf) -> (WindowId, TabId) {
    let tab = Tab::new(path);
    let tab_id = tab.id;

    if workspace.preferences.prefer_tabs {
        // Try to add to active window
        if let Some(window) = workspace.active_window_mut() {
            window.add_tab(tab);
            return (window.id, tab_id);
        }
    }

    // Create new window
    let window = EditorWindow::with_tab(tab);
    let window_id = workspace.add_window(window);
    (window_id, tab_id)
}

/// Close a specific tab.
///
/// Returns `Some(window_id)` if the window should be closed (was the last tab),
/// or `None` if the window still has tabs.
pub fn close_tab(
    workspace: &mut Workspace,
    window_id: WindowId,
    tab_id: TabId,
) -> Option<WindowId> {
    if let Some(window) = workspace.get_window_mut(window_id) {
        let should_close_window = window.close_tab(tab_id);
        if should_close_window {
            return Some(window_id);
        }
    }
    None
}

/// Close a window entirely.
pub fn close_window(workspace: &mut Workspace, window_id: WindowId) {
    workspace.remove_window(window_id);
}

/// Merge source window into target window.
///
/// All tabs from source are moved to target, and source window is closed.
pub fn merge_windows(workspace: &mut Workspace, source_id: WindowId, target_id: WindowId) {
    if source_id == target_id {
        return;
    }

    // Get source window tabs
    let source_tabs = workspace
        .get_window(source_id)
        .map(|w| w.tab_bar.tabs.clone())
        .unwrap_or_default();

    // Add tabs to target
    if let Some(target) = workspace.get_window_mut(target_id) {
        for tab in source_tabs {
            target.tab_bar.tabs.push(tab);
        }
        // Set first merged tab as active
        target.tab_bar.active_tab_index = target.tab_bar.tabs.len().saturating_sub(1);
    }

    // Remove source window
    workspace.remove_window(source_id);
}

/// Get the active tab of a window.
pub fn active_tab(workspace: &Workspace, window_id: WindowId) -> Option<&Tab> {
    workspace.get_window(window_id).and_then(|w| w.active_tab())
}

/// Move a tab from one window to another.
///
/// Returns `Some(window_id)` if the source window should be closed (was the last tab).
pub fn move_tab(
    workspace: &mut Workspace,
    source_window_id: WindowId,
    target_window_id: WindowId,
    tab_id: TabId,
) -> Option<WindowId> {
    if source_window_id == target_window_id {
        return None;
    }

    // Find and clone the tab
    let tab = workspace
        .get_window(source_window_id)
        .and_then(|w| w.tab_bar.tabs.iter().find(|t| t.id == tab_id).cloned());

    let tab = tab?;

    // Remove from source
    let should_close = if let Some(source) = workspace.get_window_mut(source_window_id) {
        source.close_tab(tab_id)
    } else {
        false
    };

    // Add to target
    if let Some(target) = workspace.get_window_mut(target_window_id) {
        target.add_tab(tab);
    }

    if should_close {
        Some(source_window_id)
    } else {
        None
    }
}

/// Create a new window from a tab (for drag-to-new-window).
///
/// Returns the new window ID and `Some(old_window_id)` if the old window should be closed.
pub fn detach_tab_to_new_window(
    workspace: &mut Workspace,
    source_window_id: WindowId,
    tab_id: TabId,
) -> (WindowId, Option<WindowId>) {
    // Find and clone the tab
    let tab = workspace
        .get_window(source_window_id)
        .and_then(|w| w.tab_bar.tabs.iter().find(|t| t.id == tab_id).cloned());

    let Some(tab) = tab else {
        // Tab not found, return a new empty window
        let window = EditorWindow::new();
        let id = workspace.add_window(window);
        return (id, None);
    };

    // Remove from source
    let should_close = if let Some(source) = workspace.get_window_mut(source_window_id) {
        source.close_tab(tab_id)
    } else {
        false
    };

    // Create new window with the tab
    let window = EditorWindow::with_tab(tab);
    let new_window_id = workspace.add_window(window);

    let close_source = if should_close {
        Some(source_window_id)
    } else {
        None
    };

    (new_window_id, close_source)
}
