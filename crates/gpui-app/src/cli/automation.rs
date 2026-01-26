//! Mouse and UI element automation.

use super::MouseAction;
use crate::element_registry;

/// Simulate mouse action at screen coordinates (cross-platform using enigo)
pub fn simulate_mouse(action: &MouseAction, window_x: i32, window_y: i32) {
    use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};

    let (rel_x, rel_y, is_click) = match action {
        MouseAction::MoveTo(x, y) => (*x, *y, false),
        MouseAction::Click(x, y) => (*x, *y, true),
    };

    // Convert window-relative to screen coordinates
    let screen_x = window_x + rel_x;
    let screen_y = window_y + rel_y;

    eprintln!(
        "Mouse action at screen ({}, {}) [window at ({}, {}), offset ({}, {})]",
        screen_x, screen_y, window_x, window_y, rel_x, rel_y
    );

    let settings = Settings::default();
    eprintln!("Creating enigo with settings: {:?}", settings);

    let mut enigo = match Enigo::new(&settings) {
        Ok(e) => {
            eprintln!("Enigo created successfully");
            e
        }
        Err(e) => {
            eprintln!("Failed to create enigo instance: {:?}", e);
            eprintln!("On macOS, ensure Accessibility permissions are granted in:");
            eprintln!("  System Settings > Privacy & Security > Accessibility");
            return;
        }
    };

    // Check current mouse position
    if let Ok((cur_x, cur_y)) = enigo.location() {
        eprintln!("Current mouse position: ({}, {})", cur_x, cur_y);
    }

    // Move to target position
    if let Err(e) = enigo.move_mouse(screen_x, screen_y, Coordinate::Abs) {
        eprintln!("Failed to move mouse: {:?}", e);
        return;
    }

    // Small delay for move to register
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Verify mouse moved
    if let Ok((cur_x, cur_y)) = enigo.location() {
        eprintln!("Mouse after move: ({}, {})", cur_x, cur_y);
        if cur_x != screen_x || cur_y != screen_y {
            eprintln!("WARNING: Mouse did not move to expected position!");
        }
    }

    if is_click {
        // Click (press + release)
        if let Err(e) = enigo.button(Button::Left, Direction::Click) {
            eprintln!("Failed to click: {:?}", e);
            return;
        }
        eprintln!("Click performed at ({}, {})", screen_x, screen_y);
    }

    // Delay for UI to react
    std::thread::sleep(std::time::Duration::from_millis(200));
}

/// List available UI elements with their positions (requires Settings window to be open with --dev)
#[allow(dead_code)]
pub fn list_elements() {
    let elements = element_registry::get_all_elements();
    if elements.is_empty() {
        eprintln!("No elements registered. Make sure:");
        eprintln!("  1. The Settings window is open (--settings)");
        eprintln!("  2. Dev mode is enabled (--dev)");
        eprintln!();
        eprintln!("Example: butterpaper --settings --dev");
        std::process::exit(1);
    }
    println!("UI Elements (dynamically tracked):");
    println!();
    element_registry::print_elements_table(&elements);
    println!();
    println!("Usage: butterpaper --click-element <id> --window-title Settings");
    std::process::exit(0);
}

/// Click on a UI element by ID (requires Settings window to be open with --dev)
#[allow(dead_code)]
pub fn click_element(element_id: &str, window_title: Option<&str>) {
    use xcap::Window;

    let elements = element_registry::get_all_elements();
    if elements.is_empty() {
        eprintln!("No elements registered. Run with --settings --dev first.");
        std::process::exit(1);
    }
    let element = elements.iter().find(|(id, _)| id == element_id);

    let (_, info) = match element {
        Some(e) => e,
        None => {
            eprintln!("Element not found: {}", element_id);
            eprintln!("Available elements:");
            for (id, info) in &elements {
                eprintln!("  {} - {}", id, info.name);
            }
            std::process::exit(1);
        }
    };

    // Find the window
    let windows = match Window::all() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to list windows: {}", e);
            std::process::exit(1);
        }
    };

    let target_title = window_title.unwrap_or(&info.window_title);
    let window = windows
        .iter()
        .find(|w| w.title().unwrap_or_default().contains(target_title));

    let window = match window {
        Some(w) => w,
        None => {
            eprintln!("Window not found: {}", target_title);
            std::process::exit(1);
        }
    };

    let win_x = window.x().unwrap_or(0);
    let win_y = window.y().unwrap_or(0);
    let (rel_x, rel_y) = info.center();

    // With transparent titlebar, xcap captures the full window including title bar.
    // Element coordinates from calculate_settings_elements() are measured from
    // xcap screenshots, so they're already in GPUI coordinates.
    // screen = window_position + gpui_coords (no additional offset needed)
    let screen_x = win_x + rel_x;
    let screen_y = win_y + rel_y;

    eprintln!(
        "Clicking element '{}' ({}) at window-relative ({}, {}), screen ({}, {})",
        element_id, info.name, rel_x, rel_y, screen_x, screen_y
    );

    use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};
    let settings = Settings::default();
    if let Ok(mut enigo) = Enigo::new(&settings) {
        let _ = enigo.move_mouse(screen_x, screen_y, Coordinate::Abs);
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = enigo.button(Button::Left, Direction::Click);
        std::thread::sleep(std::time::Duration::from_millis(100));
        eprintln!("Click sent!");
    } else {
        eprintln!("Failed to create enigo instance");
        std::process::exit(1);
    }

    std::process::exit(0);
}
