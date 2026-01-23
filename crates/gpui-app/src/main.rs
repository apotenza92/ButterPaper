mod cache;
mod cli;
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

use cli::{parse_args, MouseAction};
use components::{icon, tooltip_builder, Icon};
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

/// List all capturable windows (with optional verbose mode showing positions)
fn list_windows() {
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
        cli::simulate_mouse(action, win_x, win_y);
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
        // Use WeakEntity to avoid dangling references if tab is closed
        let sidebar_weak = sidebar.downgrade();
        viewport.update(cx, |vp, _cx| {
            vp.set_on_page_change(move |page, cx| {
                if let Some(sidebar_handle) = sidebar_weak.upgrade() {
                    sidebar_handle.update(cx, |sb, cx| {
                        sb.set_selected_page(page, cx);
                    });
                }
            });
        });

        // Set up page select callback from sidebar to viewport
        // Use WeakEntity to avoid dangling references if tab is closed
        let viewport_weak = viewport.downgrade();
        sidebar.update(cx, |sb, _cx| {
            sb.set_on_page_select(move |page, cx| {
                if let Some(viewport_handle) = viewport_weak.upgrade() {
                    viewport_handle.update(cx, |vp, cx| {
                        vp.go_to_page(page, cx);
                    });
                }
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

    /// Render individual tab items (for use in the tab bar header).
    /// Styled to match Zed's tab aesthetic:
    /// - Active tab: elevated background, side borders, NO bottom border (flows into content)
    /// - Inactive tabs: surface background, bottom border only
    /// - First tab never has left border (nav buttons provide the left edge)
    fn render_tab_items(
        &self,
        theme: &Theme,
        cx: &Context<Self>,
    ) -> Vec<impl IntoElement + use<'_>> {
        let entity = cx.entity().downgrade();

        self.tabs
            .iter()
            .enumerate()
            .map(|(idx, doc_tab)| {
                let is_active = idx == self.active_tab_index;
                let is_first = idx == 0;
                let title = doc_tab.title.clone();
                let is_dirty = doc_tab.is_dirty;
                let tab_id = doc_tab.id;

                let entity_for_select = entity.clone();
                let entity_for_close = entity.clone();

                // Zed-style tabs:
                // - Active: elevated bg, right border (left border only if not first), NO bottom border
                // - Inactive: surface bg, bottom border only
                // - First tab: no left border (nav buttons provide the edge)
                div()
                    .id(gpui::SharedString::from(format!("tab-{}", tab_id)))
                    .group("tab")
                    .h_full()
                    .px(px(12.0)) // Consistent horizontal padding for text
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .cursor_pointer()
                    .text_sm()
                    // Active tab: elevated background, side borders, no bottom border
                    .when(is_active, {
                        let bg = theme.elevated_surface;
                        let text = theme.text;
                        let border = theme.border;
                        move |d| {
                            d.bg(bg)
                                .text_color(text)
                                .border_r_1() // Right border always
                                .border_color(border)
                                // Left border only if not first tab (nav buttons provide left edge)
                                .when(!is_first, |d| d.border_l_1())
                            // No bottom border - tab flows into content
                        }
                    })
                    // Inactive tab: bottom border only, muted text
                    .when(!is_active, {
                        let text_muted = theme.text_muted;
                        let text = theme.text;
                        let hover_bg = theme.element_hover;
                        let border = theme.border;
                        move |d| {
                            d.text_color(text_muted)
                                .border_b_1()
                                .border_color(border)
                                .hover(move |s| s.text_color(text).bg(hover_bg))
                        }
                    })
                    .on_click(move |_, _, cx| {
                        if let Some(editor) = entity_for_select.upgrade() {
                            editor.update(cx, |editor, cx| {
                                editor.select_tab(tab_id, cx);
                            });
                        }
                    })
                    // Tab title with truncation
                    .child(
                        div()
                            .max_w(px(150.0))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(title),
                    )
                    .tooltip(tooltip_builder(
                        doc_tab.title.clone(),
                        theme.surface,
                        theme.border,
                    ))
                    // Dirty indicator (always visible when dirty)
                    .when(is_dirty, {
                        let text_muted = theme.text_muted;
                        move |d| d.child(icon(Icon::Dirty, 12.0, text_muted))
                    })
                    // Close button (hidden by default, visible on hover)
                    .child({
                        let hover_bg = theme.element_hover;
                        let text_muted = theme.text_muted;
                        div()
                            .id(gpui::SharedString::from(format!("tab-close-{}", tab_id)))
                            .w(px(16.0))
                            .h(px(16.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(ui::sizes::RADIUS_SM)
                            .text_xs()
                            .text_color(text_muted)
                            // Always visible
                            .hover(move |s| s.bg(hover_bg))
                            .on_click(move |_, window, cx| {
                                if let Some(editor) = entity_for_close.upgrade() {
                                    editor.update(cx, |editor, cx| {
                                        editor.close_tab(tab_id, window, cx);
                                    });
                                }
                            })
                            .child(icon(Icon::Close, 12.0, text_muted))
                    })
            })
            .collect()
    }

    /// Render navigation buttons (← →) for page navigation.
    /// Has bottom border only; left edge is provided by sidebar's right border.
    fn render_nav_buttons(
        &self,
        theme: &Theme,
        page_count: u16,
        current_page: u16,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let entity = cx.entity().downgrade();
        let can_go_back = current_page > 1;
        let can_go_forward = current_page < page_count;

        let text_enabled = theme.text;
        let text_disabled = theme.text_muted;
        let hover_bg = theme.element_hover;
        let border = theme.border;

        let entity_for_back = entity.clone();
        let entity_for_forward = entity;

        div()
            .id("nav-buttons")
            .h_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(4.0))
            .px(px(8.0))  // Symmetric padding
            .border_b_1() // Bottom border only (left edge from sidebar's right border)
            .border_color(border)
            // Back button (←)
            .child(
                div()
                    .id("nav-back")
                    .w(px(24.0))
                    .h(px(24.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(ui::sizes::RADIUS_SM)
                    .text_sm()
                    .when(can_go_back, move |d| {
                        d.cursor_pointer()
                            .text_color(text_enabled)
                            .hover(move |s| s.bg(hover_bg))
                            .on_click(move |_, _, cx| {
                                if let Some(editor) = entity_for_back.upgrade() {
                                    editor.update(cx, |editor, cx| {
                                        if let Some(tab) = editor.active_tab() {
                                            let viewport = tab.viewport.clone();
                                            viewport.update(cx, |vp, cx| {
                                                vp.prev_page(cx);
                                            });
                                        }
                                    });
                                }
                            })
                    })
                    .when(!can_go_back, move |d| d.text_color(text_disabled))
                    .child(Icon::ArrowLeft.as_str())
            )
            // Forward button (→)
            .child(
                div()
                    .id("nav-forward")
                    .w(px(24.0))
                    .h(px(24.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(ui::sizes::RADIUS_SM)
                    .text_sm()
                    .when(can_go_forward, move |d| {
                        d.cursor_pointer()
                            .text_color(text_enabled)
                            .hover(move |s| s.bg(hover_bg))
                            .on_click(move |_, _, cx| {
                                if let Some(editor) = entity_for_forward.upgrade() {
                                    editor.update(cx, |editor, cx| {
                                        if let Some(tab) = editor.active_tab() {
                                            let viewport = tab.viewport.clone();
                                            viewport.update(cx, |vp, cx| {
                                                vp.next_page(cx);
                                            });
                                        }
                                    });
                                }
                            })
                    })
                    .when(!can_go_forward, move |d| d.text_color(text_disabled))
                    .child(Icon::ArrowRight.as_str())
            )
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

        // Layout: Titlebar (32px) -> Content (sidebar | main)
        // CRITICAL: Titlebar MUST remain empty except for traffic lights
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
            .flex_col() // Vertical layout: titlebar -> content
            .bg(theme.surface)
            .text_color(theme.text)
            .size_full()
            // Title bar (32px) - EMPTY, only traffic lights
            .child(ui::title_bar("PDF Editor", theme.text, theme.border))
            // Main content below titlebar: sidebar | right column
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .overflow_hidden()
                    // Left: Sidebar
                    .when_some(self.active_tab(), |d, tab| d.child(tab.sidebar.clone()))
                    .when(self.active_tab().is_none(), |d| {
                        // Empty sidebar placeholder when no document
                        // No right border - content column provides left border for clean corners
                        d.child(
                            div()
                                .w(px(160.0))
                                .h_full()
                                .bg(theme.surface)
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .h(ui::sizes::TAB_BAR_HEIGHT)
                                        .w_full()
                                        .flex()
                                        .items_center()
                                        .px(ui::sizes::PADDING_SM)
                                        .border_b_1()
                                        .border_color(theme.border)
                                        .child(div().text_xs().text_color(theme.text_muted).child("Pages")),
                                ),
                        )
                    })
                    // Right: Content column (tab bar + viewport + status bar)
                    // Left border separates from sidebar (sidebar has no right border for clean corners)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .overflow_hidden()
                            .border_l_1()
                            .border_color(theme.border)
                            // Tab bar - Zed style: individual elements have bottom borders
                            .child(
                                div()
                                    .h(ui::sizes::TAB_BAR_HEIGHT)
                                    .w_full()
                                    .flex()
                                    .flex_row()
                                    .bg(theme.surface)
                                    // No left padding - nav buttons have their own padding with left border
                                    // Navigation buttons (← →) - with left and bottom borders
                                    .child(self.render_nav_buttons(&theme, page_count, current_page, cx))
                                    // Tabs area
                                    .when(show_tab_bar, |d| {
                                        d.children(self.render_tab_items(&theme, cx))
                                    })
                                    .when(!show_tab_bar && self.active_tab().is_some(), {
                                        // Show document title when single tab
                                        let title = self
                                            .active_tab()
                                            .map(|t| t.title.clone())
                                            .unwrap_or_default();
                                        let border = theme.border;
                                        move |d| {
                                            d.child(
                                                div()
                                                    .h_full()
                                                    .flex()
                                                    .items_center()
                                                    .border_b_1()
                                                    .border_color(border)
                                                    .text_sm()
                                                    .text_color(theme.text)
                                                    .child(title),
                                            )
                                        }
                                    })
                                    // Fill remaining space with border line
                                    .child({
                                        let border = theme.border;
                                        div().flex_1().h_full().border_b_1().border_color(border)
                                    }),
                            )
                            // Viewport
                            .child(
                                div()
                                    .flex_1()
                                    .overflow_hidden()
                                    .bg(theme.elevated_surface)
                                    .when_some(self.active_tab(), |d, tab| d.child(tab.viewport.clone()))
                                    .when(self.active_tab().is_none(), |d| {
                                        d.child(
                                            div()
                                                .size_full()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .text_color(theme.text_muted)
                                                .child("Open a PDF file to get started (Cmd+O)"),
                                        )
                                    }),
                            )
                            // Status bar
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
                            ),
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

    // Schedule screenshot if requested (skip in gui mode)
    if !cli.gui_mode {
        if let Some(screenshot_path) = cli.screenshot.clone() {
            schedule_screenshot(
                screenshot_path,
                cli.screenshot_delay_ms,
                cli.window_id,
                cli.window_title.clone(),
                cli.mouse_action.clone(),
            );
        }
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
