mod app;
mod app_update;
mod assets;
mod benchmark;
mod cache;
mod cli;
mod components;
mod element_registry;
mod icons;
#[cfg(target_os = "macos")]
mod macos;
mod process_memory;
mod preview_cache;
mod settings;
mod sidebar;
mod styles;
mod theme;
mod theme_updater;
mod ui;
mod ui_preferences;
mod viewport;
mod window;
mod workspace;

pub use element_registry::{ElementInfo, ElementType};
pub use theme::{current_theme, AppearanceMode, Theme, ThemeSettings};

use butterpaper_render::PdfDocument;
use gpui::{
    actions, point, prelude::*, px, size, App, Application, Bounds, Focusable, Global, KeyBinding,
    TitlebarOptions, WindowBounds, WindowOptions,
};
use std::path::PathBuf;

use app::{set_menus, PdfEditor};
use assets::Assets;
use benchmark::BenchmarkConfig;
use cli::parse_args;
use ui_preferences::load_ui_preferences;
use window::{focus_window, list_windows, schedule_screenshot};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThumbnailClusterWidthPx(pub f32);

impl Default for ThumbnailClusterWidthPx {
    fn default() -> Self {
        Self(ui_preferences::THUMBNAIL_CLUSTER_WIDTH_DEFAULT_PX)
    }
}

impl Global for ThumbnailClusterWidthPx {}

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
        FirstPage,
        LastPage,
        FitPage,
        FitWidth,
        ResetZoom,
        CloseWindow,
        NextTab,
        PrevTab,
        CloseTab
    ]
);

fn open_editor_window(cx: &mut App, initial_files: Vec<PathBuf>) -> gpui::WindowHandle<PdfEditor> {
    let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);

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
        move |window, cx| {
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
    .unwrap()
}

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
    let benchmark_config = if cli.benchmark_scroll {
        Some(BenchmarkConfig {
            file: cli.benchmark_file.clone().unwrap_or_else(|| {
                PathBuf::from("/Users/alex/code/ButterPaper/samples/All Slides + Cases.pdf")
            }),
            seconds: cli.benchmark_seconds,
            output: cli
                .benchmark_output
                .clone()
                .unwrap_or_else(|| PathBuf::from("/tmp/butterpaper-benchmark.json")),
        })
    } else {
        None
    };

    let app = Application::new().with_assets(Assets);
    app.on_reopen(|cx| {
        if cx.windows().is_empty() {
            let _ = open_editor_window(cx, Vec::new());
            cx.activate(true);
        }
    });

    app.run(move |cx: &mut App| {
        #[cfg(target_os = "macos")]
        macos::set_app_icon();

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
            KeyBinding::new("cmd-0", ResetZoom, Some("PdfEditor")),
            KeyBinding::new("ctrl-0", ResetZoom, Some("PdfEditor")),
            KeyBinding::new("cmd-8", FitWidth, Some("PdfEditor")),
            KeyBinding::new("ctrl-8", FitWidth, Some("PdfEditor")),
            KeyBinding::new("cmd-9", FitPage, Some("PdfEditor")),
            KeyBinding::new("ctrl-9", FitPage, Some("PdfEditor")),
            KeyBinding::new("right", NextPage, Some("PdfEditor")),
            KeyBinding::new("left", PrevPage, Some("PdfEditor")),
            KeyBinding::new("pagedown", NextPage, Some("PdfEditor")),
            KeyBinding::new("pageup", PrevPage, Some("PdfEditor")),
            KeyBinding::new("down", NextPage, Some("PdfEditor")),
            KeyBinding::new("up", PrevPage, Some("PdfEditor")),
            KeyBinding::new("home", FirstPage, Some("PdfEditor")),
            KeyBinding::new("end", LastPage, Some("PdfEditor")),
            // Tab navigation
            KeyBinding::new("ctrl-tab", NextTab, Some("PdfEditor")),
            KeyBinding::new("ctrl-shift-tab", PrevTab, Some("PdfEditor")),
            KeyBinding::new("cmd-shift-]", NextTab, Some("PdfEditor")),
            KeyBinding::new("cmd-shift-[", PrevTab, Some("PdfEditor")),
            KeyBinding::new("cmd-alt-right", NextTab, Some("PdfEditor")),
            KeyBinding::new("cmd-alt-left", PrevTab, Some("PdfEditor")),
        ]);

        // Initialize persisted appearance/theme preferences.
        let ui_preferences = load_ui_preferences();
        cx.set_global(ui_preferences.appearance_mode);
        cx.set_global(ui_preferences.theme_settings);
        cx.set_global(ThumbnailClusterWidthPx(ui_preferences.thumbnail_cluster_width_px));
        #[cfg(target_os = "macos")]
        macos::set_app_appearance(ui_preferences.appearance_mode);

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

        let editor_window = open_editor_window(cx, initial_files.clone());

        if let Some(config) = benchmark_config.clone() {
            if let Err(err) = benchmark::start(editor_window, config, cx) {
                eprintln!("benchmark: failed to start benchmark runner: {err}");
                std::process::exit(2);
            }
        }

        cx.activate(true);
    });
}
