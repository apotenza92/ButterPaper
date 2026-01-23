//! Window listing and focusing functionality.

/// List all capturable windows (with optional verbose mode showing positions)
pub fn list_windows() {
    use xcap::Window;

    match Window::all() {
        Ok(windows) => {
            println!("Capturable windows:");
            println!("{:<8} {:<30} {:<20} Title", "ID", "App", "Position");
            println!("{}", "-".repeat(100));
            for w in windows {
                let id = w.id().unwrap_or(0);
                let app = w.app_name().unwrap_or_default();
                let title = w.title().unwrap_or_default();
                let x = w.x().unwrap_or(0);
                let y = w.y().unwrap_or(0);
                let width = w.width().unwrap_or(0);
                let height = w.height().unwrap_or(0);
                // Skip windows with no title (usually system windows)
                if !title.is_empty() {
                    println!(
                        "{:<8} {:<30} ({},{}) {}x{}  {}",
                        id, app, x, y, width, height, title
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to list windows: {}", e);
            std::process::exit(1);
        }
    }
    std::process::exit(0);
}

/// Focus a window by ID or title using macOS accessibility APIs
#[cfg(target_os = "macos")]
pub fn focus_window(window_id: Option<u32>, window_title: Option<&str>) {
    use xcap::Window;

    let windows = match Window::all() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to list windows: {}", e);
            std::process::exit(1);
        }
    };

    // Find window
    let window = if let Some(id) = window_id {
        windows.iter().find(|w| w.id().unwrap_or(0) == id)
    } else if let Some(title) = window_title {
        windows
            .iter()
            .find(|w| w.title().unwrap_or_default().contains(title))
    } else {
        eprintln!("Specify --window-id or --window-title with --focus");
        std::process::exit(1);
    };

    let window = match window {
        Some(w) => w,
        None => {
            eprintln!("Window not found");
            std::process::exit(1);
        }
    };

    let title = window.title().unwrap_or_default();
    let app = window.app_name().unwrap_or_default();

    eprintln!("Focusing window: {} (app: {})", title, app);

    // Use AppleScript with AXRaise to bring window to front
    let script = format!(
        r#"
        tell application "System Events"
            tell process "{}"
                set frontmost to true
                repeat with w in windows
                    if name of w contains "{}" then
                        perform action "AXRaise" of w
                    end if
                end repeat
            end tell
        end tell
        "#,
        app, title
    );

    match std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                eprintln!("Window raised successfully");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("AppleScript error: {}", stderr);
            }
        }
        Err(e) => {
            eprintln!("Failed to run osascript: {}", e);
        }
    }

    std::process::exit(0);
}

#[cfg(target_os = "windows")]
pub fn focus_window(window_id: Option<u32>, window_title: Option<&str>) {
    use xcap::Window;

    let windows = match Window::all() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to list windows: {}", e);
            std::process::exit(1);
        }
    };

    let window = if let Some(id) = window_id {
        windows.iter().find(|w| w.id().unwrap_or(0) == id)
    } else if let Some(title) = window_title {
        windows
            .iter()
            .find(|w| w.title().unwrap_or_default().contains(title))
    } else {
        eprintln!("Specify --window-id or --window-title with --focus");
        std::process::exit(1);
    };

    let window = match window {
        Some(w) => w,
        None => {
            eprintln!("Window not found");
            std::process::exit(1);
        }
    };

    let title = window.title().unwrap_or_default();
    eprintln!("Focusing window: {}", title);

    // Use PowerShell to bring window to front
    let script = format!(
        r#"Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public class Win32 {{ [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd); }}'; $hwnd = (Get-Process | Where-Object {{$_.MainWindowTitle -like "*{}*"}}).MainWindowHandle; [Win32]::SetForegroundWindow($hwnd)"#,
        title
    );

    match std::process::Command::new("powershell")
        .arg("-Command")
        .arg(&script)
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                eprintln!("Window raised successfully");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("PowerShell error: {}", stderr);
            }
        }
        Err(e) => {
            eprintln!("Failed to run powershell: {}", e);
        }
    }

    std::process::exit(0);
}

#[cfg(target_os = "linux")]
pub fn focus_window(window_id: Option<u32>, window_title: Option<&str>) {
    use xcap::Window;

    let windows = match Window::all() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to list windows: {}", e);
            std::process::exit(1);
        }
    };

    let window = if let Some(id) = window_id {
        windows.iter().find(|w| w.id().unwrap_or(0) == id)
    } else if let Some(title) = window_title {
        windows
            .iter()
            .find(|w| w.title().unwrap_or_default().contains(title))
    } else {
        eprintln!("Specify --window-id or --window-title with --focus");
        std::process::exit(1);
    };

    let window = match window {
        Some(w) => w,
        None => {
            eprintln!("Window not found");
            std::process::exit(1);
        }
    };

    let id = window.id().unwrap_or(0);
    let title = window.title().unwrap_or_default();
    eprintln!("Focusing window: {} (ID: {})", title, id);

    // Use wmctrl or xdotool to bring window to front
    // Try xdotool first (more common)
    let result = std::process::Command::new("xdotool")
        .arg("windowactivate")
        .arg(id.to_string())
        .output();

    match result {
        Ok(output) if output.status.success() => {
            eprintln!("Window raised successfully (xdotool)");
        }
        _ => {
            // Fallback to wmctrl
            let result = std::process::Command::new("wmctrl")
                .arg("-i")
                .arg("-a")
                .arg(format!("0x{:x}", id))
                .output();

            match result {
                Ok(output) if output.status.success() => {
                    eprintln!("Window raised successfully (wmctrl)");
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!("wmctrl error: {}", stderr);
                }
                Err(e) => {
                    eprintln!("Failed to run wmctrl: {}. Install xdotool or wmctrl.", e);
                }
            }
        }
    }

    std::process::exit(0);
}
