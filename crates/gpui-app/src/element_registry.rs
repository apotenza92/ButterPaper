//! Registry for tracking UI element positions for automated testing
//!
//! This module provides a way to register clickable UI elements with their
//! screen coordinates, making it easy to automate UI interactions.
//!
//! Usage: Elements register themselves during render using `on_children_prepainted`,
//! and the CLI can query these positions to click on specific UI elements.
//!
//! ## Dev Mode
//!
//! When dev mode is enabled via `set_dev_mode(true)`, elements will dynamically
//! register their bounds during each render cycle. This provides accurate,
//! up-to-date positions for UI automation.

#![allow(dead_code)]

use gpui::{Bounds, Pixels};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, RwLock};

/// Whether dev mode is enabled (dynamic element tracking)
static DEV_MODE: AtomicBool = AtomicBool::new(false);

/// Enable or disable dev mode for dynamic element tracking
pub fn set_dev_mode(enabled: bool) {
    DEV_MODE.store(enabled, Ordering::SeqCst);
}

/// Check if dev mode is enabled
pub fn is_dev_mode() -> bool {
    DEV_MODE.load(Ordering::SeqCst)
}

/// Information about a registered UI element
#[derive(Debug, Clone)]
pub struct ElementInfo {
    /// Human-readable name of the element
    pub name: String,
    /// Window-relative bounds (in points)
    pub bounds: Bounds<Pixels>,
    /// Element type (button, dropdown, etc.)
    pub element_type: ElementType,
    /// Window title this element belongs to
    pub window_title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    Button,
    Dropdown,
    NavItem,
}

impl ElementInfo {
    /// Get the center point of this element (window-relative)
    pub fn center(&self) -> (i32, i32) {
        let x = (self.bounds.origin.x.0 + self.bounds.size.width.0 / 2.0) as i32;
        let y = (self.bounds.origin.y.0 + self.bounds.size.height.0 / 2.0) as i32;
        (x, y)
    }
}

/// Global static registry - accessible from anywhere
static ELEMENT_REGISTRY: LazyLock<RwLock<HashMap<String, ElementInfo>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Register an element with its bounds (only if dev mode is enabled)
pub fn register_element(
    id: &str,
    name: &str,
    bounds: Bounds<Pixels>,
    element_type: ElementType,
    window_title: &str,
) {
    if !is_dev_mode() {
        return;
    }
    if let Ok(mut elements) = ELEMENT_REGISTRY.write() {
        elements.insert(
            id.to_string(),
            ElementInfo {
                name: name.to_string(),
                bounds,
                element_type,
                window_title: window_title.to_string(),
            },
        );
    }
}

/// Register an element dynamically from prepaint bounds
///
/// Call this from `on_children_prepainted` to track element positions.
/// Only registers if dev mode is enabled.
pub fn register_from_bounds(
    id: &str,
    name: &str,
    bounds: &[Bounds<Pixels>],
    element_type: ElementType,
    window_title: &str,
) {
    if !is_dev_mode() || bounds.is_empty() {
        return;
    }
    // Use the first child's bounds (typically the clickable element)
    register_element(id, name, bounds[0], element_type, window_title);
}

/// Clear all elements (call before a new render cycle in dev mode)
pub fn clear_all_elements() {
    if let Ok(mut elements) = ELEMENT_REGISTRY.write() {
        elements.clear();
    }
}

/// Get all registered elements
pub fn get_all_elements() -> Vec<(String, ElementInfo)> {
    if let Ok(elements) = ELEMENT_REGISTRY.read() {
        elements.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    } else {
        Vec::new()
    }
}

/// Get element by ID
pub fn get_element(id: &str) -> Option<ElementInfo> {
    if let Ok(elements) = ELEMENT_REGISTRY.read() {
        elements.get(id).cloned()
    } else {
        None
    }
}

/// Clear all elements for a specific window (call before re-render)
pub fn clear_window_elements(window_title: &str) {
    if let Ok(mut elements) = ELEMENT_REGISTRY.write() {
        elements.retain(|_, v| v.window_title != window_title);
    }
}

/// Helper to output elements as JSON for CLI tooling
pub fn elements_to_json(elements: &[(String, ElementInfo)]) -> String {
    let mut json = String::from("{\n  \"elements\": [\n");
    for (i, (id, info)) in elements.iter().enumerate() {
        let (cx, cy) = info.center();
        json.push_str(&format!(
            "    {{\"id\": \"{}\", \"name\": \"{}\", \"type\": \"{:?}\", \"window\": \"{}\", \"x\": {}, \"y\": {}, \"w\": {}, \"h\": {}, \"center\": [{}, {}]}}",
            id,
            info.name,
            info.element_type,
            info.window_title,
            info.bounds.origin.x.0 as i32,
            info.bounds.origin.y.0 as i32,
            info.bounds.size.width.0 as i32,
            info.bounds.size.height.0 as i32,
            cx,
            cy
        ));
        if i < elements.len() - 1 {
            json.push_str(",\n");
        } else {
            json.push('\n');
        }
    }
    json.push_str("  ]\n}");
    json
}

/// Print elements in a human-readable table format
pub fn print_elements_table(elements: &[(String, ElementInfo)]) {
    println!("{:<25} {:<20} {:<10} {:<15} {:<10}", "ID", "Name", "Type", "Position", "Size");
    println!("{}", "-".repeat(80));
    for (id, info) in elements {
        let (cx, cy) = info.center();
        println!(
            "{:<25} {:<20} {:<10} ({:>4},{:>4}) {:>4}x{:<4} center=({},{})",
            id,
            info.name,
            format!("{:?}", info.element_type),
            info.bounds.origin.x.0 as i32,
            info.bounds.origin.y.0 as i32,
            info.bounds.size.width.0 as i32,
            info.bounds.size.height.0 as i32,
            cx,
            cy
        );
    }
}
