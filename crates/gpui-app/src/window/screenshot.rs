//! Screenshot capture functionality.

use crate::cli::MouseAction;
use std::path::PathBuf;

/// Schedule a screenshot using xcap (cross-platform window capture)
pub fn schedule_screenshot(
    path: PathBuf,
    delay_ms: u64,
    window_id: Option<u32>,
    window_title: Option<String>,
    mouse_action: Option<MouseAction>,
) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));

        match capture_window(window_id, window_title.as_deref(), mouse_action.as_ref(), &path) {
            Ok(()) => {
                eprintln!("Screenshot saved to: {}", path.display());
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Screenshot failed: {}", e);
                std::process::exit(1);
            }
        }
    });
}

/// Capture a window by ID or title using xcap (cross-platform)
pub fn capture_window(
    window_id: Option<u32>,
    window_title: Option<&str>,
    mouse_action: Option<&MouseAction>,
    path: &std::path::Path,
) -> Result<(), String> {
    use crate::cli::simulate_mouse;
    use xcap::Window;

    let windows = Window::all().map_err(|e| format!("Failed to list windows: {}", e))?;

    // Find window by ID first (most precise)
    let window = if let Some(id) = window_id {
        windows.iter().find(|w| w.id().unwrap_or(0) == id)
    } else if let Some(title) = window_title {
        // Find by title (partial match)
        windows.iter().find(|w| w.title().unwrap_or_default().contains(title))
    } else {
        // No filter - show available windows and error
        let available: Vec<String> = windows
            .iter()
            .filter(|w| !w.title().unwrap_or_default().is_empty())
            .map(|w| {
                format!(
                    "{}: {} - {}",
                    w.id().unwrap_or(0),
                    w.app_name().unwrap_or_default(),
                    w.title().unwrap_or_default()
                )
            })
            .collect();

        return Err(format!(
            "No window specified. Use --window-id or --window-title.\nAvailable windows:\n{}",
            available.join("\n")
        ));
    };

    let window = window.ok_or_else(|| {
        "Window not found. Use --list-windows to see available windows.".to_string()
    })?;

    let id = window.id().unwrap_or(0);
    let app = window.app_name().unwrap_or_default();
    let title = window.title().unwrap_or_default();
    let win_x = window.x().unwrap_or(0);
    let win_y = window.y().unwrap_or(0);
    let win_w = window.width().unwrap_or(0);
    let win_h = window.height().unwrap_or(0);

    eprintln!(
        "Capturing: {} - {} (ID: {}, pos: {}, {}, size: {}x{})",
        app, title, id, win_x, win_y, win_w, win_h
    );

    // Perform mouse action if specified
    if let Some(action) = mouse_action {
        simulate_mouse(action, win_x, win_y);
        // Extra delay for UI to react to interaction
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    let image = window.capture_image().map_err(|e| format!("Failed to capture window: {}", e))?;

    image.save(path).map_err(|e| format!("Failed to save screenshot: {}", e))?;

    Ok(())
}
