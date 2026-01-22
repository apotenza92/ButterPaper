//! PDF Editor - egui-based UI
//!
//! New entry point using eframe for UI chrome with system theme support.

use eframe::egui;
use pdf_editor_render::PdfDocument;
use std::collections::HashMap;
use std::path::PathBuf;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("PDF Editor"),
        ..Default::default()
    };

    eframe::run_native(
        "PDF Editor",
        options,
        Box::new(|cc| {
            configure_visuals(&cc.egui_ctx);
            Ok(Box::new(PdfEditorApp::new(cc)))
        }),
    )
}

fn configure_visuals(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    ctx.set_style(style);
}

/// Thumbnail texture for a page
struct ThumbnailTexture {
    handle: egui::TextureHandle,
}

/// Viewport texture cache key
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ViewportCacheKey {
    page: usize,
    zoom_percent: u32,
}

/// Viewport texture with dimensions
struct ViewportTexture {
    handle: egui::TextureHandle,
    width: f32,
    height: f32,
}

struct PdfEditorApp {
    // Document state
    document: Option<PdfDocument>,
    file_path: Option<PathBuf>,

    // Navigation
    current_page: usize,
    total_pages: usize,

    // View
    zoom_level: f32,
    current_tool: Tool,

    // Thumbnail cache: page_index -> texture
    thumbnails: HashMap<usize, ThumbnailTexture>,

    // Viewport cache: (page, zoom) -> texture
    viewport_cache: HashMap<ViewportCacheKey, ViewportTexture>,

    // Viewport pan offset (for Hand tool dragging)
    viewport_offset: egui::Vec2,

    // UI state
    sidebar_scroll_to_current: bool,

    // Dialogs
    error_dialog: Option<ErrorDialogState>,
    calibration_dialog: Option<CalibrationDialogState>,
    search_bar: SearchBarState,
}

/// Error dialog state
struct ErrorDialogState {
    severity: ErrorSeverity,
    title: String,
    message: String,
}

#[derive(Clone, Copy, PartialEq)]
enum ErrorSeverity {
    Error,
    Warning,
    Info,
}

impl ErrorSeverity {
    fn icon(&self) -> &'static str {
        match self {
            ErrorSeverity::Error => "❌",
            ErrorSeverity::Warning => "⚠️",
            ErrorSeverity::Info => "ℹ️",
        }
    }

    fn title(&self) -> &'static str {
        match self {
            ErrorSeverity::Error => "Error",
            ErrorSeverity::Warning => "Warning",
            ErrorSeverity::Info => "Notice",
        }
    }
}

/// Calibration dialog state
struct CalibrationDialogState {
    distance_input: String,
    selected_unit_index: usize,
    page_distance: f32,
}

const CALIBRATION_UNITS: [&str; 6] = ["m", "ft", "cm", "mm", "in", "yd"];

impl CalibrationDialogState {
    fn new(page_distance: f32) -> Self {
        Self {
            distance_input: String::new(),
            selected_unit_index: 0,
            page_distance,
        }
    }

    fn selected_unit(&self) -> &'static str {
        CALIBRATION_UNITS[self.selected_unit_index]
    }

    fn cycle_unit(&mut self) {
        self.selected_unit_index = (self.selected_unit_index + 1) % CALIBRATION_UNITS.len();
    }

    fn parse_distance(&self) -> Option<f32> {
        self.distance_input.parse::<f32>().ok().filter(|&v| v > 0.0)
    }
}

/// Search bar state
#[derive(Default)]
struct SearchBarState {
    visible: bool,
    query: String,
    current_match: usize,
    total_matches: usize,
    case_sensitive: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Select,
    Hand,
    Text,
    Highlight,
    Comment,
    Measure,
    Freedraw,
}

impl Default for Tool {
    fn default() -> Self {
        Tool::Select
    }
}

impl PdfEditorApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            document: None,
            file_path: None,
            current_page: 0,
            total_pages: 0,
            zoom_level: 100.0,
            current_tool: Tool::default(),
            thumbnails: HashMap::new(),
            viewport_cache: HashMap::new(),
            viewport_offset: egui::Vec2::ZERO,
            sidebar_scroll_to_current: false,
            error_dialog: None,
            calibration_dialog: None,
            search_bar: SearchBarState::default(),
        }
    }

    fn show_error(&mut self, severity: ErrorSeverity, message: impl Into<String>) {
        self.error_dialog = Some(ErrorDialogState {
            severity,
            title: severity.title().to_string(),
            message: message.into(),
        });
    }

    /// Open a PDF file using the file picker
    fn open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .pick_file()
        {
            self.load_pdf(path);
        }
    }

    /// Load a PDF from path
    fn load_pdf(&mut self, path: PathBuf) {
        match PdfDocument::open(&path) {
            Ok(pdf) => {
                self.total_pages = pdf.page_count() as usize;
                self.current_page = 0;
                self.file_path = Some(path);
                self.document = Some(pdf);
                self.thumbnails.clear();
                self.viewport_cache.clear();
                self.viewport_offset = egui::Vec2::ZERO;
                self.sidebar_scroll_to_current = true;
            }
            Err(e) => {
                self.show_error(ErrorSeverity::Error, format!("Failed to open PDF: {}", e));
            }
        }
    }

    /// Render a thumbnail for a page and cache it
    fn render_thumbnail(&mut self, ctx: &egui::Context, page_index: usize) {
        if self.thumbnails.contains_key(&page_index) {
            return;
        }

        let Some(pdf) = &self.document else { return };

        const THUMB_MAX_WIDTH: u32 = 100;
        const THUMB_MAX_HEIGHT: u32 = 140;

        match pdf.render_page_scaled(page_index as u16, THUMB_MAX_WIDTH, THUMB_MAX_HEIGHT) {
            Ok((rgba_data, width, height)) => {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [width as usize, height as usize],
                    &rgba_data,
                );
                let handle = ctx.load_texture(
                    format!("thumb_{}", page_index),
                    image,
                    egui::TextureOptions::LINEAR,
                );
                self.thumbnails.insert(page_index, ThumbnailTexture { handle });
            }
            Err(e) => {
                eprintln!("Failed to render thumbnail for page {}: {}", page_index, e);
            }
        }
    }

    /// Navigate to a specific page
    fn go_to_page(&mut self, page: usize) {
        if page < self.total_pages && page != self.current_page {
            self.current_page = page;
            self.viewport_offset = egui::Vec2::ZERO;
            self.sidebar_scroll_to_current = true;
        }
    }
}

impl eframe::App for PdfEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_keyboard_shortcuts(ctx);
        self.draw_toolbar(ctx);
        self.draw_search_bar(ctx);
        self.draw_sidebar(ctx);
        self.draw_viewport(ctx);
        self.draw_error_dialog(ctx);
        self.draw_calibration_dialog(ctx);
    }
}

impl PdfEditorApp {
    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let modifiers = ctx.input(|i| i.modifiers);
        let cmd_or_ctrl = modifiers.command || modifiers.ctrl;

        ctx.input(|i| {
            // Cmd/Ctrl+F: Open search
            if cmd_or_ctrl && i.key_pressed(egui::Key::F) {
                self.search_bar.visible = true;
            }

            // Escape: Close dialogs/search
            if i.key_pressed(egui::Key::Escape) {
                if self.error_dialog.is_some() {
                    self.error_dialog = None;
                } else if self.calibration_dialog.is_some() {
                    self.calibration_dialog = None;
                } else if self.search_bar.visible {
                    self.search_bar.visible = false;
                    self.search_bar.query.clear();
                }
            }

            // Enter in search: go to next match
            if self.search_bar.visible
                && i.key_pressed(egui::Key::Enter)
                && self.search_bar.total_matches > 0
            {
                if modifiers.shift {
                    if self.search_bar.current_match > 1 {
                        self.search_bar.current_match -= 1;
                    }
                } else if self.search_bar.current_match < self.search_bar.total_matches {
                    self.search_bar.current_match += 1;
                }
            }
        });
    }
}

impl PdfEditorApp {
    fn draw_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(8.0);

                // File menu
                if ui.button("📂 Open").clicked() {
                    self.open_file();
                }

                ui.separator();

                // Navigation (only if document loaded)
                ui.add_enabled_ui(self.document.is_some(), |ui| {
                    if ui.button("◀").clicked() && self.current_page > 0 {
                        self.current_page -= 1;
                        self.sidebar_scroll_to_current = true;
                    }

                    let page_text = if self.total_pages > 0 {
                        format!("{} / {}", self.current_page + 1, self.total_pages)
                    } else {
                        "— / —".to_string()
                    };
                    ui.label(page_text);

                    if ui.button("▶").clicked() && self.current_page + 1 < self.total_pages {
                        self.current_page += 1;
                        self.sidebar_scroll_to_current = true;
                    }

                    ui.separator();

                    // Zoom controls
                    if ui.button("−").clicked() {
                        self.zoom_level = (self.zoom_level - 10.0).max(25.0);
                    }

                    egui::ComboBox::from_id_salt("zoom")
                        .selected_text(format!("{}%", self.zoom_level as i32))
                        .width(70.0)
                        .show_ui(ui, |ui| {
                            for &level in &[25.0, 50.0, 75.0, 100.0, 125.0, 150.0, 200.0, 300.0] {
                                ui.selectable_value(
                                    &mut self.zoom_level,
                                    level,
                                    format!("{}%", level as i32),
                                );
                            }
                        });

                    if ui.button("+").clicked() {
                        self.zoom_level = (self.zoom_level + 10.0).min(500.0);
                    }

                    ui.separator();

                    // Tools
                    self.tool_button(ui, Tool::Select, "Select");
                    self.tool_button(ui, Tool::Hand, "Hand");
                    self.tool_button(ui, Tool::Text, "Text");
                    self.tool_button(ui, Tool::Highlight, "Highlight");
                    self.tool_button(ui, Tool::Comment, "Comment");
                    self.tool_button(ui, Tool::Measure, "Measure");
                    self.tool_button(ui, Tool::Freedraw, "Draw");
                });
            });
        });
    }

    fn tool_button(&mut self, ui: &mut egui::Ui, tool: Tool, label: &str) {
        let is_selected = self.current_tool == tool;
        if ui.selectable_label(is_selected, label).clicked() {
            self.current_tool = tool;
        }
    }

    fn draw_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("thumbnails")
            .default_width(130.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Pages");
                ui.separator();

                if self.document.is_none() {
                    ui.weak("No document loaded");
                    return;
                }

                // Render visible thumbnails lazily
                let scroll = egui::ScrollArea::vertical().show(ui, |ui| {
                    for page in 0..self.total_pages {
                        let is_current = page == self.current_page;

                        // Render thumbnail if not cached
                        self.render_thumbnail(ctx, page);

                        // Frame for selection highlight
                        let frame = if is_current {
                            egui::Frame::NONE
                                .stroke(egui::Stroke::new(2.0, ui.visuals().selection.bg_fill))
                                .inner_margin(2.0)
                                .corner_radius(4.0)
                        } else {
                            egui::Frame::NONE
                                .stroke(egui::Stroke::new(1.0, ui.visuals().widgets.inactive.bg_stroke.color))
                                .inner_margin(2.0)
                                .corner_radius(4.0)
                        };

                        let response = frame.show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                // Show thumbnail or placeholder
                                if let Some(thumb) = self.thumbnails.get(&page) {
                                    ui.image(&thumb.handle);
                                } else {
                                    // Placeholder while loading
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(100.0, 140.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(
                                        rect,
                                        4.0,
                                        ui.visuals().widgets.inactive.bg_fill,
                                    );
                                }

                                // Page number
                                ui.small(format!("{}", page + 1));
                            });
                        });

                        // Handle click
                        if response.response.interact(egui::Sense::click()).clicked() {
                            self.go_to_page(page);
                        }

                        // Scroll to current page if needed
                        if is_current && self.sidebar_scroll_to_current {
                            response.response.scroll_to_me(Some(egui::Align::Center));
                            self.sidebar_scroll_to_current = false;
                        }

                        ui.add_space(4.0);
                    }
                });

                let _ = scroll;
            });
    }

    fn render_viewport_page(&mut self, ctx: &egui::Context) -> Option<ViewportCacheKey> {
        let pdf = self.document.as_ref()?;
        let key = ViewportCacheKey {
            page: self.current_page,
            zoom_percent: self.zoom_level as u32,
        };

        if self.viewport_cache.contains_key(&key) {
            return Some(key);
        }

        let page = pdf.get_page(self.current_page as u16).ok()?;
        let page_width = page.width().value;
        let page_height = page.height().value;

        let scale = self.zoom_level / 100.0;
        let render_width = (page_width * scale) as u32;
        let render_height = (page_height * scale) as u32;

        match pdf.render_page_rgba(self.current_page as u16, render_width, render_height) {
            Ok(rgba_data) => {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [render_width as usize, render_height as usize],
                    &rgba_data,
                );
                let handle = ctx.load_texture(
                    format!("viewport_{}_{}", self.current_page, self.zoom_level as u32),
                    image,
                    egui::TextureOptions::LINEAR,
                );
                self.viewport_cache.insert(
                    key,
                    ViewportTexture {
                        handle,
                        width: render_width as f32,
                        height: render_height as f32,
                    },
                );
                Some(key)
            }
            Err(e) => {
                eprintln!("Failed to render page {}: {}", self.current_page, e);
                None
            }
        }
    }

    fn draw_viewport(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.document.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.heading("Open a PDF to get started");
                });
                return;
            }

            let key = self.render_viewport_page(ctx);

            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(key) = key {
                        if let Some(texture) = self.viewport_cache.get(&key) {
                            let size = egui::vec2(texture.width, texture.height);

                            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::drag());

                            if response.dragged() && self.current_tool == Tool::Hand {
                                self.viewport_offset += response.drag_delta();
                            }

                            ui.painter().image(
                                texture.handle.id(),
                                rect.translate(self.viewport_offset),
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                egui::Color32::WHITE,
                            );
                        }
                    }
                });
        });
    }

    fn draw_search_bar(&mut self, ctx: &egui::Context) {
        if !self.search_bar.visible {
            return;
        }

        egui::TopBottomPanel::top("search_bar")
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Search input
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.search_bar.query)
                            .hint_text("Search...")
                            .desired_width(200.0),
                    );

                    // Auto-focus on open
                    if response.gained_focus() || self.search_bar.query.is_empty() {
                        response.request_focus();
                    }

                    ui.separator();

                    // Match count
                    if self.search_bar.total_matches > 0 {
                        ui.label(format!(
                            "{} / {}",
                            self.search_bar.current_match, self.search_bar.total_matches
                        ));
                    } else if !self.search_bar.query.is_empty() {
                        ui.weak("No matches");
                    }

                    // Navigation buttons
                    if ui.button("▲").clicked() && self.search_bar.current_match > 1 {
                        self.search_bar.current_match -= 1;
                    }
                    if ui
                        .button("▼")
                        .clicked()
                        && self.search_bar.current_match < self.search_bar.total_matches
                    {
                        self.search_bar.current_match += 1;
                    }

                    ui.separator();

                    // Case sensitive toggle
                    ui.toggle_value(&mut self.search_bar.case_sensitive, "Aa")
                        .on_hover_text("Case sensitive");

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕").clicked() {
                            self.search_bar.visible = false;
                            self.search_bar.query.clear();
                            self.search_bar.current_match = 0;
                            self.search_bar.total_matches = 0;
                        }
                    });
                });
            });
    }

    fn draw_error_dialog(&mut self, ctx: &egui::Context) {
        let Some(error) = &self.error_dialog else {
            return;
        };

        let title = format!("{} {}", error.severity.icon(), error.title);
        let message = error.message.clone();

        let mut should_close = false;
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(&message);
                ui.add_space(12.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.button("OK").clicked() {
                        should_close = true;
                    }
                });
            });

        if should_close {
            self.error_dialog = None;
        }
    }

    fn draw_calibration_dialog(&mut self, ctx: &egui::Context) {
        let Some(cal) = &mut self.calibration_dialog else {
            return;
        };

        let mut should_close = false;
        let mut confirmed = false;

        egui::Window::new("Calibrate Measurement")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(format!("Page distance: {:.2} pts", cal.page_distance));
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Known distance:");
                    ui.add(
                        egui::TextEdit::singleline(&mut cal.distance_input)
                            .desired_width(80.0)
                            .hint_text("0.0"),
                    );

                    if ui.button(cal.selected_unit()).clicked() {
                        cal.cycle_unit();
                    }
                });

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }
                    if ui
                        .add_enabled(cal.parse_distance().is_some(), egui::Button::new("OK"))
                        .clicked()
                    {
                        confirmed = true;
                        should_close = true;
                    }
                });
            });

        if should_close {
            if confirmed {
                if let Some(cal) = &self.calibration_dialog {
                    if let Some(_distance) = cal.parse_distance() {
                        // TODO: Apply calibration to measurement tool
                    }
                }
            }
            self.calibration_dialog = None;
        }
    }
}
