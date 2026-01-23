mod cache;
mod components;
mod element_registry;
#[cfg(target_os = "macos")]
mod macos;
mod settings;
mod sidebar;
mod theme;
mod theme_updater;
mod ui;
mod viewport;
mod workspace;

pub use element_registry::{ElementInfo, ElementType};

use gpui::{
    actions, div, point, prelude::*, px, size, App, Application, Bounds, Context, Entity,
    FocusHandle, Focusable, Global, KeyBinding, Menu, MenuItem, TitlebarOptions, Window,
    WindowAppearance, WindowBounds, WindowOptions,
};
use sidebar::ThumbnailSidebar;
use std::path::PathBuf;
pub use theme::{Theme, ThemeSettings};
use viewport::PdfViewport;

actions!(
    pdf_editor,
    [
        Quit,
        Open,
        About,
        ZoomIn,
        ZoomOut,
        NextPage,
        PrevPage,
        CloseWindow,
        NextTab,
        PrevTab,
        CloseTab
    ]
);

/// User's preferred appearance mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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
    let mode = cx
        .try_global::<AppearanceMode>()
        .copied()
        .unwrap_or_default();
    let settings = cx
        .try_global::<ThemeSettings>()
        .cloned()
        .unwrap_or_default();
    let appearance = mode.resolve(window.appearance());
    let registry = theme::theme_registry();

    match appearance {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => {
            registry.get_colors(&settings.dark_theme, true)
        }
        WindowAppearance::Light | WindowAppearance::VibrantLight => {
            registry.get_colors(&settings.light_theme, false)
        }
    }
}

/// Mouse action to perform before screenshot
#[derive(Clone)]
enum MouseAction {
    MoveTo(i32, i32), // Hover at window-relative coordinates
    Click(i32, i32),  // Click at window-relative coordinates
}

/// Parse "x,y" coordinate string
fn parse_coords(s: &str) -> Option<(i32, i32)> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() == 2 {
        let x = parts[0].trim().parse().ok()?;
        let y = parts[1].trim().parse().ok()?;
        Some((x, y))
    } else {
        None
    }
}

/// Parsed command line arguments
struct CliArgs {
    files: Vec<PathBuf>,
    screenshot: Option<PathBuf>,
    screenshot_delay_ms: u64,
    window_id: Option<u32>,
    window_title: Option<String>,
    mouse_action: Option<MouseAction>,
    list_windows: bool,
    open_settings: bool,
    focus_window: bool,
    list_elements: bool,
    click_element: Option<String>,
    dev_mode: bool,
}

/// Parse command line arguments
fn parse_args() -> CliArgs {
    let args: Vec<String> = std::env::args().collect();

    // Check for help flag
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("PDF Editor - GPUI Edition");
        println!();
        println!("Usage: pdf-editor [OPTIONS] [FILE...]");
        println!();
        println!("Arguments:");
        println!("  [FILE...]  PDF files to open (multiple files open as tabs)");
        println!();
        println!("Options:");
        println!("  -h, --help                 Show this help message");
        println!("  -v, --version              Show version information");
        println!("  --screenshot <path>        Take screenshot and save to path, then exit");
        println!("  --screenshot-delay <ms>    Delay before screenshot (default: 500ms)");
        println!("  --window-id <id>           Capture window by ID");
        println!("  --window-title <text>      Capture window by title (partial match)");
        println!(
            "  --hover <x,y>              Move mouse to window-relative coords before capture"
        );
        println!("  --click <x,y>              Click at window-relative coords before capture");
        println!("  --list-windows             List all capturable windows");
        println!(
            "  --list-elements            List UI elements with positions (for Settings window)"
        );
        println!("  --click-element <id>       Click on a UI element by ID");
        println!("  --focus                    Focus a window (use with --window-title)");
        println!("  --settings                 Open settings window");
        println!("  --dev                      Enable dev mode (dynamic element tracking)");
        println!();
        println!("Keyboard Shortcuts:");
        println!("  Cmd+O              Open file");
        println!("  Cmd++/Cmd+=        Zoom in");
        println!("  Cmd+-              Zoom out");
        println!("  Arrow keys         Navigate pages");
        println!("  PageUp/Down        Navigate pages");
        println!("  Cmd+Alt+Right      Next tab");
        println!("  Cmd+Alt+Left       Previous tab");
        println!("  Cmd+W              Close tab");
        println!("  Cmd+Shift+W        Close window");
        println!();
        println!("Visual Testing:");
        println!("  # Screenshot main window with a PDF");
        println!("  pdf-editor test.pdf --screenshot screenshot.png");
        println!();
        println!("  # Screenshot settings window");
        println!("  pdf-editor --settings --screenshot settings.png");
        std::process::exit(0);
    }

    // Check for version flag
    if args.iter().any(|a| a == "-v" || a == "--version") {
        println!("pdf-editor 0.1.0 (GPUI)");
        std::process::exit(0);
    }

    let mut cli = CliArgs {
        files: Vec::new(),
        screenshot: None,
        screenshot_delay_ms: 500,
        window_id: None,
        window_title: None,
        mouse_action: None,
        list_windows: false,
        open_settings: false,
        focus_window: false,
        list_elements: false,
        click_element: None,
        dev_mode: false,
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--screenshot" => {
                if i + 1 < args.len() {
                    cli.screenshot = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--screenshot-delay" => {
                if i + 1 < args.len() {
                    cli.screenshot_delay_ms = args[i + 1].parse().unwrap_or(500);
                    i += 1;
                }
            }
            "--window-id" => {
                if i + 1 < args.len() {
                    cli.window_id = args[i + 1].parse().ok();
                    i += 1;
                }
            }
            "--window-title" => {
                if i + 1 < args.len() {
                    cli.window_title = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--hover" => {
                if i + 1 < args.len() {
                    if let Some((x, y)) = parse_coords(&args[i + 1]) {
                        cli.mouse_action = Some(MouseAction::MoveTo(x, y));
                    }
                    i += 1;
                }
            }
            "--click" => {
                if i + 1 < args.len() {
                    if let Some((x, y)) = parse_coords(&args[i + 1]) {
                        cli.mouse_action = Some(MouseAction::Click(x, y));
                    }
                    i += 1;
                }
            }
            "--list-windows" => {
                cli.list_windows = true;
            }
            "--list-elements" => {
                cli.list_elements = true;
            }
            "--click-element" => {
                if i + 1 < args.len() {
                    cli.click_element = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--settings" => {
                cli.open_settings = true;
            }
            "--focus" => {
                cli.focus_window = true;
            }
            "--dev" => {
                cli.dev_mode = true;
            }
            arg if !arg.starts_with('-') => {
                cli.files.push(PathBuf::from(arg));
            }
            _ => {}
        }
        i += 1;
    }

    cli
}

/// Simulate mouse action at screen coordinates (cross-platform using enigo)
fn simulate_mouse(action: &MouseAction, window_x: i32, window_y: i32) {
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

/// List all capturable windows (with optional verbose mode showing positions)
fn list_windows() {
    use xcap::Window;

    match Window::all() {
        Ok(windows) => {
            println!("Capturable windows:");
            println!("{:<8} {:<30} {:<20} {}", "ID", "App", "Position", "Title");
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
fn focus_window(window_id: Option<u32>, window_title: Option<&str>) {
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
fn focus_window(window_id: Option<u32>, window_title: Option<&str>) {
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
fn focus_window(window_id: Option<u32>, window_title: Option<&str>) {
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

/// List available UI elements with their positions (requires Settings window to be open with --dev)
#[allow(dead_code)]
fn list_elements() {
    let elements = element_registry::get_all_elements();
    if elements.is_empty() {
        eprintln!("No elements registered. Make sure:");
        eprintln!("  1. The Settings window is open (--settings)");
        eprintln!("  2. Dev mode is enabled (--dev)");
        eprintln!();
        eprintln!("Example: pdf-editor --settings --dev");
        std::process::exit(1);
    }
    println!("UI Elements (dynamically tracked):");
    println!();
    element_registry::print_elements_table(&elements);
    println!();
    println!("Usage: pdf-editor --click-element <id> --window-title Settings");
    std::process::exit(0);
}

/// Click on a UI element by ID (requires Settings window to be open with --dev)
#[allow(dead_code)]
fn click_element(element_id: &str, window_title: Option<&str>) {
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

/// Schedule a screenshot using xcap (cross-platform window capture)
fn schedule_screenshot(
    path: PathBuf,
    delay_ms: u64,
    window_id: Option<u32>,
    window_title: Option<String>,
    mouse_action: Option<MouseAction>,
) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));

        match capture_window(
            window_id,
            window_title.as_deref(),
            mouse_action.as_ref(),
            &path,
        ) {
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
fn capture_window(
    window_id: Option<u32>,
    window_title: Option<&str>,
    mouse_action: Option<&MouseAction>,
    path: &std::path::Path,
) -> Result<(), String> {
    use xcap::Window;

    let windows = Window::all().map_err(|e| format!("Failed to list windows: {}", e))?;

    // Find window by ID first (most precise)
    let window = if let Some(id) = window_id {
        windows.iter().find(|w| w.id().unwrap_or(0) == id)
    } else if let Some(title) = window_title {
        // Find by title (partial match)
        windows
            .iter()
            .find(|w| w.title().unwrap_or_default().contains(title))
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

    let window = window
        .ok_or_else(|| "Window not found. Use --list-windows to see available windows.".to_string())?;

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

    let image = window
        .capture_image()
        .map_err(|e| format!("Failed to capture window: {}", e))?;

    image
        .save(path)
        .map_err(|e| format!("Failed to save screenshot: {}", e))?;

    Ok(())
}

use components::tab_bar::TabId as UiTabId;
use workspace::{load_preferences, TabPreferences};

/// A document tab containing the viewport and sidebar for a single PDF.
struct DocumentTab {
    id: UiTabId,
    path: std::path::PathBuf,
    title: String,
    viewport: Entity<PdfViewport>,
    sidebar: Entity<ThumbnailSidebar>,
    is_dirty: bool,
}

struct PdfEditor {
    tabs: Vec<DocumentTab>,
    active_tab_index: usize,
    focus_handle: FocusHandle,
    preferences: TabPreferences,
}

impl PdfEditor {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            tabs: Vec::new(),
            active_tab_index: 0,
            focus_handle: cx.focus_handle(),
            preferences: load_preferences(),
        }
    }

    /// Create a new document tab for a file path.
    fn create_tab(&mut self, path: PathBuf, cx: &mut Context<Self>) -> usize {
        let viewport = cx.new(PdfViewport::new);
        let sidebar = cx.new(ThumbnailSidebar::new);

        // Set up page change callback from viewport to sidebar
        let sidebar_handle = sidebar.clone();
        viewport.update(cx, |vp, _cx| {
            vp.set_on_page_change(move |page, cx| {
                sidebar_handle.update(cx, |sb, cx| {
                    sb.set_selected_page(page, cx);
                });
            });
        });

        // Set up page select callback from sidebar to viewport
        let viewport_handle = viewport.clone();
        sidebar.update(cx, |sb, _cx| {
            sb.set_on_page_select(move |page, cx| {
                viewport_handle.update(cx, |vp, cx| {
                    vp.go_to_page(page, cx);
                });
            });
        });

        let title = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string());

        let tab = DocumentTab {
            id: UiTabId::new(),
            path,
            title,
            viewport,
            sidebar,
            is_dirty: false,
        };

        self.tabs.push(tab);
        self.tabs.len() - 1
    }

    fn open_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        // Check if file is already open in a tab
        if let Some(idx) = self.tabs.iter().position(|t| t.path == path) {
            self.active_tab_index = idx;
            cx.notify();
            return;
        }

        // Create new tab
        let tab_index = self.create_tab(path.clone(), cx);
        self.active_tab_index = tab_index;

        // Load the PDF in the new tab
        let tab = &self.tabs[tab_index];
        tab.viewport.update(cx, |viewport, cx| {
            if let Err(e) = viewport.load_pdf(path, cx) {
                eprintln!("Error loading PDF: {}", e);
            }
        });

        // Share document with sidebar
        let doc = tab.viewport.read(cx).document();
        tab.sidebar.update(cx, |sidebar, cx| {
            sidebar.set_document(doc, cx);
        });

        cx.notify();
    }

    fn active_tab(&self) -> Option<&DocumentTab> {
        self.tabs.get(self.active_tab_index)
    }

    fn select_tab(&mut self, tab_id: UiTabId, cx: &mut Context<Self>) {
        if let Some(idx) = self.tabs.iter().position(|t| t.id == tab_id) {
            self.active_tab_index = idx;
            cx.notify();
        }
    }

    fn close_tab(&mut self, tab_id: UiTabId, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(idx) = self.tabs.iter().position(|t| t.id == tab_id) {
            self.tabs.remove(idx);

            // Adjust active index
            if self.tabs.is_empty() {
                // Close window if no tabs left
                window.remove_window();
                return;
            }

            if self.active_tab_index >= self.tabs.len() {
                self.active_tab_index = self.tabs.len() - 1;
            }

            cx.notify();
        }
    }

    fn next_tab(&mut self, cx: &mut Context<Self>) {
        if !self.tabs.is_empty() {
            self.active_tab_index = (self.active_tab_index + 1) % self.tabs.len();
            cx.notify();
        }
    }

    fn prev_tab(&mut self, cx: &mut Context<Self>) {
        if !self.tabs.is_empty() {
            if self.active_tab_index == 0 {
                self.active_tab_index = self.tabs.len() - 1;
            } else {
                self.active_tab_index -= 1;
            }
            cx.notify();
        }
    }

    fn show_tab_bar(&self) -> bool {
        // Show tab bar if preference is set or if there are multiple tabs
        self.preferences.show_tab_bar || self.tabs.len() > 1
    }

    fn handle_zoom_in(&mut self, _: &ZoomIn, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.zoom_in(cx);
            });
        }
    }

    fn handle_zoom_out(&mut self, _: &ZoomOut, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.zoom_out(cx);
            });
        }
    }

    fn handle_next_page(&mut self, _: &NextPage, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.next_page(cx);
            });
        }
    }

    fn handle_prev_page(&mut self, _: &PrevPage, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.prev_page(cx);
            });
        }
    }

    fn handle_open(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        let future = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
        });

        cx.spawn_in(
            window,
            async |this: gpui::WeakEntity<PdfEditor>, cx: &mut gpui::AsyncWindowContext| {
                if let Ok(Ok(Some(paths))) = future.await {
                    if let Some(path) = paths.into_iter().next() {
                        let _ = cx.update(|_window, cx| {
                            this.update(cx, |editor: &mut PdfEditor, cx| {
                                editor.open_file(path, cx);
                            })
                            .ok()
                        });
                    }
                }
            },
        )
        .detach();
    }

    fn handle_close_window(
        &mut self,
        _: &CloseWindow,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        window.remove_window();
    }

    fn handle_next_tab(&mut self, _: &NextTab, _window: &mut Window, cx: &mut Context<Self>) {
        self.next_tab(cx);
    }

    fn handle_prev_tab(&mut self, _: &PrevTab, _window: &mut Window, cx: &mut Context<Self>) {
        self.prev_tab(cx);
    }

    fn handle_close_tab(&mut self, _: &CloseTab, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let tab_id = tab.id;
            self.close_tab(tab_id, window, cx);
        }
    }

    /// Render the tab bar component.
    fn render_tab_bar(&self, theme: &Theme, cx: &Context<Self>) -> impl IntoElement {
        let entity = cx.entity().downgrade();

        let tabs_for_render: Vec<_> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(idx, doc_tab)| {
                let is_active = idx == self.active_tab_index;
                let title = doc_tab.title.clone();
                let is_dirty = doc_tab.is_dirty;
                let tab_id = doc_tab.id;

                let entity_for_select = entity.clone();
                let entity_for_close = entity.clone();

                div()
                    .id(gpui::SharedString::from(format!("tab-{}", tab_id)))
                    .h(px(26.0))
                    .px(ui::sizes::PADDING_MD)
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(ui::sizes::GAP_SM)
                    .rounded(ui::sizes::RADIUS_SM)
                    .cursor_pointer()
                    .text_sm()
                    .when(is_active, |d| d.bg(theme.element_selected))
                    .when(!is_active, {
                        let hover_bg = theme.element_hover;
                        move |d| d.hover(move |s| s.bg(hover_bg))
                    })
                    .on_click(move |_, _, cx| {
                        if let Some(editor) = entity_for_select.upgrade() {
                            editor.update(cx, |editor, cx| {
                                editor.select_tab(tab_id, cx);
                            });
                        }
                    })
                    // Tab title
                    .child(
                        div()
                            .max_w(px(150.0))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(title),
                    )
                    // Dirty indicator
                    .when(is_dirty, |d| {
                        d.child(
                            div()
                                .text_xs()
                                .text_color(theme.text_muted)
                                .child("\u{2022}"),
                        )
                    })
                    // Close button
                    .child({
                        let hover_bg = theme.element_hover;
                        div()
                            .id(gpui::SharedString::from(format!("tab-close-{}", tab_id)))
                            .w(px(16.0))
                            .h(px(16.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(ui::sizes::RADIUS_SM)
                            .text_xs()
                            .text_color(theme.text_muted)
                            .hover(move |s| s.bg(hover_bg))
                            .on_click(move |_, window, cx| {
                                if let Some(editor) = entity_for_close.upgrade() {
                                    editor.update(cx, |editor, cx| {
                                        editor.close_tab(tab_id, window, cx);
                                    });
                                }
                            })
                            .child("\u{2715}") // X symbol
                    })
            })
            .collect();

        div()
            .id("tab-bar")
            .h(px(32.0))
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .bg(theme.surface)
            .border_b_1()
            .border_color(theme.border)
            .px(ui::sizes::PADDING_SM)
            .gap(px(2.0))
            .children(tabs_for_render)
    }
}

impl Focusable for PdfEditor {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PdfEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = current_theme(window, cx);

        // Get viewport info for status bar
        let (page_count, current_page, zoom_level) = if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.read(cx);
            (
                viewport.page_count(),
                viewport.current_page_display(),
                viewport.zoom_level,
            )
        } else {
            (0, 0, 100)
        };

        let status_text = if page_count > 0 {
            format!(
                "Page {} of {} \u{2022} {}%",
                current_page, page_count, zoom_level
            )
        } else {
            "No document".to_string()
        };

        let show_tab_bar = self.show_tab_bar();

        div()
            .id("pdf-editor")
            .key_context("PdfEditor")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::handle_zoom_in))
            .on_action(cx.listener(Self::handle_zoom_out))
            .on_action(cx.listener(Self::handle_next_page))
            .on_action(cx.listener(Self::handle_prev_page))
            .on_action(cx.listener(Self::handle_open))
            .on_action(cx.listener(Self::handle_close_window))
            .on_action(cx.listener(Self::handle_next_tab))
            .on_action(cx.listener(Self::handle_prev_tab))
            .on_action(cx.listener(Self::handle_close_tab))
            .flex()
            .flex_col()
            .bg(theme.surface)
            .text_color(theme.text)
            .size_full()
            // Title bar with centered title
            .child(ui::title_bar("PDF Editor", theme.text, theme.border))
            // Tab bar (conditional)
            .when(show_tab_bar, |d| d.child(self.render_tab_bar(&theme, cx)))
            // Main content: sidebar + viewport
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .overflow_hidden()
                    .when_some(self.active_tab(), |d, tab| {
                        d.child(tab.sidebar.clone()).child(tab.viewport.clone())
                    })
                    .when(self.active_tab().is_none(), |d| {
                        d.child(
                            div()
                                .flex_1()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(theme.text_muted)
                                .child("Open a PDF file to get started (Cmd+O)"),
                        )
                    }),
            )
            // Status bar at bottom
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_center()
                    .h(px(24.0))
                    .bg(theme.elevated_surface)
                    .border_t_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child(status_text),
                    ),
            )
    }
}

fn set_menus(cx: &mut App) {
    cx.set_menus(vec![
        Menu {
            name: "PDF Editor".into(),
            items: vec![
                MenuItem::action("About PDF Editor", About),
                MenuItem::separator(),
                MenuItem::action("Settings...", settings::OpenSettings),
                MenuItem::separator(),
                MenuItem::action("Quit PDF Editor", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: vec![MenuItem::action("Open...", Open)],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Zoom In", ZoomIn),
                MenuItem::action("Zoom Out", ZoomOut),
                MenuItem::separator(),
                MenuItem::action("Next Page", NextPage),
                MenuItem::action("Previous Page", PrevPage),
            ],
        },
    ]);
}

fn main() {
    // Parse CLI args before starting the app
    let cli = parse_args();

    // Enable dev mode if requested (for dynamic element tracking)
    if cli.dev_mode {
        element_registry::set_dev_mode(true);
    }

    // List windows and exit if requested
    if cli.list_windows {
        list_windows();
    }

    // These commands need a running window with dev mode, so don't exit early
    // They will be checked after the app runs (handled differently)

    // Focus window and exit if requested (this one can run without app)
    if cli.focus_window {
        focus_window(cli.window_id, cli.window_title.as_deref());
    }

    // Click element - needs window to be running, can't do from cold CLI
    if cli.click_element.is_some() && !cli.dev_mode {
        eprintln!("--click-element requires --dev mode with Settings window open");
        std::process::exit(1);
    }

    // Schedule screenshot if requested
    if let Some(screenshot_path) = cli.screenshot.clone() {
        schedule_screenshot(
            screenshot_path,
            cli.screenshot_delay_ms,
            cli.window_id,
            cli.window_title.clone(),
            cli.mouse_action.clone(),
        );
    }

    let initial_files = cli.files;
    let open_settings = cli.open_settings;

    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);

        // Bind keyboard shortcuts - global
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

        // Bind keyboard shortcuts - editor context
        cx.bind_keys([
            KeyBinding::new("cmd-o", Open, Some("PdfEditor")),
            KeyBinding::new("cmd-w", CloseTab, Some("PdfEditor")),
            KeyBinding::new("cmd-shift-w", CloseWindow, Some("PdfEditor")),
            KeyBinding::new("cmd-=", ZoomIn, Some("PdfEditor")),
            KeyBinding::new("cmd-+", ZoomIn, Some("PdfEditor")),
            KeyBinding::new("cmd--", ZoomOut, Some("PdfEditor")),
            KeyBinding::new("right", NextPage, Some("PdfEditor")),
            KeyBinding::new("left", PrevPage, Some("PdfEditor")),
            KeyBinding::new("pagedown", NextPage, Some("PdfEditor")),
            KeyBinding::new("pageup", PrevPage, Some("PdfEditor")),
            KeyBinding::new("down", NextPage, Some("PdfEditor")),
            KeyBinding::new("up", PrevPage, Some("PdfEditor")),
            // Tab navigation
            KeyBinding::new("ctrl-tab", NextTab, Some("PdfEditor")),
            KeyBinding::new("ctrl-shift-tab", PrevTab, Some("PdfEditor")),
            KeyBinding::new("cmd-shift-]", NextTab, Some("PdfEditor")),
            KeyBinding::new("cmd-shift-[", PrevTab, Some("PdfEditor")),
            KeyBinding::new("cmd-alt-right", NextTab, Some("PdfEditor")),
            KeyBinding::new("cmd-alt-left", PrevTab, Some("PdfEditor")),
        ]);

        // Initialize default appearance mode and theme settings
        cx.set_global(AppearanceMode::default());
        cx.set_global(ThemeSettings::default());

        // Check for theme updates in background (once per 24 hours)
        theme_updater::spawn_update_check();

        // Global actions
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_action(|_: &About, _cx| {
            println!("PDF Editor - GPUI Edition");
        });

        // Settings action
        settings::register_bindings(cx);
        cx.on_action(|_: &settings::OpenSettings, cx| {
            settings::open_settings_window(cx);
        });

        set_menus(cx);

        // Open settings window if requested (for screenshot mode)
        if open_settings {
            settings::open_settings_window(cx);
        }

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("PDF Editor".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(12.0), px(9.0))),
                }),
                focus: true,
                show: true,
                is_movable: true,
                window_min_size: Some(size(px(600.0), px(400.0))),
                ..Default::default()
            },
            |window, cx| {
                // Observe system appearance changes to trigger re-render
                window
                    .observe_window_appearance(|window, _cx| {
                        window.refresh();
                    })
                    .detach();

                cx.new(|cx| {
                    let mut editor = PdfEditor::new(cx);

                    // Open initial files if provided via CLI (each as a separate tab)
                    for path in initial_files {
                        if path.exists() {
                            editor.open_file(path, cx);
                        } else {
                            eprintln!("File not found: {:?}", path);
                        }
                    }

                    editor
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
