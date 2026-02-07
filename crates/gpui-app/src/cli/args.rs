//! CLI argument parsing.

use std::path::PathBuf;

/// Mouse action to perform before screenshot
#[derive(Clone)]
pub enum MouseAction {
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
pub struct CliArgs {
    pub files: Vec<PathBuf>,
    pub screenshot: Option<PathBuf>,
    pub screenshot_delay_ms: u64,
    pub window_id: Option<u32>,
    pub window_title: Option<String>,
    pub mouse_action: Option<MouseAction>,
    pub list_windows: bool,
    pub open_settings: bool,
    pub focus_window: bool,
    pub list_elements: bool,
    pub click_element: Option<String>,
    pub dev_mode: bool,
    pub gui_mode: bool,
}

/// Parse command line arguments
pub fn parse_args() -> CliArgs {
    let args: Vec<String> = std::env::args().collect();

    // Check for help flag
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("ButterPaper - GPUI Edition");
        println!();
        println!("Usage: butterpaper [OPTIONS] [FILE...]");
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
        println!("  --gui                      Force GUI mode (skip screenshot/headless commands)");
        println!();
        println!("Keyboard Shortcuts:");
        println!("  Cmd+O              Open file");
        println!("  Cmd++/Cmd+=        Zoom in");
        println!("  Cmd+-              Zoom out");
        println!("  Cmd+0              Reset zoom to 100%");
        println!("  Cmd+8              Fit width");
        println!("  Cmd+9              Fit page");
        println!("  Arrow keys         Navigate pages");
        println!("  PageUp/Down        Navigate pages");
        println!("  Home/End           First/last page");
        println!("  Cmd+Alt+Right      Next tab");
        println!("  Cmd+Alt+Left       Previous tab");
        println!("  Cmd+W              Close tab");
        println!("  Cmd+Shift+W        Close window");
        println!();
        println!("Visual Testing:");
        println!("  # Screenshot main window with a PDF");
        println!("  butterpaper test.pdf --screenshot screenshot.png");
        println!();
        println!("  # Screenshot settings window");
        println!("  butterpaper --settings --screenshot settings.png");
        std::process::exit(0);
    }

    // Check for version flag
    if args.iter().any(|a| a == "-v" || a == "--version") {
        println!("butterpaper 0.1.0 (GPUI)");
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
        gui_mode: false,
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
            "--gui" => {
                cli.gui_mode = true;
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
