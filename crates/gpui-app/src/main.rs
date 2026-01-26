mod app;
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
mod window;
mod workspace;

pub use element_registry::{ElementInfo, ElementType};
pub use theme::{current_theme, AppearanceMode, Theme, ThemeSettings};

use gpui::{
    actions, point, prelude::*, px, size, App, Application, Bounds, Focusable, KeyBinding,
    TitlebarOptions, WindowBounds, WindowOptions,
};
use butterpaper_render::PdfDocument;

use app::{set_menus, PdfEditor};
use cli::parse_args;
use window::{focus_window, list_windows, schedule_screenshot};

actions!(
    butterpaper,
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

fn main() {
    // Pre-initialize Pdfium library early (shared instance for all documents)
    // This moves the initialization cost to startup rather than first PDF open
    if let Err(e) = PdfDocument::init_pdfium_global() {
        eprintln!("Warning: Failed to pre-initialize PDFium: {}", e);
        // Continue anyway - it will retry when opening a PDF
    }

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
            println!("ButterPaper - GPUI Edition");
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
                    title: Some("ButterPaper".into()),
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

                let editor = cx.new(|cx| {
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
                });

                // Focus the editor immediately so keyboard shortcuts work right away
                editor.focus_handle(cx).focus(window);

                editor
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
