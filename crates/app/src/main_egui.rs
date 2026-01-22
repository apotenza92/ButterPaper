//! PDF Editor - egui-based UI
//!
//! New entry point using eframe for UI chrome with system theme support.

use eframe::egui;

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

struct PdfEditorApp {
    current_page: usize,
    total_pages: usize,
    zoom_level: f32,
    current_tool: Tool,
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
            current_page: 1,
            total_pages: 10,
            zoom_level: 100.0,
            current_tool: Tool::default(),
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

                // Navigation
                if ui.button("◀").clicked() && self.current_page > 1 {
                    self.current_page -= 1;
                }

                let page_text = format!("{} / {}", self.current_page, self.total_pages);
                ui.label(page_text);

                if ui.button("▶").clicked() && self.current_page < self.total_pages {
                    self.current_page += 1;
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
    }

    fn tool_button(&mut self, ui: &mut egui::Ui, tool: Tool, label: &str) {
        let is_selected = self.current_tool == tool;
        if ui.selectable_label(is_selected, label).clicked() {
            self.current_tool = tool;
        }
    }

    fn draw_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("thumbnails")
            .default_width(120.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Pages");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for page in 1..=self.total_pages {
                        let is_current = page == self.current_page;
                        let response = ui.selectable_label(is_current, format!("Page {}", page));
                        if response.clicked() {
                            self.current_page = page;
                        }
                    }
                });
            });
    }

    fn draw_viewport(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                ui.heading(format!(
                    "PDF Viewport - Page {} at {}%",
                    self.current_page, self.zoom_level as i32
                ));
            });
        });
    }
}
