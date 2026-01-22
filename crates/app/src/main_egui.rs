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

    // UI state
    sidebar_scroll_to_current: bool,
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
            sidebar_scroll_to_current: false,
        }
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
                self.sidebar_scroll_to_current = true;
            }
            Err(e) => {
                eprintln!("Failed to open PDF: {}", e);
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
        if page < self.total_pages {
            self.current_page = page;
            self.sidebar_scroll_to_current = true;
        }
    }
}

impl eframe::App for PdfEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.draw_toolbar(ctx);
        self.draw_sidebar(ctx);
        self.draw_viewport(ctx);
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

    fn draw_viewport(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.document.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.heading("Open a PDF to get started");
                });
                return;
            }

            ui.centered_and_justified(|ui| {
                ui.heading(format!(
                    "Page {} at {}%",
                    self.current_page + 1,
                    self.zoom_level as i32
                ));
            });
        });
    }
}
