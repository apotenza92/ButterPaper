//! ButterPaper main component with tabbed document management.

use gpui::{
    div, prelude::*, px, App, ClickEvent, Context, ExternalPaths, FocusHandle, Focusable,
    KeyDownEvent, MouseButton, Rgba, ScrollDelta, ScrollWheelEvent, SharedString, Window,
};
use std::path::PathBuf;

use super::document::DocumentTab;
use crate::components::tab_bar::TabId as UiTabId;
use crate::components::{
    context_menu, icon, icon_button, popover_menu, tab_item, text_button_with_shortcut, ButtonSize,
    ContextMenuItem, Icon, TabItemData,
};
use crate::sidebar::{ThumbnailSidebar, SIDEBAR_WIDTH};
use crate::styles::DynamicSpacing;
use crate::viewport::PdfViewport;
use crate::workspace::{load_preferences, TabPreferences};
use crate::{current_theme, ui, Theme};
use crate::{
    CloseTab, CloseWindow, FirstPage, FitPage, FitWidth, LastPage, NextPage, NextTab, Open,
    PrevPage, PrevTab, ResetZoom, ZoomIn, ZoomOut,
};

const MIN_ZOOM_PERCENT: u32 = 25;
const MAX_ZOOM_PERCENT: u32 = 400;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuKind {
    File,
    View,
    Go,
    Window,
    Help,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuCommand {
    Open,
    NewTab,
    CloseTab,
    ZoomIn,
    ZoomOut,
    ResetZoom,
    FitWidth,
    FitPage,
    FirstPage,
    PrevPage,
    NextPage,
    LastPage,
    NextTab,
    PrevTab,
    About,
}

#[derive(Clone, Copy, Debug)]
struct ActiveViewportInfo {
    has_document: bool,
    page_count: u16,
    current_page: u16,
    zoom_level: u32,
}

impl Default for ActiveViewportInfo {
    fn default() -> Self {
        Self { has_document: false, page_count: 0, current_page: 0, zoom_level: 100 }
    }
}

pub struct PdfEditor {
    tabs: Vec<DocumentTab>,
    active_tab_index: usize,
    focus_handle: FocusHandle,
    zoom_input_focus_handle: FocusHandle,
    preferences: TabPreferences,
    /// Horizontal scroll offset for the tab bar (in pixels)
    tab_scroll_offset: f32,
    /// Whether the thumbnail sidebar is visible for the active tab.
    thumbnail_sidebar_visible: bool,
    /// Which in-window menu is currently open.
    open_menu: Option<MenuKind>,
    /// Current text shown in the zoom combo field.
    zoom_input_text: String,
    /// Whether the zoom field is in edit mode.
    zoom_input_editing: bool,
    /// Whether the zoom presets dropdown is open.
    zoom_preset_open: bool,
}

fn next_active_tab_index_after_close(
    active_index: usize,
    closed_index: usize,
    remaining_len: usize,
) -> Option<usize> {
    if remaining_len == 0 {
        return None;
    }

    let next = if closed_index < active_index {
        active_index.saturating_sub(1)
    } else if closed_index == active_index {
        active_index.min(remaining_len.saturating_sub(1))
    } else {
        active_index
    };
    Some(next.min(remaining_len.saturating_sub(1)))
}

fn clamp_zoom_percent(zoom: u32) -> u32 {
    zoom.clamp(MIN_ZOOM_PERCENT, MAX_ZOOM_PERCENT)
}

fn parse_zoom_input_percent(input: &str) -> Option<u32> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let numeric = trimmed.strip_suffix('%').unwrap_or(trimmed).trim();
    if numeric.is_empty() {
        return None;
    }

    numeric.parse::<u32>().ok().map(clamp_zoom_percent)
}

fn compute_canvas_metrics(
    viewport_width: f32,
    viewport_height: f32,
    show_sidebar: bool,
) -> (f32, f32) {
    let sidebar = if show_sidebar { SIDEBAR_WIDTH } else { 0.0 };
    let canvas_width = (viewport_width - ui::sizes::TOOL_RAIL_WIDTH.0 - sidebar).max(1.0);
    let chrome_height = ui::sizes::TITLE_BAR_HEIGHT.0
        + ui::sizes::MENU_ROW_HEIGHT.0
        + ui::sizes::TAB_BAR_HEIGHT.0
        + ui::sizes::CANVAS_TOOLBAR_HEIGHT.0;
    let canvas_height = (viewport_height - chrome_height).max(1.0);
    (canvas_width, canvas_height)
}

fn map_menu_command(value: &str) -> Option<MenuCommand> {
    match value {
        "file.open" => Some(MenuCommand::Open),
        "file.new_tab" => Some(MenuCommand::NewTab),
        "file.close_tab" => Some(MenuCommand::CloseTab),
        "view.zoom_in" => Some(MenuCommand::ZoomIn),
        "view.zoom_out" => Some(MenuCommand::ZoomOut),
        "view.reset_zoom" => Some(MenuCommand::ResetZoom),
        "view.fit_width" => Some(MenuCommand::FitWidth),
        "view.fit_page" => Some(MenuCommand::FitPage),
        "go.first" => Some(MenuCommand::FirstPage),
        "go.prev" => Some(MenuCommand::PrevPage),
        "go.next" => Some(MenuCommand::NextPage),
        "go.last" => Some(MenuCommand::LastPage),
        "window.next_tab" => Some(MenuCommand::NextTab),
        "window.prev_tab" => Some(MenuCommand::PrevTab),
        "help.about" => Some(MenuCommand::About),
        _ => None,
    }
}

fn flat_icon_button<F>(
    id: impl Into<SharedString>,
    icon_type: Icon,
    enabled: bool,
    selected: bool,
    theme: &Theme,
    on_click: F,
) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let transparent = Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    let selected_bg = if selected {
        Rgba {
            r: theme.element_selected.r,
            g: theme.element_selected.g,
            b: theme.element_selected.b,
            a: theme.element_selected.a * 0.78,
        }
    } else {
        transparent
    };
    let hover_bg = if selected {
        selected_bg
    } else {
        Rgba {
            r: theme.element_hover.r,
            g: theme.element_hover.g,
            b: theme.element_hover.b,
            a: theme.element_hover.a * 0.8,
        }
    };
    let active_bg = Rgba {
        r: theme.element_selected.r,
        g: theme.element_selected.g,
        b: theme.element_selected.b,
        a: theme.element_selected.a * 0.84,
    };
    let icon_color = if !enabled {
        Rgba {
            r: theme.text_muted.r,
            g: theme.text_muted.g,
            b: theme.text_muted.b,
            a: theme.text_muted.a * 0.65,
        }
    } else if selected {
        theme.text
    } else {
        Rgba {
            r: theme.text_muted.r,
            g: theme.text_muted.g,
            b: theme.text_muted.b,
            a: theme.text_muted.a * 0.96,
        }
    };

    div()
        .id(id.into())
        .w(px(32.0))
        .h(px(32.0))
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(px(5.0))
        .bg(selected_bg)
        .when(enabled, move |d| {
            d.cursor_pointer()
                .hover(move |s| s.bg(hover_bg))
                .active(move |s| s.bg(active_bg))
                .on_click(on_click)
        })
        .child(icon(icon_type, 15.0, icon_color))
}

impl PdfEditor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            tabs: Vec::new(),
            active_tab_index: 0,
            focus_handle: cx.focus_handle(),
            zoom_input_focus_handle: cx.focus_handle(),
            preferences: load_preferences(),
            tab_scroll_offset: 0.0,
            thumbnail_sidebar_visible: true,
            open_menu: None,
            zoom_input_text: "100%".to_string(),
            zoom_input_editing: false,
            zoom_preset_open: false,
        }
    }

    /// Create a new document tab, optionally with a file path.
    /// If path is None, creates a welcome tab.
    fn create_tab(&mut self, path: Option<PathBuf>, cx: &mut Context<Self>) -> usize {
        let viewport = cx.new(PdfViewport::new);
        let sidebar = cx.new(ThumbnailSidebar::new);

        // Set up page change callback from viewport to sidebar
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
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Welcome".to_string());

        let tab =
            DocumentTab { id: UiTabId::new(), path, title, viewport, sidebar, is_dirty: false };

        self.tabs.push(tab);
        self.tabs.len() - 1
    }

    fn create_welcome_tab(&mut self, cx: &mut Context<Self>) -> usize {
        self.create_tab(None, cx)
    }

    fn new_tab(&mut self, cx: &mut Context<Self>) {
        let idx = self.create_welcome_tab(cx);
        self.active_tab_index = idx;
        cx.notify();
    }

    fn active_tab(&self) -> Option<&DocumentTab> {
        self.tabs.get(self.active_tab_index)
    }

    fn active_viewport_info(&self, cx: &App) -> ActiveViewportInfo {
        let Some(tab) = self.active_tab() else {
            return ActiveViewportInfo::default();
        };

        let viewport = tab.viewport.read(cx);
        ActiveViewportInfo {
            has_document: viewport.has_document(),
            page_count: viewport.page_count(),
            current_page: viewport.current_page(),
            zoom_level: viewport.zoom_level,
        }
    }

    fn sync_zoom_input_from_active(&mut self, cx: &App) {
        if self.zoom_input_editing {
            return;
        }

        let zoom = self.active_viewport_info(cx).zoom_level;
        self.zoom_input_text = format!("{}%", zoom);
    }

    pub fn open_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        // Check if file is already open in a tab
        if let Some(idx) = self.tabs.iter().position(|t| t.path.as_ref() == Some(&path)) {
            self.active_tab_index = idx;
            cx.notify();
            return;
        }

        // If current tab is a welcome tab, reuse it instead of creating a new one
        let tab_index = if self.active_tab().map(|t| t.is_welcome()).unwrap_or(false) {
            let tab = &mut self.tabs[self.active_tab_index];
            tab.path = Some(path.clone());
            tab.title = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Untitled".to_string());
            self.active_tab_index
        } else {
            let idx = self.create_tab(Some(path.clone()), cx);
            self.active_tab_index = idx;
            idx
        };

        let tab = &self.tabs[tab_index];
        tab.viewport.update(cx, |viewport, cx| {
            if let Err(e) = viewport.load_pdf(path, cx) {
                eprintln!("Error loading PDF: {}", e);
            }
        });

        let doc = tab.viewport.read(cx).document();
        tab.sidebar.update(cx, |sidebar, cx| {
            sidebar.set_document(doc, cx);
        });

        self.zoom_input_editing = false;
        self.zoom_preset_open = false;
        self.sync_zoom_input_from_active(cx);
        cx.notify();
    }

    fn handle_file_drop(&mut self, paths: &ExternalPaths, cx: &mut Context<Self>) {
        for path in paths.paths() {
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == "pdf" {
                    self.open_file(path.clone(), cx);
                }
            }
        }
    }

    fn handle_tab_scroll(&mut self, delta: ScrollDelta, cx: &mut Context<Self>) {
        let scroll_amount: f32 = match delta {
            ScrollDelta::Lines(lines) => {
                let horizontal = lines.x * 30.0;
                let vertical_as_horizontal = lines.y * 30.0;
                if horizontal.abs() > vertical_as_horizontal.abs() {
                    horizontal
                } else {
                    vertical_as_horizontal
                }
            }
            ScrollDelta::Pixels(pixels) => {
                let px_x: f32 = pixels.x.into();
                let px_y: f32 = pixels.y.into();
                if px_x.abs() > px_y.abs() {
                    px_x
                } else {
                    px_y
                }
            }
        };

        self.tab_scroll_offset = (self.tab_scroll_offset - scroll_amount).max(0.0);
        cx.notify();
    }

    fn select_tab(&mut self, tab_id: UiTabId, cx: &mut Context<Self>) {
        if let Some(idx) = self.tabs.iter().position(|t| t.id == tab_id) {
            self.active_tab_index = idx;
            self.zoom_input_editing = false;
            self.zoom_preset_open = false;
            self.sync_zoom_input_from_active(cx);
            cx.notify();
        }
    }

    fn close_tab(&mut self, tab_id: UiTabId, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(idx) = self.tabs.iter().position(|t| t.id == tab_id) {
            self.tabs.remove(idx);

            if let Some(next_index) =
                next_active_tab_index_after_close(self.active_tab_index, idx, self.tabs.len())
            {
                self.active_tab_index = next_index;
            } else {
                self.active_tab_index = 0;
            }

            self.zoom_input_editing = false;
            self.zoom_preset_open = false;
            self.sync_zoom_input_from_active(cx);
            cx.notify();
        }
    }

    fn next_tab(&mut self, cx: &mut Context<Self>) {
        if !self.tabs.is_empty() {
            self.active_tab_index = (self.active_tab_index + 1) % self.tabs.len();
            self.sync_zoom_input_from_active(cx);
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
            self.sync_zoom_input_from_active(cx);
            cx.notify();
        }
    }

    fn show_tab_bar(&self) -> bool {
        self.preferences.show_tab_bar || self.tabs.len() > 1
    }

    fn close_transient_ui(&mut self) {
        self.open_menu = None;
        self.zoom_preset_open = false;
    }

    fn toggle_menu(&mut self, kind: MenuKind, cx: &mut Context<Self>) {
        self.zoom_preset_open = false;
        if self.open_menu == Some(kind) {
            self.open_menu = None;
        } else {
            self.open_menu = Some(kind);
        }
        cx.notify();
    }

    fn apply_zoom_input(&mut self, cx: &mut Context<Self>) {
        let Some(value) = parse_zoom_input_percent(&self.zoom_input_text) else {
            self.zoom_input_editing = false;
            self.sync_zoom_input_from_active(cx);
            cx.notify();
            return;
        };

        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.set_zoom(value, cx);
            });
        }

        self.zoom_input_editing = false;
        self.zoom_input_text = format!("{}%", value);
        cx.notify();
    }

    fn cancel_zoom_input_edit(&mut self, cx: &mut Context<Self>) {
        self.zoom_input_editing = false;
        self.sync_zoom_input_from_active(cx);
        cx.notify();
    }

    fn handle_zoom_input_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();

        match key {
            "enter" => {
                self.apply_zoom_input(cx);
                window.focus(&self.focus_handle);
                cx.stop_propagation();
                return;
            }
            "escape" => {
                self.cancel_zoom_input_edit(cx);
                window.focus(&self.focus_handle);
                cx.stop_propagation();
                return;
            }
            "backspace" => {
                self.zoom_input_editing = true;
                self.zoom_input_text.pop();
                cx.notify();
                cx.stop_propagation();
                return;
            }
            _ => {}
        }

        if let Some(input) = event.keystroke.key_char.as_ref() {
            let Some(ch) = input.chars().next() else {
                return;
            };

            if ch.is_ascii_digit() {
                self.zoom_input_editing = true;
                if self.zoom_input_text.ends_with('%') {
                    self.zoom_input_text.pop();
                }
                if self.zoom_input_text.len() < 4 {
                    self.zoom_input_text.push(ch);
                }
                cx.notify();
                cx.stop_propagation();
            } else if ch == '%' {
                self.zoom_input_editing = true;
                if !self.zoom_input_text.ends_with('%') {
                    self.zoom_input_text.push('%');
                }
                cx.notify();
                cx.stop_propagation();
            }
        }
    }

    fn handle_menu_command(
        &mut self,
        command: MenuCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match command {
            MenuCommand::Open => self.handle_open(&Open, window, cx),
            MenuCommand::NewTab => self.new_tab(cx),
            MenuCommand::CloseTab => self.handle_close_tab(&CloseTab, window, cx),
            MenuCommand::ZoomIn => self.handle_zoom_in(&ZoomIn, window, cx),
            MenuCommand::ZoomOut => self.handle_zoom_out(&ZoomOut, window, cx),
            MenuCommand::ResetZoom => self.handle_reset_zoom(&ResetZoom, window, cx),
            MenuCommand::FitWidth => self.handle_fit_width(&FitWidth, window, cx),
            MenuCommand::FitPage => self.handle_fit_page(&FitPage, window, cx),
            MenuCommand::FirstPage => self.handle_first_page(&FirstPage, window, cx),
            MenuCommand::PrevPage => self.handle_prev_page(&PrevPage, window, cx),
            MenuCommand::NextPage => self.handle_next_page(&NextPage, window, cx),
            MenuCommand::LastPage => self.handle_last_page(&LastPage, window, cx),
            MenuCommand::NextTab => self.handle_next_tab(&NextTab, window, cx),
            MenuCommand::PrevTab => self.handle_prev_tab(&PrevTab, window, cx),
            MenuCommand::About => {
                println!("ButterPaper - GPUI Edition");
            }
        }
    }

    fn handle_zoom_in(&mut self, _: &ZoomIn, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.zoom_in(cx);
            });
            self.sync_zoom_input_from_active(cx);
        }
    }

    fn handle_zoom_out(&mut self, _: &ZoomOut, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.zoom_out(cx);
            });
            self.sync_zoom_input_from_active(cx);
        }
    }

    fn handle_reset_zoom(&mut self, _: &ResetZoom, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.reset_zoom(cx);
            });
            self.sync_zoom_input_from_active(cx);
        }
    }

    fn handle_fit_width(&mut self, _: &FitWidth, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.fit_width(cx);
            });
            self.sync_zoom_input_from_active(cx);
        }
    }

    fn handle_fit_page(&mut self, _: &FitPage, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.fit_page(cx);
            });
            self.sync_zoom_input_from_active(cx);
        }
    }

    fn handle_next_page(&mut self, _: &NextPage, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.next_page(cx);
            });
            self.sync_zoom_input_from_active(cx);
        }
    }

    fn handle_prev_page(&mut self, _: &PrevPage, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.prev_page(cx);
            });
            self.sync_zoom_input_from_active(cx);
        }
    }

    fn handle_first_page(&mut self, _: &FirstPage, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.first_page(cx);
            });
        }
    }

    fn handle_last_page(&mut self, _: &LastPage, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.last_page(cx);
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
                let title = doc_tab.title.clone();
                let is_dirty = doc_tab.is_dirty;
                let tab_id = doc_tab.id;

                let entity_for_select = entity.clone();
                let entity_for_close = entity.clone();

                tab_item(
                    TabItemData::new(tab_id, title, is_active, is_dirty),
                    theme,
                    move |_, _, cx| {
                        if let Some(editor) = entity_for_select.upgrade() {
                            editor.update(cx, |editor, cx| {
                                editor.select_tab(tab_id, cx);
                            });
                        }
                    },
                    move |_, window, cx| {
                        if let Some(editor) = entity_for_close.upgrade() {
                            editor.update(cx, |editor, cx| {
                                editor.close_tab(tab_id, window, cx);
                            });
                        }
                    },
                )
            })
            .collect()
    }

    fn render_menu_entry(
        &self,
        id: &'static str,
        label: &'static str,
        kind: MenuKind,
        items: Vec<ContextMenuItem>,
        theme: &Theme,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let entity_for_toggle = cx.entity().downgrade();
        let entity_for_select = cx.entity().downgrade();
        let is_open = self.open_menu == Some(kind);

        let trigger = div()
            .id(id)
            .h_full()
            .px(ui::sizes::SPACE_2)
            .flex()
            .items_center()
            .text_sm()
            .text_color(if is_open { theme.text } else { theme.text_muted })
            .cursor_pointer()
            .rounded_sm()
            .hover({
                let hover = theme.element_hover;
                move |s| s.bg(hover)
            })
            .on_click(move |_, _, cx| {
                if let Some(editor) = entity_for_toggle.upgrade() {
                    editor.update(cx, |editor, cx| {
                        editor.toggle_menu(kind, cx);
                    });
                }
            })
            .child(label);

        let menu = context_menu(format!("{id}-menu"), items, theme, move |value, window, cx| {
            if let Some(command) = map_menu_command(value) {
                if let Some(editor) = entity_for_select.upgrade() {
                    editor.update(cx, |editor, cx| {
                        editor.close_transient_ui();
                        editor.handle_menu_command(command, window, cx);
                    });
                }
            }
        });

        div()
            .on_mouse_down_out(cx.listener(move |this, _event, _window, cx| {
                if this.open_menu == Some(kind) {
                    this.open_menu = None;
                    cx.notify();
                }
            }))
            .child(popover_menu(trigger, menu, is_open))
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::{
        compute_canvas_metrics, map_menu_command, next_active_tab_index_after_close,
        parse_zoom_input_percent, MenuCommand,
    };

    #[test]
    fn close_tab_before_active_shifts_left() {
        let next = next_active_tab_index_after_close(3, 1, 4);
        assert_eq!(next, Some(2));
    }

    #[test]
    fn close_active_tab_selects_next_or_previous_when_last() {
        let next_middle = next_active_tab_index_after_close(1, 1, 3);
        assert_eq!(next_middle, Some(1));

        let next_last = next_active_tab_index_after_close(2, 2, 2);
        assert_eq!(next_last, Some(1));
    }

    #[test]
    fn close_tab_after_active_keeps_active_index() {
        let next = next_active_tab_index_after_close(1, 3, 4);
        assert_eq!(next, Some(1));
    }

    #[test]
    fn close_last_remaining_tab_returns_none() {
        let next = next_active_tab_index_after_close(0, 0, 0);
        assert_eq!(next, None);
    }

    #[test]
    fn parse_zoom_accepts_percent_and_plain_numbers() {
        assert_eq!(parse_zoom_input_percent("125"), Some(125));
        assert_eq!(parse_zoom_input_percent("125%"), Some(125));
        assert_eq!(parse_zoom_input_percent("   300 %  "), Some(300));
    }

    #[test]
    fn parse_zoom_clamps_and_rejects_invalid() {
        assert_eq!(parse_zoom_input_percent("10"), Some(25));
        assert_eq!(parse_zoom_input_percent("900"), Some(400));
        assert_eq!(parse_zoom_input_percent("abc"), None);
        assert_eq!(parse_zoom_input_percent(""), None);
    }

    #[test]
    fn canvas_metrics_account_for_sidebar() {
        let (w_open, h_open) = compute_canvas_metrics(1200.0, 900.0, true);
        let (w_closed, h_closed) = compute_canvas_metrics(1200.0, 900.0, false);

        assert!(w_closed > w_open);
        assert_eq!(h_open, h_closed);
    }

    #[test]
    fn menu_command_mapping_is_stable() {
        assert_eq!(map_menu_command("file.open"), Some(MenuCommand::Open));
        assert_eq!(map_menu_command("view.fit_page"), Some(MenuCommand::FitPage));
        assert_eq!(map_menu_command("unknown"), None);
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
        let show_tab_bar = self.show_tab_bar();

        let viewport_size = window.viewport_size();
        let show_sidebar = self.thumbnail_sidebar_visible && self.active_tab().is_some();
        let (canvas_width, canvas_height) =
            compute_canvas_metrics(viewport_size.width.0, viewport_size.height.0, show_sidebar);

        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.set_canvas_metrics(canvas_width, canvas_height, cx);
            });
        }

        let viewport_info = self.active_viewport_info(cx);
        if !self.zoom_input_editing {
            self.zoom_input_text = format!("{}%", viewport_info.zoom_level);
        }

        let page_label = if viewport_info.page_count > 0 {
            format!("{} / {}", viewport_info.current_page + 1, viewport_info.page_count)
        } else {
            "0 / 0".to_string()
        };

        let can_navigate = viewport_info.has_document && viewport_info.page_count > 0;
        let is_first = viewport_info.current_page == 0;
        let is_last = viewport_info.page_count > 0
            && viewport_info.current_page >= viewport_info.page_count.saturating_sub(1);

        let can_first_prev = can_navigate && !is_first;
        let can_next_last = can_navigate && !is_last;
        let can_zoom = viewport_info.has_document;

        let toolbar_separator = Rgba {
            r: theme.border.r,
            g: theme.border.g,
            b: theme.border.b,
            a: theme.border.a * 0.2,
        };
        let toolbar_chrome_border = Rgba {
            r: theme.border.r,
            g: theme.border.g,
            b: theme.border.b,
            a: theme.border.a * 0.38,
        };
        let zoom_combo_border = Rgba {
            r: theme.border.r,
            g: theme.border.g,
            b: theme.border.b,
            a: theme.border.a * 0.3,
        };
        let zoom_combo_hover = Rgba {
            r: theme.element_hover.r,
            g: theme.element_hover.g,
            b: theme.element_hover.b,
            a: theme.element_hover.a * 0.8,
        };

        let file_items = vec![
            ContextMenuItem::new("file.open", "Open…"),
            ContextMenuItem::new("file.new_tab", "New Tab"),
            ContextMenuItem::new("file.close_tab", "Close Tab")
                .disabled(self.active_tab().is_none()),
        ];
        let view_items = vec![
            ContextMenuItem::new("view.zoom_in", "Zoom In").disabled(!viewport_info.has_document),
            ContextMenuItem::new("view.zoom_out", "Zoom Out").disabled(!viewport_info.has_document),
            ContextMenuItem::new("view.reset_zoom", "100%").disabled(!viewport_info.has_document),
            ContextMenuItem::new("view.fit_width", "Fit Width")
                .disabled(!viewport_info.has_document),
            ContextMenuItem::new("view.fit_page", "Fit Page").disabled(!viewport_info.has_document),
        ];
        let go_items = vec![
            ContextMenuItem::new("go.first", "First Page").disabled(!can_first_prev),
            ContextMenuItem::new("go.prev", "Previous Page").disabled(!can_first_prev),
            ContextMenuItem::new("go.next", "Next Page").disabled(!can_next_last),
            ContextMenuItem::new("go.last", "Last Page").disabled(!can_next_last),
        ];
        let window_items = vec![
            ContextMenuItem::new("window.next_tab", "Next Tab").disabled(self.tabs.len() < 2),
            ContextMenuItem::new("window.prev_tab", "Previous Tab").disabled(self.tabs.len() < 2),
        ];
        let help_items = vec![ContextMenuItem::new("help.about", "About ButterPaper")];

        let entity_for_zoom_preset = cx.entity().downgrade();
        let zoom_presets = vec![50_u32, 75, 100, 125, 150, 200, 300, 400]
            .into_iter()
            .map(|z| {
                ContextMenuItem::new(format!("zoom.{z}"), format!("{z}%"))
                    .disabled(!viewport_info.has_document)
            })
            .collect::<Vec<_>>();

        let zoom_combo_trigger = {
            let entity_for_click = cx.entity().downgrade();
            let zoom_focus = self.zoom_input_focus_handle.clone();
            let entity_for_toggle = cx.entity().downgrade();

            div()
                .id("zoom-combo")
                .h(ui::sizes::CONTROL_HEIGHT_DEFAULT)
                .min_w(ui::sizes::ZOOM_COMBO_MIN_WIDTH)
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .bg(theme.background)
                .border_1()
                .border_color(zoom_combo_border)
                .rounded(px(6.0))
                .when(can_zoom, |d| d.cursor_pointer().hover(move |s| s.bg(zoom_combo_hover)))
                .child(
                    div()
                        .id("zoom-combo-input")
                        .track_focus(&self.zoom_input_focus_handle)
                        .h_full()
                        .flex()
                        .items_center()
                        .px(ui::sizes::SPACE_2)
                        .text_xs()
                        .text_color(if can_zoom { theme.text } else { theme.text_muted })
                        .child(self.zoom_input_text.clone())
                        .on_click(move |_, window, cx| {
                            if let Some(editor) = entity_for_click.upgrade() {
                                editor.update(cx, |editor, cx| {
                                    if !can_zoom {
                                        return;
                                    }
                                    editor.zoom_input_editing = true;
                                    editor.open_menu = None;
                                    cx.notify();
                                    window.focus(&zoom_focus);
                                });
                            }
                        })
                        .on_key_down(cx.listener(Self::handle_zoom_input_key)),
                )
                .child(
                    div()
                        .id("zoom-combo-toggle")
                        .h_full()
                        .px(ui::sizes::SPACE_1)
                        .flex()
                        .items_center()
                        .text_xs()
                        .text_color(theme.text_muted)
                        .border_l_1()
                        .border_color(zoom_combo_border)
                        .child(icon(Icon::ChevronDown, 10.0, theme.text_muted))
                        .when(can_zoom, move |d| d.hover(move |s| s.bg(zoom_combo_hover)))
                        .on_click(move |_, _, cx| {
                            if let Some(editor) = entity_for_toggle.upgrade() {
                                editor.update(cx, |editor, cx| {
                                    if !can_zoom {
                                        return;
                                    }
                                    editor.open_menu = None;
                                    editor.zoom_preset_open = !editor.zoom_preset_open;
                                    cx.notify();
                                });
                            }
                        }),
                )
        };

        let zoom_combo_menu =
            context_menu("zoom-preset-menu", zoom_presets, &theme, move |value, _window, cx| {
                if let Some(editor) = entity_for_zoom_preset.upgrade() {
                    editor.update(cx, |editor, cx| {
                        if let Some(raw) = value.strip_prefix("zoom.") {
                            if let Ok(zoom) = raw.parse::<u32>() {
                                if let Some(tab) = editor.active_tab() {
                                    let viewport = tab.viewport.clone();
                                    viewport.update(cx, |viewport, cx| {
                                        viewport.set_zoom(zoom, cx);
                                    });
                                }
                            }
                        }
                        editor.zoom_preset_open = false;
                        editor.sync_zoom_input_from_active(cx);
                        cx.notify();
                    });
                }
            });

        let zoom_combo = div()
            .on_mouse_down_out(cx.listener(|this, _event, _window, cx| {
                let mut changed = false;
                if this.zoom_preset_open {
                    this.zoom_preset_open = false;
                    changed = true;
                }
                if this.zoom_input_editing {
                    this.zoom_input_editing = false;
                    this.sync_zoom_input_from_active(cx);
                    changed = true;
                }
                if changed {
                    cx.notify();
                }
            }))
            .child(popover_menu(zoom_combo_trigger, zoom_combo_menu, self.zoom_preset_open));

        let active_viewport = self.active_tab().map(|t| t.viewport.clone());

        div()
            .id("butterpaper")
            .key_context("PdfEditor")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::handle_zoom_in))
            .on_action(cx.listener(Self::handle_zoom_out))
            .on_action(cx.listener(Self::handle_reset_zoom))
            .on_action(cx.listener(Self::handle_fit_width))
            .on_action(cx.listener(Self::handle_fit_page))
            .on_action(cx.listener(Self::handle_next_page))
            .on_action(cx.listener(Self::handle_prev_page))
            .on_action(cx.listener(Self::handle_first_page))
            .on_action(cx.listener(Self::handle_last_page))
            .on_action(cx.listener(Self::handle_open))
            .on_action(cx.listener(Self::handle_close_window))
            .on_action(cx.listener(Self::handle_next_tab))
            .on_action(cx.listener(Self::handle_prev_tab))
            .on_action(cx.listener(Self::handle_close_tab))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _window, cx| {
                this.handle_file_drop(paths, cx);
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    if this.zoom_input_editing {
                        this.cancel_zoom_input_edit(cx);
                        window.focus(&this.focus_handle);
                        cx.stop_propagation();
                        return;
                    }
                    if this.open_menu.is_some() || this.zoom_preset_open {
                        this.close_transient_ui();
                        cx.notify();
                        cx.stop_propagation();
                    }
                }
            }))
            .flex()
            .flex_col()
            .bg(theme.background)
            .text_color(theme.text)
            .size_full()
            .child(ui::title_bar("ButterPaper", theme.text, theme.border))
            .child(
                div()
                    .id("app-menu-row")
                    .h(ui::sizes::MENU_ROW_HEIGHT)
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(ui::sizes::SPACE_1)
                    .px(ui::sizes::SPACE_2)
                    .bg(theme.surface)
                    .border_b_1()
                    .border_color(theme.border)
                    .child(self.render_menu_entry(
                        "menu-file",
                        "File",
                        MenuKind::File,
                        file_items,
                        &theme,
                        cx,
                    ))
                    .child(self.render_menu_entry(
                        "menu-view",
                        "View",
                        MenuKind::View,
                        view_items,
                        &theme,
                        cx,
                    ))
                    .child(self.render_menu_entry(
                        "menu-go",
                        "Go",
                        MenuKind::Go,
                        go_items,
                        &theme,
                        cx,
                    ))
                    .child(self.render_menu_entry(
                        "menu-window",
                        "Window",
                        MenuKind::Window,
                        window_items,
                        &theme,
                        cx,
                    ))
                    .child(self.render_menu_entry(
                        "menu-help",
                        "Help",
                        MenuKind::Help,
                        help_items,
                        &theme,
                        cx,
                    )),
            )
            .child(
                div()
                    .id("app-tab-row")
                    .h(ui::sizes::TAB_BAR_HEIGHT)
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .bg(theme.surface)
                    .border_b_1()
                    .border_color(theme.border)
                    .when(show_tab_bar, |d| {
                        let scroll_offset = self.tab_scroll_offset;
                        let tab_items = self.render_tab_items(&theme, cx);
                        d.child(
                            div()
                                .id("tabs-scroll")
                                .flex()
                                .flex_row()
                                .h_full()
                                .min_w_0()
                                .flex_1()
                                .overflow_hidden()
                                .on_scroll_wheel(cx.listener(
                                    |this, event: &ScrollWheelEvent, _window, cx| {
                                        this.handle_tab_scroll(event.delta, cx);
                                    },
                                ))
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .h_full()
                                        .ml(px(-scroll_offset))
                                        .children(tab_items),
                                ),
                        )
                    })
                    .when(!show_tab_bar, {
                        let title = self.active_tab().map(|t| t.title.clone()).unwrap_or_default();
                        move |d| {
                            d.child(
                                div()
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .px(ui::sizes::SPACE_3)
                                    .text_sm()
                                    .text_color(theme.text)
                                    .child(title),
                            )
                        }
                    })
                    .child({
                        let entity = cx.entity().downgrade();
                        div().id("tab-bar-empty").flex_1().h_full().on_mouse_down(
                            MouseButton::Left,
                            move |event, _window, cx| {
                                if event.click_count == 2 {
                                    if let Some(editor) = entity.upgrade() {
                                        editor.update(cx, |editor, cx| {
                                            editor.new_tab(cx);
                                        });
                                    }
                                }
                            },
                        )
                    })
                    .child({
                        let entity = cx.entity().downgrade();
                        div()
                            .id("new-tab")
                            .h_full()
                            .px(DynamicSpacing::Base02.px(cx))
                            .flex()
                            .items_center()
                            .border_l_1()
                            .border_color(theme.border)
                            .child(icon_button(
                                "new-tab-button",
                                Icon::Plus,
                                ButtonSize::Default,
                                &theme,
                                move |_, _, cx| {
                                    if let Some(editor) = entity.upgrade() {
                                        editor.update(cx, |editor, cx| {
                                            editor.new_tab(cx);
                                        });
                                    }
                                },
                            ))
                    }),
            )
            .child(
                div()
                    .id("content-row")
                    .flex()
                    .flex_row()
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        div()
                            .id("tool-rail")
                            .w(ui::sizes::TOOL_RAIL_WIDTH)
                            .h_full()
                            .flex()
                            .flex_col()
                            .items_center()
                            .pt(ui::sizes::SPACE_3)
                            .bg(theme.surface)
                            .border_r_1()
                            .border_color(toolbar_chrome_border)
                            .child({
                                let entity = cx.entity().downgrade();
                                flat_icon_button(
                                    "tool-rail-toggle-thumbnails",
                                    Icon::PanelLeft,
                                    true,
                                    self.thumbnail_sidebar_visible,
                                    &theme,
                                    move |_, _, cx| {
                                        if let Some(editor) = entity.upgrade() {
                                            editor.update(cx, |editor, cx| {
                                                editor.thumbnail_sidebar_visible =
                                                    !editor.thumbnail_sidebar_visible;
                                                cx.notify();
                                            });
                                        }
                                    },
                                )
                            }),
                    )
                    .when(show_sidebar, |d| {
                        d.when_some(self.active_tab(), |d, tab| d.child(tab.sidebar.clone()))
                    })
                    .child(
                        div()
                            .id("main-column")
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .child(
                                div()
                                    .id("pdf-canvas-toolbar")
                                    .h(ui::sizes::CANVAS_TOOLBAR_HEIGHT)
                                    .w_full()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap(ui::sizes::SPACE_1)
                                    .px(ui::sizes::SPACE_2)
                                    .bg(theme.surface)
                                    .border_b_1()
                                    .border_color(toolbar_chrome_border)
                                    .child(flat_icon_button(
                                        "toolbar-first-page",
                                        Icon::PageFirst,
                                        can_first_prev,
                                        false,
                                        &theme,
                                        {
                                            let entity = cx.entity().downgrade();
                                            move |_, _, cx| {
                                                if let Some(editor) = entity.upgrade() {
                                                    editor.update(cx, |editor, cx| {
                                                        if let Some(tab) = editor.active_tab() {
                                                            tab.viewport.update(
                                                                cx,
                                                                |viewport, cx| {
                                                                    viewport.first_page(cx);
                                                                },
                                                            );
                                                        }
                                                    });
                                                }
                                            }
                                        },
                                    ))
                                    .child(flat_icon_button(
                                        "toolbar-prev-page",
                                        Icon::ChevronLeft,
                                        can_first_prev,
                                        false,
                                        &theme,
                                        {
                                            let entity = cx.entity().downgrade();
                                            move |_, _, cx| {
                                                if let Some(editor) = entity.upgrade() {
                                                    editor.update(cx, |editor, cx| {
                                                        if let Some(tab) = editor.active_tab() {
                                                            tab.viewport.update(
                                                                cx,
                                                                |viewport, cx| {
                                                                    viewport.prev_page(cx);
                                                                },
                                                            );
                                                        }
                                                    });
                                                }
                                            }
                                        },
                                    ))
                                    .child(
                                        div()
                                            .min_w(ui::sizes::PAGE_LABEL_MIN_WIDTH)
                                            .text_xs()
                                            .text_color(if can_navigate {
                                                theme.text
                                            } else {
                                                theme.text_muted
                                            })
                                            .child(page_label),
                                    )
                                    .child(flat_icon_button(
                                        "toolbar-next-page",
                                        Icon::ChevronRight,
                                        can_next_last,
                                        false,
                                        &theme,
                                        {
                                            let entity = cx.entity().downgrade();
                                            move |_, _, cx| {
                                                if let Some(editor) = entity.upgrade() {
                                                    editor.update(cx, |editor, cx| {
                                                        if let Some(tab) = editor.active_tab() {
                                                            tab.viewport.update(
                                                                cx,
                                                                |viewport, cx| {
                                                                    viewport.next_page(cx);
                                                                },
                                                            );
                                                        }
                                                    });
                                                }
                                            }
                                        },
                                    ))
                                    .child(flat_icon_button(
                                        "toolbar-last-page",
                                        Icon::PageLast,
                                        can_next_last,
                                        false,
                                        &theme,
                                        {
                                            let entity = cx.entity().downgrade();
                                            move |_, _, cx| {
                                                if let Some(editor) = entity.upgrade() {
                                                    editor.update(cx, |editor, cx| {
                                                        if let Some(tab) = editor.active_tab() {
                                                            tab.viewport.update(
                                                                cx,
                                                                |viewport, cx| {
                                                                    viewport.last_page(cx);
                                                                },
                                                            );
                                                        }
                                                    });
                                                }
                                            }
                                        },
                                    ))
                                    .child(
                                        div()
                                            .w(ui::sizes::TOOLBAR_SEPARATOR_WIDTH)
                                            .h(px(12.0))
                                            .mx(px(2.0))
                                            .bg(toolbar_separator),
                                    )
                                    .child(flat_icon_button(
                                        "toolbar-fit-page",
                                        Icon::FitPage,
                                        can_zoom,
                                        false,
                                        &theme,
                                        {
                                            let entity = cx.entity().downgrade();
                                            move |_, _, cx| {
                                                if let Some(editor) = entity.upgrade() {
                                                    editor.update(cx, |editor, cx| {
                                                        if let Some(tab) = editor.active_tab() {
                                                            tab.viewport.update(
                                                                cx,
                                                                |viewport, cx| {
                                                                    viewport.fit_page(cx);
                                                                },
                                                            );
                                                            editor.sync_zoom_input_from_active(cx);
                                                        }
                                                    });
                                                }
                                            }
                                        },
                                    ))
                                    .child(flat_icon_button(
                                        "toolbar-fit-width",
                                        Icon::FitWidth,
                                        can_zoom,
                                        false,
                                        &theme,
                                        {
                                            let entity = cx.entity().downgrade();
                                            move |_, _, cx| {
                                                if let Some(editor) = entity.upgrade() {
                                                    editor.update(cx, |editor, cx| {
                                                        if let Some(tab) = editor.active_tab() {
                                                            tab.viewport.update(
                                                                cx,
                                                                |viewport, cx| {
                                                                    viewport.fit_width(cx);
                                                                },
                                                            );
                                                            editor.sync_zoom_input_from_active(cx);
                                                        }
                                                    });
                                                }
                                            }
                                        },
                                    ))
                                    .child(
                                        div()
                                            .w(ui::sizes::TOOLBAR_SEPARATOR_WIDTH)
                                            .h(px(12.0))
                                            .mx(px(2.0))
                                            .bg(toolbar_separator),
                                    )
                                    .child(flat_icon_button(
                                        "toolbar-zoom-out",
                                        Icon::Minus,
                                        can_zoom,
                                        false,
                                        &theme,
                                        {
                                            let entity = cx.entity().downgrade();
                                            move |_, _, cx| {
                                                if let Some(editor) = entity.upgrade() {
                                                    editor.update(cx, |editor, cx| {
                                                        if let Some(tab) = editor.active_tab() {
                                                            tab.viewport.update(
                                                                cx,
                                                                |viewport, cx| {
                                                                    viewport.zoom_out(cx);
                                                                },
                                                            );
                                                            editor.sync_zoom_input_from_active(cx);
                                                        }
                                                    });
                                                }
                                            }
                                        },
                                    ))
                                    .child(zoom_combo)
                                    .child(flat_icon_button(
                                        "toolbar-zoom-in",
                                        Icon::Plus,
                                        can_zoom,
                                        false,
                                        &theme,
                                        {
                                            let entity = cx.entity().downgrade();
                                            move |_, _, cx| {
                                                if let Some(editor) = entity.upgrade() {
                                                    editor.update(cx, |editor, cx| {
                                                        if let Some(tab) = editor.active_tab() {
                                                            tab.viewport.update(
                                                                cx,
                                                                |viewport, cx| {
                                                                    viewport.zoom_in(cx);
                                                                },
                                                            );
                                                            editor.sync_zoom_input_from_active(cx);
                                                        }
                                                    });
                                                }
                                            }
                                        },
                                    ))
                                    .child(div().ml_auto()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .overflow_hidden()
                                    .bg(theme.elevated_surface)
                                    .when(self.tabs.is_empty(), {
                                        let entity = cx.entity().downgrade();
                                        let theme_clone = theme;
                                        move |d| {
                                            d.child(
                                                div()
                                                    .size_full()
                                                    .flex()
                                                    .flex_col()
                                                    .items_center()
                                                    .justify_center()
                                                    .child(text_button_with_shortcut(
                                                        "welcome-open-file",
                                                        "Open File",
                                                        "⌘O",
                                                        ButtonSize::Medium,
                                                        &theme_clone,
                                                        move |_, window, cx| {
                                                            if let Some(editor) = entity.upgrade() {
                                                                editor.update(cx, |editor, cx| {
                                                                    editor.handle_open(
                                                                        &Open, window, cx,
                                                                    );
                                                                });
                                                            }
                                                        },
                                                    )),
                                            )
                                        }
                                    })
                                    .when(!self.tabs.is_empty(), {
                                        let is_welcome = self
                                            .active_tab()
                                            .map(|t| t.is_welcome())
                                            .unwrap_or(false);
                                        let entity = cx.entity().downgrade();
                                        let theme_clone = theme;
                                        move |d| {
                                            d.when(is_welcome, |d| {
                                                d.child(
                                                    div()
                                                        .size_full()
                                                        .flex()
                                                        .flex_col()
                                                        .items_center()
                                                        .justify_center()
                                                        .child(text_button_with_shortcut(
                                                            "tab-welcome-open-file",
                                                            "Open File",
                                                            "⌘O",
                                                            ButtonSize::Medium,
                                                            &theme_clone,
                                                            move |_, window, cx| {
                                                                if let Some(editor) =
                                                                    entity.upgrade()
                                                                {
                                                                    editor.update(
                                                                        cx,
                                                                        |editor, cx| {
                                                                            editor.handle_open(
                                                                                &Open, window, cx,
                                                                            );
                                                                        },
                                                                    );
                                                                }
                                                            },
                                                        )),
                                                )
                                            })
                                            .when_some(
                                                active_viewport.filter(|_| !is_welcome),
                                                |d, vp| d.child(vp),
                                            )
                                        }
                                    }),
                            ),
                    ),
            )
    }
}
