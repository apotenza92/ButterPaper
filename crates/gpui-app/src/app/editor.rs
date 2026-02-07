//! ButterPaper main component with tabbed document management.

use gpui::{
    div, prelude::*, px, App, Context, ExternalPaths, FocusHandle, Focusable, KeyDownEvent,
    MouseButton, MouseMoveEvent, ScrollHandle, Window,
};
use std::path::PathBuf;

use super::document::DocumentTab;
use crate::components::tab_bar::TabId as UiTabId;
use crate::components::{
    chrome_control_shell, chrome_icon_button, context_menu, icon, popover_menu, tab_item,
    text_button_with_shortcut, ButtonSize, ContextMenuItem, Icon, TabItemData,
};
use crate::settings;
use crate::sidebar::{ThumbnailSidebar, SIDEBAR_WIDTH};
use crate::ui::TypographyExt;
use crate::viewport::{PdfViewport, ViewMode, ZoomMode};
use crate::workspace::{load_preferences, TabPreferences};
use crate::{current_theme, ui, Theme};
use crate::{
    CloseTab, CloseWindow, FirstPage, FitPage, FitWidth, LastPage, NextPage, NextTab, Open,
    PrevPage, PrevTab, ResetZoom, ZoomIn, ZoomOut,
};

const MIN_ZOOM_PERCENT: u32 = 25;
const MAX_ZOOM_PERCENT: u32 = 400;
const TAB_BAR_OVERFLOW_EPSILON: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuKind {
    ButterPaper,
    File,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuCommand {
    About,
    OpenSettings,
    Quit,
    Open,
}

#[derive(Clone, Copy, Debug)]
struct ActiveViewportInfo {
    has_document: bool,
    page_count: u16,
    current_page: u16,
    zoom_level: u32,
    zoom_mode: ZoomMode,
    view_mode: ViewMode,
}

impl Default for ActiveViewportInfo {
    fn default() -> Self {
        Self {
            has_document: false,
            page_count: 0,
            current_page: 0,
            zoom_level: 100,
            zoom_mode: ZoomMode::Percent,
            view_mode: ViewMode::Continuous,
        }
    }
}

pub struct PdfEditor {
    tabs: Vec<DocumentTab>,
    active_tab_index: usize,
    focus_handle: FocusHandle,
    zoom_input_focus_handle: FocusHandle,
    page_input_focus_handle: FocusHandle,
    preferences: TabPreferences,
    /// Tracks native horizontal scrolling for the tab strip.
    tab_scroll_handle: ScrollHandle,
    /// Whether tab content currently overflows the visible tab-strip viewport.
    tab_bar_overflowing: bool,
    /// Whether overflow state should be recomputed next frame.
    tab_bar_needs_measure: bool,
    /// Guard to avoid scheduling duplicate next-frame overflow checks.
    tab_bar_measure_scheduled: bool,
    /// Last observed tab-row width (in pixels); used to trigger re-measure on resize.
    tab_bar_last_row_width: f32,
    /// Whether the thumbnail sidebar is visible for the active tab.
    thumbnail_sidebar_visible: bool,
    /// Which in-window menu is currently open.
    open_menu: Option<MenuKind>,
    /// Current text shown in the zoom combo field.
    zoom_input_text: String,
    /// Whether the zoom field is in edit mode.
    zoom_input_editing: bool,
    /// Whether the current zoom text should behave like a selected value.
    zoom_input_select_all: bool,
    /// Whether the zoom presets dropdown is open.
    zoom_preset_open: bool,
    /// Current text shown in the page input field (1-based display value).
    page_input_text: String,
    /// Whether the page field is in edit mode.
    page_input_editing: bool,
    /// Whether the current page text should behave like a selected value.
    page_input_select_all: bool,
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

fn parse_page_input_index(input: &str, page_count: u16) -> Option<u16> {
    if page_count == 0 {
        return None;
    }

    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let entered = trimmed.parse::<u16>().ok()?;
    if entered == 0 {
        return Some(0);
    }

    Some(entered.saturating_sub(1).min(page_count.saturating_sub(1)))
}

fn compute_canvas_metrics(
    viewport_width: f32,
    viewport_height: f32,
    show_sidebar: bool,
    show_in_window_menu: bool,
) -> (f32, f32) {
    let sidebar = if show_sidebar { SIDEBAR_WIDTH } else { 0.0 };
    let canvas_width = (viewport_width - ui::sizes::TOOL_RAIL_WIDTH.0 - sidebar).max(1.0);
    let menu_height = if show_in_window_menu { ui::sizes::MENU_ROW_HEIGHT.0 } else { 0.0 };
    let chrome_height = ui::sizes::TITLE_BAR_HEIGHT.0
        + menu_height
        + ui::sizes::TAB_BAR_HEIGHT.0
        + ui::sizes::CANVAS_TOOLBAR_HEIGHT.0;
    let canvas_height = (viewport_height - chrome_height).max(1.0);
    (canvas_width, canvas_height)
}

fn map_menu_command(value: &str) -> Option<MenuCommand> {
    match value {
        "app.about" => Some(MenuCommand::About),
        "app.settings" => Some(MenuCommand::OpenSettings),
        "app.quit" => Some(MenuCommand::Quit),
        "file.open" => Some(MenuCommand::Open),
        _ => None,
    }
}

fn hover_open_menu(current: Option<MenuKind>, hovered: MenuKind) -> Option<MenuKind> {
    if current.is_some() {
        Some(hovered)
    } else {
        None
    }
}

fn tab_bar_is_overflowing(max_offset_width: f32) -> bool {
    max_offset_width > TAB_BAR_OVERFLOW_EPSILON
}

impl PdfEditor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            tabs: Vec::new(),
            active_tab_index: 0,
            focus_handle: cx.focus_handle(),
            zoom_input_focus_handle: cx.focus_handle(),
            page_input_focus_handle: cx.focus_handle(),
            preferences: load_preferences(),
            tab_scroll_handle: ScrollHandle::new(),
            tab_bar_overflowing: false,
            tab_bar_needs_measure: true,
            tab_bar_measure_scheduled: false,
            tab_bar_last_row_width: 0.0,
            thumbnail_sidebar_visible: true,
            open_menu: None,
            zoom_input_text: "100%".to_string(),
            zoom_input_editing: false,
            zoom_input_select_all: false,
            zoom_preset_open: false,
            page_input_text: "0".to_string(),
            page_input_editing: false,
            page_input_select_all: false,
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
            zoom_mode: viewport.zoom_mode(),
            view_mode: viewport.view_mode(),
        }
    }

    fn sync_zoom_input_from_active(&mut self, cx: &App) {
        if self.zoom_input_editing {
            return;
        }

        let zoom = self.active_viewport_info(cx).zoom_level;
        self.zoom_input_text = format!("{}%", zoom);
    }

    fn sync_page_input_from_active(&mut self, cx: &App) {
        if self.page_input_editing {
            return;
        }

        let info = self.active_viewport_info(cx);
        self.page_input_text =
            if info.page_count > 0 { (info.current_page + 1).to_string() } else { "0".to_string() };
    }

    pub fn open_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        // Check if file is already open in a tab
        if let Some(idx) = self.tabs.iter().position(|t| t.path.as_ref() == Some(&path)) {
            self.active_tab_index = idx;
            self.reveal_active_tab();
            cx.notify();
            return;
        }

        let tab_index = self.create_tab(Some(path.clone()), cx);
        self.active_tab_index = tab_index;

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

        if let Ok(mode) = std::env::var("BUTTERPAPER_VISUAL_FIT_MODE") {
            let mode = mode.to_ascii_lowercase();
            if mode == "page" || mode == "fit-page" {
                tab.viewport.update(cx, |viewport, cx| {
                    viewport.fit_page(cx);
                });
            } else if mode == "width" || mode == "fit-width" {
                tab.viewport.update(cx, |viewport, cx| {
                    viewport.fit_width(cx);
                });
            }
        }

        self.zoom_input_editing = false;
        self.zoom_input_select_all = false;
        self.zoom_preset_open = false;
        self.sync_zoom_input_from_active(cx);
        self.page_input_editing = false;
        self.page_input_select_all = false;
        self.sync_page_input_from_active(cx);
        self.mark_tab_bar_layout_dirty();
        self.reveal_active_tab();
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

    fn mark_tab_bar_layout_dirty(&mut self) {
        self.tab_bar_needs_measure = true;
    }

    fn update_tab_bar_overflow_state(&mut self) -> bool {
        let overflowing = tab_bar_is_overflowing(self.tab_bar_overflow_width());
        let changed = overflowing != self.tab_bar_overflowing;
        self.tab_bar_overflowing = overflowing;
        changed
    }

    fn estimated_tab_content_width(&self) -> f32 {
        let tab_padding = ui::sizes::SPACE_2.0 * 2.0;
        let tab_gap = ui::sizes::SPACE_1.0;
        let close_button = ui::sizes::TAB_CLOSE_SIZE.0;
        let dirty_icon_budget = 12.0;
        let inline_new_tab_budget = 40.0;

        let mut width = 0.0;
        for tab in &self.tabs {
            let title_width = tab.title.chars().count() as f32 * 7.0;
            let estimated = title_width + tab_padding + tab_gap + close_button + dirty_icon_budget;
            width += ui::sizes::TAB_MIN_WIDTH.0.max(estimated);
        }

        if !self.tab_bar_overflowing {
            width += inline_new_tab_budget;
        }

        width
    }

    fn tab_bar_overflow_width(&self) -> f32 {
        let measured = self.tab_bar_overflow_width_from_layout();
        if measured > 0.0 {
            return measured;
        }
        (self.estimated_tab_content_width() - self.tab_bar_last_row_width).max(0.0)
    }

    fn tab_bar_overflow_width_from_layout(&self) -> f32 {
        if self.tabs.is_empty() {
            return 0.0;
        }

        let Some(first) = self.tab_scroll_handle.bounds_for_item(0) else {
            return 0.0;
        };
        let Some(last) = self.tab_scroll_handle.bounds_for_item(self.tabs.len().saturating_sub(1))
        else {
            return 0.0;
        };

        let content_width = (last.right() - first.left()).0;
        let viewport_width = self.tab_scroll_handle.bounds().size.width.0;
        (content_width - viewport_width).max(0.0)
    }

    fn schedule_tab_bar_measure(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.tab_bar_needs_measure {
            return;
        }

        self.tab_bar_needs_measure = false;
        if self.update_tab_bar_overflow_state() {
            cx.notify();
        }

        if self.tab_bar_measure_scheduled {
            return;
        }
        self.tab_bar_measure_scheduled = true;
        cx.on_next_frame(window, |this, _window, cx| {
            this.tab_bar_measure_scheduled = false;
            if this.update_tab_bar_overflow_state() {
                cx.notify();
            }
        });
    }

    fn reveal_active_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.tab_scroll_handle.scroll_to_item(self.active_tab_index);
    }

    fn sync_tab_row_width_and_mark_dirty(&mut self, width: f32) {
        if (self.tab_bar_last_row_width - width).abs() > 0.5 {
            self.tab_bar_last_row_width = width;
            self.mark_tab_bar_layout_dirty();
        }
    }

    fn select_tab(&mut self, tab_id: UiTabId, cx: &mut Context<Self>) {
        if let Some(idx) = self.tabs.iter().position(|t| t.id == tab_id) {
            self.active_tab_index = idx;
            self.zoom_input_editing = false;
            self.zoom_input_select_all = false;
            self.zoom_preset_open = false;
            self.sync_zoom_input_from_active(cx);
            self.page_input_editing = false;
            self.page_input_select_all = false;
            self.sync_page_input_from_active(cx);
            self.reveal_active_tab();
            cx.notify();
        }
    }

    fn close_tab(&mut self, tab_id: UiTabId, cx: &mut Context<Self>) {
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
            self.zoom_input_select_all = false;
            self.zoom_preset_open = false;
            self.sync_zoom_input_from_active(cx);
            self.page_input_editing = false;
            self.page_input_select_all = false;
            self.sync_page_input_from_active(cx);
            self.mark_tab_bar_layout_dirty();
            self.reveal_active_tab();
            cx.notify();
        }
    }

    fn next_tab(&mut self, cx: &mut Context<Self>) {
        if !self.tabs.is_empty() {
            self.active_tab_index = (self.active_tab_index + 1) % self.tabs.len();
            self.sync_zoom_input_from_active(cx);
            self.sync_page_input_from_active(cx);
            self.reveal_active_tab();
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
            self.sync_page_input_from_active(cx);
            self.reveal_active_tab();
            cx.notify();
        }
    }

    fn show_tab_bar(&self) -> bool {
        if self.tabs.is_empty() {
            return true;
        }

        if self.active_tab().map(|tab| tab.is_welcome()).unwrap_or(false) {
            return true;
        }

        self.preferences.show_tab_bar || self.tabs.len() > 1
    }

    fn close_transient_ui(&mut self) {
        self.open_menu = None;
        self.zoom_preset_open = false;
        self.page_input_editing = false;
        self.page_input_select_all = false;
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
            self.zoom_input_select_all = false;
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
        self.zoom_input_select_all = false;
        self.zoom_input_text = format!("{}%", value);
        cx.notify();
    }

    fn cancel_zoom_input_edit(&mut self, cx: &mut Context<Self>) {
        self.zoom_input_editing = false;
        self.zoom_input_select_all = false;
        self.sync_zoom_input_from_active(cx);
        cx.notify();
    }

    fn apply_page_input(&mut self, cx: &mut Context<Self>) {
        let page_count = self.active_viewport_info(cx).page_count;
        let Some(page_index) = parse_page_input_index(&self.page_input_text, page_count) else {
            self.page_input_editing = false;
            self.page_input_select_all = false;
            self.sync_page_input_from_active(cx);
            cx.notify();
            return;
        };

        if let Some(tab) = self.active_tab() {
            let viewport = tab.viewport.clone();
            viewport.update(cx, |viewport, cx| {
                viewport.go_to_page(page_index, cx);
            });
        }

        self.page_input_editing = false;
        self.page_input_select_all = false;
        self.sync_page_input_from_active(cx);
        cx.notify();
    }

    fn cancel_page_input_edit(&mut self, cx: &mut Context<Self>) {
        self.page_input_editing = false;
        self.page_input_select_all = false;
        self.sync_page_input_from_active(cx);
        cx.notify();
    }

    fn open_page_input_for_edit(&mut self) {
        self.page_input_editing = true;
        self.page_input_select_all = true;
        self.open_menu = None;
        self.zoom_preset_open = false;
    }

    fn open_zoom_combo_for_edit(&mut self) {
        self.zoom_input_editing = true;
        self.zoom_input_select_all = true;
        self.open_menu = None;
        self.zoom_preset_open = true;
    }

    fn finalize_zoom_preset_selection(&mut self, cx: &mut Context<Self>) {
        self.zoom_preset_open = false;
        self.zoom_input_select_all = false;
        self.sync_zoom_input_from_active(cx);
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
                if self.zoom_input_select_all {
                    self.zoom_input_text.clear();
                    self.zoom_input_select_all = false;
                } else {
                    self.zoom_input_text.pop();
                }
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
                if self.zoom_input_select_all {
                    self.zoom_input_text.clear();
                    self.zoom_input_select_all = false;
                } else if self.zoom_input_text.ends_with('%') {
                    self.zoom_input_text.pop();
                }
                if self.zoom_input_text.len() < 4 {
                    self.zoom_input_text.push(ch);
                }
                cx.notify();
                cx.stop_propagation();
            } else if ch == '%' {
                self.zoom_input_editing = true;
                if self.zoom_input_select_all {
                    self.zoom_input_text.clear();
                    self.zoom_input_select_all = false;
                }
                if !self.zoom_input_text.ends_with('%') {
                    self.zoom_input_text.push('%');
                }
                cx.notify();
                cx.stop_propagation();
            }
        }
    }

    fn handle_page_input_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();

        match key {
            "enter" => {
                self.apply_page_input(cx);
                window.focus(&self.focus_handle);
                cx.stop_propagation();
                return;
            }
            "escape" => {
                self.cancel_page_input_edit(cx);
                window.focus(&self.focus_handle);
                cx.stop_propagation();
                return;
            }
            "backspace" => {
                self.page_input_editing = true;
                if self.page_input_select_all {
                    self.page_input_text.clear();
                    self.page_input_select_all = false;
                } else {
                    self.page_input_text.pop();
                }
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
                self.page_input_editing = true;
                if self.page_input_select_all {
                    self.page_input_text.clear();
                    self.page_input_select_all = false;
                }
                if self.page_input_text.len() < 5 {
                    self.page_input_text.push(ch);
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
            MenuCommand::About => println!("ButterPaper - GPUI Edition"),
            MenuCommand::OpenSettings => settings::open_settings_window(cx),
            MenuCommand::Quit => cx.quit(),
            MenuCommand::Open => self.handle_open(&Open, window, cx),
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
        let _ = window;
        if let Some(tab) = self.active_tab() {
            let tab_id = tab.id;
            self.close_tab(tab_id, cx);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_canvas_toolbar(
        &self,
        can_first_prev: bool,
        can_next_last: bool,
        can_zoom: bool,
        fit_page_selected: bool,
        fit_width_selected: bool,
        page_view_mode: ViewMode,
        page_control: gpui::AnyElement,
        zoom_combo: gpui::AnyElement,
        theme: &Theme,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        div()
            .id("pdf-canvas-toolbar")
            .h(ui::sizes::CANVAS_TOOLBAR_HEIGHT)
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(ui::sizes::TOOLBAR_ZONE_GAP)
            .px(ui::sizes::TOOLBAR_INSET_X)
            .bg(theme.surface)
            .border_b_1()
            .border_color(ui::color::subtle_border(theme.border))
            .child(
                div()
                    .id("toolbar-left-zone")
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .justify_start()
                    .child(
                        div()
                            .id("toolbar-left-cluster")
                            .h(ui::sizes::TOOLBAR_CONTROL_SIZE)
                            .flex()
                            .items_center()
                            .gap(ui::sizes::TOOLBAR_CLUSTER_INNER_GAP)
                            .child(chrome_icon_button(
                                "toolbar-zoom-out",
                                Icon::ZoomOut,
                                "Zoom out",
                                can_zoom,
                                false,
                                theme,
                                {
                                    let entity = cx.entity().downgrade();
                                    move |_, _, cx| {
                                        if let Some(editor) = entity.upgrade() {
                                            editor.update(cx, |editor, cx| {
                                                if let Some(tab) = editor.active_tab() {
                                                    tab.viewport.update(cx, |viewport, cx| {
                                                        viewport.zoom_out(cx);
                                                    });
                                                    editor.sync_zoom_input_from_active(cx);
                                                }
                                            });
                                        }
                                    }
                                },
                            ))
                            .child(zoom_combo)
                            .child(chrome_icon_button(
                                "toolbar-zoom-in",
                                Icon::ZoomIn,
                                "Zoom in",
                                can_zoom,
                                false,
                                theme,
                                {
                                    let entity = cx.entity().downgrade();
                                    move |_, _, cx| {
                                        if let Some(editor) = entity.upgrade() {
                                            editor.update(cx, |editor, cx| {
                                                if let Some(tab) = editor.active_tab() {
                                                    tab.viewport.update(cx, |viewport, cx| {
                                                        viewport.zoom_in(cx);
                                                    });
                                                    editor.sync_zoom_input_from_active(cx);
                                                }
                                            });
                                        }
                                    }
                                },
                            ))
                            .child(
                                div()
                                    .id("toolbar-separator-zoom-fit")
                                    .px(ui::sizes::SPACE_1)
                                    .text_ui_body()
                                    .text_color(theme.text_muted)
                                    .child("|"),
                            )
                            .child(chrome_icon_button(
                                "toolbar-fit-page",
                                Icon::FitPage,
                                "Fit page",
                                can_zoom,
                                fit_page_selected,
                                theme,
                                {
                                    let entity = cx.entity().downgrade();
                                    move |_, _, cx| {
                                        if let Some(editor) = entity.upgrade() {
                                            editor.update(cx, |editor, cx| {
                                                if let Some(tab) = editor.active_tab() {
                                                    tab.viewport.update(cx, |viewport, cx| {
                                                        viewport.fit_page(cx);
                                                    });
                                                    editor.sync_zoom_input_from_active(cx);
                                                }
                                            });
                                        }
                                    }
                                },
                            ))
                            .child(chrome_icon_button(
                                "toolbar-fit-width",
                                Icon::FitWidth,
                                "Fit width",
                                can_zoom,
                                fit_width_selected,
                                theme,
                                {
                                    let entity = cx.entity().downgrade();
                                    move |_, _, cx| {
                                        if let Some(editor) = entity.upgrade() {
                                            editor.update(cx, |editor, cx| {
                                                if let Some(tab) = editor.active_tab() {
                                                    tab.viewport.update(cx, |viewport, cx| {
                                                        viewport.fit_width(cx);
                                                    });
                                                    editor.sync_zoom_input_from_active(cx);
                                                }
                                            });
                                        }
                                    }
                                },
                            ))
                            .child(
                                div()
                                    .id("toolbar-separator-fit-view")
                                    .px(ui::sizes::SPACE_1)
                                    .text_ui_body()
                                    .text_color(theme.text_muted)
                                    .child("|"),
                            )
                            .child(chrome_icon_button(
                                "toolbar-view-single-page",
                                Icon::ViewSinglePage,
                                "Single page view",
                                can_zoom,
                                page_view_mode == ViewMode::SinglePage,
                                theme,
                                {
                                    let entity = cx.entity().downgrade();
                                    move |_, _, cx| {
                                        if let Some(editor) = entity.upgrade() {
                                            editor.update(cx, |editor, cx| {
                                                if let Some(tab) = editor.active_tab() {
                                                    tab.viewport.update(cx, |viewport, cx| {
                                                        viewport.set_view_mode(
                                                            ViewMode::SinglePage,
                                                            cx,
                                                        );
                                                    });
                                                }
                                            });
                                        }
                                    }
                                },
                            ))
                            .child(chrome_icon_button(
                                "toolbar-view-continuous",
                                Icon::ViewContinuous,
                                "Continuous view",
                                can_zoom,
                                page_view_mode == ViewMode::Continuous,
                                theme,
                                {
                                    let entity = cx.entity().downgrade();
                                    move |_, _, cx| {
                                        if let Some(editor) = entity.upgrade() {
                                            editor.update(cx, |editor, cx| {
                                                if let Some(tab) = editor.active_tab() {
                                                    tab.viewport.update(cx, |viewport, cx| {
                                                        viewport.set_view_mode(
                                                            ViewMode::Continuous,
                                                            cx,
                                                        );
                                                    });
                                                }
                                            });
                                        }
                                    }
                                },
                            )),
                    ),
            )
            .child(
                div()
                    .id("toolbar-right-zone")
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .justify_end()
                    .child(
                        div()
                            .id("toolbar-right-cluster")
                            .h(ui::sizes::TOOLBAR_CONTROL_SIZE)
                            .flex()
                            .items_center()
                            .gap(ui::sizes::TOOLBAR_CLUSTER_INNER_GAP)
                            .child(chrome_icon_button(
                                "toolbar-first-page",
                                Icon::PageFirst,
                                "First page",
                                can_first_prev,
                                false,
                                theme,
                                {
                                    let entity = cx.entity().downgrade();
                                    move |_, _, cx| {
                                        if let Some(editor) = entity.upgrade() {
                                            editor.update(cx, |editor, cx| {
                                                if let Some(tab) = editor.active_tab() {
                                                    tab.viewport.update(cx, |viewport, cx| {
                                                        viewport.first_page(cx);
                                                    });
                                                }
                                            });
                                        }
                                    }
                                },
                            ))
                            .child(chrome_icon_button(
                                "toolbar-prev-page",
                                Icon::ChevronLeft,
                                "Previous page",
                                can_first_prev,
                                false,
                                theme,
                                {
                                    let entity = cx.entity().downgrade();
                                    move |_, _, cx| {
                                        if let Some(editor) = entity.upgrade() {
                                            editor.update(cx, |editor, cx| {
                                                if let Some(tab) = editor.active_tab() {
                                                    tab.viewport.update(cx, |viewport, cx| {
                                                        viewport.prev_page(cx);
                                                    });
                                                }
                                            });
                                        }
                                    }
                                },
                            ))
                            .child(page_control)
                            .child(chrome_icon_button(
                                "toolbar-next-page",
                                Icon::ChevronRight,
                                "Next page",
                                can_next_last,
                                false,
                                theme,
                                {
                                    let entity = cx.entity().downgrade();
                                    move |_, _, cx| {
                                        if let Some(editor) = entity.upgrade() {
                                            editor.update(cx, |editor, cx| {
                                                if let Some(tab) = editor.active_tab() {
                                                    tab.viewport.update(cx, |viewport, cx| {
                                                        viewport.next_page(cx);
                                                    });
                                                }
                                            });
                                        }
                                    }
                                },
                            ))
                            .child(chrome_icon_button(
                                "toolbar-last-page",
                                Icon::PageLast,
                                "Last page",
                                can_next_last,
                                false,
                                theme,
                                {
                                    let entity = cx.entity().downgrade();
                                    move |_, _, cx| {
                                        if let Some(editor) = entity.upgrade() {
                                            editor.update(cx, |editor, cx| {
                                                if let Some(tab) = editor.active_tab() {
                                                    tab.viewport.update(cx, |viewport, cx| {
                                                        viewport.last_page(cx);
                                                    });
                                                }
                                            });
                                        }
                                    }
                                },
                            )),
                    ),
            )
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
                    TabItemData::new(tab_id, title, is_active, is_dirty, !doc_tab.is_welcome()),
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
                                let _ = window;
                                editor.close_tab(tab_id, cx);
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
            .justify_center()
            .items_center()
            .text_ui_body()
            .text_color(if is_open { theme.text } else { theme.text_muted })
            .when(is_open, {
                let hover = theme.element_hover;
                move |d| d.bg(hover)
            })
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
            .on_mouse_move(cx.listener(move |this, _: &MouseMoveEvent, _window, cx| {
                let next = hover_open_menu(this.open_menu, kind);
                if this.open_menu != next {
                    this.open_menu = next;
                    cx.notify();
                }
            }))
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

        div().child(popover_menu(trigger, menu, is_open, px(-7.0)))
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use gpui::{point, px, size, AppContext as _, ScrollDelta, ScrollWheelEvent, TestAppContext};

    use super::{
        compute_canvas_metrics, hover_open_menu, map_menu_command,
        next_active_tab_index_after_close, parse_page_input_index, parse_zoom_input_percent,
        tab_bar_is_overflowing, MenuCommand, MenuKind, TAB_BAR_OVERFLOW_EPSILON,
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
    fn parse_page_input_converts_to_zero_based_index() {
        assert_eq!(parse_page_input_index("1", 10), Some(0));
        assert_eq!(parse_page_input_index("4", 10), Some(3));
        assert_eq!(parse_page_input_index(" 9 ", 10), Some(8));
    }

    #[test]
    fn parse_page_input_clamps_and_rejects_invalid() {
        assert_eq!(parse_page_input_index("0", 10), Some(0));
        assert_eq!(parse_page_input_index("99", 10), Some(9));
        assert_eq!(parse_page_input_index("", 10), None);
        assert_eq!(parse_page_input_index("abc", 10), None);
        assert_eq!(parse_page_input_index("1", 0), None);
    }

    #[test]
    fn canvas_metrics_account_for_sidebar() {
        let (w_open, h_open) = compute_canvas_metrics(1200.0, 900.0, true, true);
        let (w_closed, h_closed) = compute_canvas_metrics(1200.0, 900.0, false, true);

        assert!(w_closed > w_open);
        assert_eq!(h_open, h_closed);
    }

    #[test]
    fn canvas_metrics_account_for_menu_visibility() {
        let (_w_with_menu, h_with_menu) = compute_canvas_metrics(1200.0, 900.0, false, true);
        let (_w_without_menu, h_without_menu) = compute_canvas_metrics(1200.0, 900.0, false, false);

        assert!(h_without_menu > h_with_menu);
    }

    #[test]
    fn menu_command_mapping_is_stable() {
        assert_eq!(map_menu_command("app.about"), Some(MenuCommand::About));
        assert_eq!(map_menu_command("app.settings"), Some(MenuCommand::OpenSettings));
        assert_eq!(map_menu_command("app.quit"), Some(MenuCommand::Quit));
        assert_eq!(map_menu_command("file.open"), Some(MenuCommand::Open));
        assert_eq!(map_menu_command("view.fit_page"), None);
        assert_eq!(map_menu_command("unknown"), None);
    }

    #[test]
    fn hover_switches_menu_only_when_one_is_open() {
        assert_eq!(hover_open_menu(None, MenuKind::File), None);
        assert_eq!(
            hover_open_menu(Some(MenuKind::File), MenuKind::ButterPaper),
            Some(MenuKind::ButterPaper)
        );
    }

    #[test]
    fn tab_bar_overflow_threshold_is_stable() {
        assert!(!tab_bar_is_overflowing(TAB_BAR_OVERFLOW_EPSILON));
        assert!(tab_bar_is_overflowing(TAB_BAR_OVERFLOW_EPSILON + 0.01));
    }

    #[test]
    fn tab_bar_overflow_state_transitions_follow_max_offset() {
        let mut overflowing = false;

        let next = tab_bar_is_overflowing(14.0);
        assert_ne!(overflowing, next);
        overflowing = next;
        assert!(overflowing);

        let next = tab_bar_is_overflowing(0.0);
        assert_ne!(overflowing, next);
        overflowing = next;
        assert!(!overflowing);
    }

    #[gpui::test]
    fn zoom_combo_open_sets_edit_and_select_all_state(cx: &mut TestAppContext) {
        let (editor, cx) = cx.add_window_view(|_, cx| super::PdfEditor::new(cx));

        cx.update(|_, app| {
            editor.update(app, |editor, _cx| {
                editor.zoom_input_editing = false;
                editor.zoom_input_select_all = false;
                editor.zoom_preset_open = false;
                editor.open_zoom_combo_for_edit();
            });
        });

        let (editing, select_all, preset_open) = cx.read_entity(&editor, |editor, _| {
            (editor.zoom_input_editing, editor.zoom_input_select_all, editor.zoom_preset_open)
        });
        assert!(editing);
        assert!(select_all);
        assert!(preset_open);
    }

    #[gpui::test]
    fn finalizing_zoom_preset_clears_selection_state(cx: &mut TestAppContext) {
        let (editor, cx) = cx.add_window_view(|_, cx| super::PdfEditor::new(cx));

        cx.update(|_, app| {
            editor.update(app, |editor, cx| {
                editor.zoom_input_editing = true;
                editor.zoom_input_select_all = true;
                editor.zoom_preset_open = true;
                editor.finalize_zoom_preset_selection(cx);
            });
        });

        let (select_all, preset_open) = cx.read_entity(&editor, |editor, _| {
            (editor.zoom_input_select_all, editor.zoom_preset_open)
        });
        assert!(!select_all);
        assert!(!preset_open);
    }

    fn seed_long_tabs(
        editor: &mut super::PdfEditor,
        cx: &mut gpui::Context<super::PdfEditor>,
        n: usize,
    ) {
        for index in 0..n {
            let tab_index = editor.create_tab(None, cx);
            editor.tabs[tab_index].title = format!(
                "Very long document title {} - this should never truncate in the tab bar",
                index
            );
            editor.active_tab_index = tab_index;
        }
        editor.mark_tab_bar_layout_dirty();
        editor.reveal_active_tab();
        cx.notify();
    }

    #[gpui::test]
    fn tab_bar_overflow_mode_switches_with_tab_count(cx: &mut TestAppContext) {
        let (editor, cx) = cx.add_window_view(|_, cx| super::PdfEditor::new(cx));
        cx.simulate_resize(size(px(520.0), px(700.0)));

        cx.update(|_, app| {
            editor.update(app, |editor, cx| {
                let tab_index = editor.create_tab(None, cx);
                editor.tabs[tab_index].title = "Short".to_string();
                editor.active_tab_index = tab_index;
                editor.mark_tab_bar_layout_dirty();
                editor.reveal_active_tab();
                cx.notify();
            });
        });
        cx.run_until_parked();

        let not_overflowing = cx.read_entity(&editor, |editor, _| !editor.tab_bar_overflowing);
        assert!(not_overflowing);

        cx.update(|_, app| {
            editor.update(app, |editor, cx| seed_long_tabs(editor, cx, 18));
        });
        cx.run_until_parked();

        let overflowing = cx.read_entity(&editor, |editor, _| editor.tab_bar_overflowing);
        assert!(overflowing);
    }

    #[gpui::test]
    fn vertical_wheel_scrolls_horizontally_in_tab_bar(cx: &mut TestAppContext) {
        let (editor, cx) = cx.add_window_view(|_, cx| super::PdfEditor::new(cx));
        cx.simulate_resize(size(px(520.0), px(700.0)));

        cx.update(|_, app| {
            editor.update(app, |editor, cx| seed_long_tabs(editor, cx, 24));
        });
        cx.run_until_parked();

        let before = cx.read_entity(&editor, |editor, _| editor.tab_scroll_handle.offset().x.0);
        cx.simulate_event(ScrollWheelEvent {
            position: point(px(120.0), px(72.0)),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-160.0))),
            ..Default::default()
        });
        cx.run_until_parked();
        let after = cx.read_entity(&editor, |editor, _| editor.tab_scroll_handle.offset().x.0);

        assert_ne!(before, after);
    }

    #[gpui::test]
    fn activating_far_tab_auto_reveals_it(cx: &mut TestAppContext) {
        let (editor, cx) = cx.add_window_view(|_, cx| super::PdfEditor::new(cx));
        cx.simulate_resize(size(px(520.0), px(700.0)));

        cx.update(|_, app| {
            editor.update(app, |editor, cx| seed_long_tabs(editor, cx, 20));
        });
        cx.run_until_parked();

        cx.update(|_, app| {
            editor.update(app, |editor, cx| {
                editor.tab_scroll_handle.set_offset(point(px(0.0), px(0.0)));
                let last_tab_id = editor.tabs.last().expect("seeded tabs").id;
                editor.select_tab(last_tab_id, cx);
            });
        });
        cx.run_until_parked();

        let offset = cx.read_entity(&editor, |editor, _| editor.tab_scroll_handle.offset().x.0);
        assert!(offset < 0.0);
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
        let show_in_window_menu = true;
        let active_is_welcome = self.active_tab().map(|tab| tab.is_welcome()).unwrap_or(false);

        let viewport_size = window.viewport_size();
        self.sync_tab_row_width_and_mark_dirty(viewport_size.width.0);
        let show_sidebar =
            self.thumbnail_sidebar_visible && self.active_tab().is_some() && !active_is_welcome;
        let (canvas_width, canvas_height) = compute_canvas_metrics(
            viewport_size.width.0,
            viewport_size.height.0,
            show_sidebar,
            show_in_window_menu,
        );
        if show_tab_bar {
            self.schedule_tab_bar_measure(window, cx);
        }

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
        if !self.page_input_editing {
            self.page_input_text = if viewport_info.page_count > 0 {
                (viewport_info.current_page + 1).to_string()
            } else {
                "0".to_string()
            };
        }
        let total_page_label = if viewport_info.page_count > 0 {
            viewport_info.page_count.to_string()
        } else {
            "0".to_string()
        };

        let can_navigate = viewport_info.has_document && viewport_info.page_count > 0;
        let is_first = viewport_info.current_page == 0;
        let is_last = viewport_info.page_count > 0
            && viewport_info.current_page >= viewport_info.page_count.saturating_sub(1);

        let can_first_prev = can_navigate && !is_first;
        let can_next_last = can_navigate && !is_last;
        let can_zoom = viewport_info.has_document;
        let fit_page_selected = can_zoom && viewport_info.zoom_mode == ZoomMode::FitPage;
        let fit_width_selected = can_zoom && viewport_info.zoom_mode == ZoomMode::FitWidth;
        let page_view_mode = viewport_info.view_mode;

        let toolbar_chrome_border = ui::color::subtle_border(theme.border);
        let tab_row_bg = theme.surface;
        let tab_row_border = ui::color::subtle_border(theme.border);

        let app_items = vec![
            ContextMenuItem::new("app.about", "About ButterPaper"),
            ContextMenuItem::new("app.settings", "Settings...").shortcut("⌘,"),
            ContextMenuItem::new("app.quit", "Quit ButterPaper").shortcut("⌘Q"),
        ];
        let file_items = vec![ContextMenuItem::new("file.open", "Open…").shortcut("⌘O")];

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
            let zoom_focus_for_input = self.zoom_input_focus_handle.clone();
            let entity_for_toggle = cx.entity().downgrade();
            let zoom_focus_for_toggle = self.zoom_input_focus_handle.clone();
            let zoom_input_selected = self.zoom_input_editing && self.zoom_input_select_all;

            chrome_control_shell(
                "zoom-combo",
                can_zoom,
                false,
                Some(ui::sizes::ZOOM_COMBO_MIN_WIDTH),
                &theme,
                div()
                    .h_full()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .id("zoom-combo-input")
                            .track_focus(&self.zoom_input_focus_handle)
                            .h_full()
                            .flex()
                            .flex_1()
                            .justify_center()
                            .items_center()
                            .px(ui::sizes::SPACE_1)
                            .text_ui_body()
                            .text_center()
                            .text_color(if can_zoom { theme.text } else { theme.text_muted })
                            .child(
                                div()
                                    .id("zoom-combo-value")
                                    .h(ui::sizes::CONTROL_HEIGHT_COMPACT)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .px(ui::sizes::SPACE_1)
                                    .rounded(ui::sizes::RADIUS_SM)
                                    .when(zoom_input_selected, |d| d.bg(theme.element_selected))
                                    .child(self.zoom_input_text.clone()),
                            )
                            .on_click(move |_, window, cx| {
                                if let Some(editor) = entity_for_click.upgrade() {
                                    editor.update(cx, |editor, cx| {
                                        if !can_zoom {
                                            return;
                                        }
                                        editor.open_zoom_combo_for_edit();
                                        cx.notify();
                                        window.focus(&zoom_focus_for_input);
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
                            .text_ui_body()
                            .text_color(theme.text_muted)
                            .child(icon(Icon::ChevronDown, 12.0, theme.text_muted))
                            .on_click(move |_, window, cx| {
                                if let Some(editor) = entity_for_toggle.upgrade() {
                                    editor.update(cx, |editor, cx| {
                                        if !can_zoom {
                                            return;
                                        }
                                        editor.open_zoom_combo_for_edit();
                                        cx.notify();
                                        window.focus(&zoom_focus_for_toggle);
                                    });
                                }
                            }),
                    ),
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
                        editor.finalize_zoom_preset_selection(cx);
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
                    this.apply_zoom_input(cx);
                    changed = false;
                }
                if changed {
                    cx.notify();
                }
            }))
            .child(popover_menu(
                zoom_combo_trigger,
                zoom_combo_menu,
                self.zoom_preset_open,
                px(0.0),
            ))
            .into_any_element();

        let page_input_selected = self.page_input_editing && self.page_input_select_all;
        let page_control = div()
            .id("toolbar-page-control")
            .h(ui::sizes::TOOLBAR_CONTROL_SIZE)
            .px(ui::sizes::PAGE_LABEL_HORIZONTAL_PADDING)
            .flex()
            .items_center()
            .justify_center()
            .gap(ui::sizes::SPACE_1)
            .on_mouse_down_out(cx.listener(|this, _event, _window, cx| {
                if this.page_input_editing {
                    this.apply_page_input(cx);
                }
            }))
            .child({
                let entity_for_page_click = cx.entity().downgrade();
                let page_focus_for_input = self.page_input_focus_handle.clone();
                chrome_control_shell(
                    "toolbar-page-input-shell",
                    can_navigate,
                    false,
                    Some(ui::sizes::PAGE_LABEL_MIN_WIDTH),
                    &theme,
                    div()
                        .id("toolbar-page-input")
                        .track_focus(&self.page_input_focus_handle)
                        .h_full()
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .px(ui::sizes::SPACE_1)
                        .text_ui_body()
                        .text_center()
                        .text_color(if can_navigate { theme.text } else { theme.text_muted })
                        .child(
                            div()
                                .id("toolbar-page-input-value")
                                .h(ui::sizes::CONTROL_HEIGHT_COMPACT)
                                .flex()
                                .items_center()
                                .justify_center()
                                .px(ui::sizes::SPACE_1)
                                .rounded(ui::sizes::RADIUS_SM)
                                .when(page_input_selected, |d| d.bg(theme.element_selected))
                                .child(self.page_input_text.clone()),
                        )
                        .on_click(move |_, window, cx| {
                            if let Some(editor) = entity_for_page_click.upgrade() {
                                editor.update(cx, |editor, cx| {
                                    if !can_navigate {
                                        return;
                                    }
                                    editor.open_page_input_for_edit();
                                    cx.notify();
                                    window.focus(&page_focus_for_input);
                                });
                            }
                        })
                        .on_key_down(cx.listener(Self::handle_page_input_key)),
                )
            })
            .child(
                div()
                    .id("toolbar-page-separator")
                    .text_ui_body()
                    .text_color(theme.text_muted)
                    .child("/"),
            )
            .child(
                div()
                    .id("toolbar-page-total")
                    .text_ui_body()
                    .text_color(if can_navigate { theme.text } else { theme.text_muted })
                    .child(total_page_label),
            )
            .into_any_element();

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
                    if this.page_input_editing {
                        this.cancel_page_input_edit(cx);
                        window.focus(&this.focus_handle);
                        cx.stop_propagation();
                        return;
                    }
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
            .when(show_in_window_menu, |d| {
                d.child(
                    div()
                        .id("app-menu-row")
                        .on_mouse_down_out(cx.listener(|this, _event, _window, cx| {
                            if this.open_menu.is_some() {
                                this.open_menu = None;
                                cx.notify();
                            }
                        }))
                        .h(ui::sizes::MENU_ROW_HEIGHT)
                        .w_full()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(ui::sizes::SPACE_0)
                        .pl(ui::sizes::SPACE_1)
                        .pr(ui::sizes::SPACE_1)
                        .bg(theme.surface)
                        .border_b_1()
                        .border_color(theme.border)
                        .child(self.render_menu_entry(
                            "menu-app",
                            "ButterPaper",
                            MenuKind::ButterPaper,
                            app_items,
                            &theme,
                            cx,
                        ))
                        .child(self.render_menu_entry(
                            "menu-file",
                            "File",
                            MenuKind::File,
                            file_items,
                            &theme,
                            cx,
                        )),
                )
            })
            .child(
                div()
                    .id("app-tab-row")
                    .h(ui::sizes::TAB_BAR_HEIGHT)
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .bg(tab_row_bg)
                    .border_b_1()
                    .border_color(tab_row_border)
                    .when(show_tab_bar, |d| {
                        let tab_items = self.render_tab_items(&theme, cx);
                        let tab_bar_overflowing = self.tab_bar_overflowing;
                        let tab_scroll_handle = self.tab_scroll_handle.clone();
                        let render_new_tab_button = {
                            let entity = cx.entity().downgrade();
                            move || {
                                let entity_for_click = entity.clone();
                                div()
                                    .id("new-tab")
                                    .h(ui::sizes::TAB_HEIGHT)
                                    .px(ui::sizes::SPACE_0)
                                    .flex()
                                    .items_center()
                                    .flex_shrink_0()
                                    .child(chrome_icon_button(
                                        "new-tab-button",
                                        Icon::Plus,
                                        "Open file",
                                        true,
                                        false,
                                        &theme,
                                        move |_, window, cx| {
                                            if let Some(editor) = entity_for_click.upgrade() {
                                                editor.update(cx, |editor, cx| {
                                                    editor.handle_open(&Open, window, cx);
                                                });
                                            }
                                        },
                                    ))
                            }
                        };
                        d.child(
                            div()
                                .id("tabs-scroll")
                                .flex()
                                .flex_row()
                                .flex_nowrap()
                                .h(ui::sizes::TAB_HEIGHT)
                                .min_w_0()
                                .flex_1()
                                .items_center()
                                .px(ui::sizes::SPACE_1)
                                .overflow_x_scroll()
                                .track_scroll(&tab_scroll_handle)
                                .children(tab_items)
                                .when(!tab_bar_overflowing, |d| {
                                    d.child(
                                        div()
                                            .id("new-tab-inline")
                                            .h(ui::sizes::TAB_HEIGHT)
                                            .flex_shrink_0()
                                            .child(render_new_tab_button()),
                                    )
                                })
                                .child({
                                    let entity = cx.entity().downgrade();
                                    div()
                                        .id("tab-bar-empty")
                                        .h(ui::sizes::TAB_HEIGHT)
                                        .when(!tab_bar_overflowing, |d| {
                                            d.flex_1().on_mouse_down(
                                                MouseButton::Left,
                                                move |event, window, cx| {
                                                    if event.click_count == 2 {
                                                        if let Some(editor) = entity.upgrade() {
                                                            editor.update(cx, |editor, cx| {
                                                                editor
                                                                    .handle_open(&Open, window, cx);
                                                            });
                                                        }
                                                    }
                                                },
                                            )
                                        })
                                        .when(tab_bar_overflowing, |d| d.w(px(0.0)).flex_shrink_0())
                                }),
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
                                    .text_ui_body()
                                    .text_color(theme.text)
                                    .child(title),
                            )
                        }
                    })
                    .when(show_tab_bar && self.tab_bar_overflowing, |d| {
                        let entity = cx.entity().downgrade();
                        d.child(
                            div()
                                .id("new-tab-pinned")
                                .h(ui::sizes::TAB_HEIGHT)
                                .ml(ui::sizes::SPACE_1)
                                .flex()
                                .items_center()
                                .child(
                                    div()
                                        .id("new-tab")
                                        .h(ui::sizes::TAB_HEIGHT)
                                        .px(ui::sizes::SPACE_0)
                                        .flex()
                                        .items_center()
                                        .child(chrome_icon_button(
                                            "new-tab-button",
                                            Icon::Plus,
                                            "Open file",
                                            true,
                                            false,
                                            &theme,
                                            move |_, window, cx| {
                                                if let Some(editor) = entity.upgrade() {
                                                    editor.update(cx, |editor, cx| {
                                                        editor.handle_open(&Open, window, cx);
                                                    });
                                                }
                                            },
                                        )),
                                ),
                        )
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
                            .pt(ui::sizes::TOOL_RAIL_TOP_INSET)
                            .bg(theme.surface)
                            .border_r_1()
                            .border_color(toolbar_chrome_border)
                            .child({
                                let entity = cx.entity().downgrade();
                                chrome_icon_button(
                                    "tool-rail-toggle-thumbnails",
                                    Icon::PanelLeft,
                                    "Toggle thumbnails",
                                    !active_is_welcome,
                                    show_sidebar,
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
                            .child(self.render_canvas_toolbar(
                                can_first_prev,
                                can_next_last,
                                can_zoom,
                                fit_page_selected,
                                fit_width_selected,
                                page_view_mode,
                                page_control,
                                zoom_combo,
                                &theme,
                                cx,
                            ))
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
