//! PDF Viewport component
//!
//! Displays rendered PDF pages with continuous vertical scroll
//! and GPU texture caching.

#![allow(clippy::type_complexity)]
#![allow(clippy::arc_with_non_send_sync)]

use crate::cache::{create_render_image, CacheKey, PageCache};
use crate::current_theme;
use butterpaper_render::PdfDocument;
use gpui::{
    div, img, prelude::*, px, FocusHandle, Focusable, ImageSource, MouseMoveEvent, Pixels, Point,
    ScrollDelta, ScrollWheelEvent,
};
use std::path::PathBuf;
use std::sync::Arc;

/// Gap between pages in pixels
const PAGE_GAP: f32 = 20.0;

/// Buffer above/below viewport for pre-rendering
const RENDER_BUFFER: f32 = 400.0;

/// Maximum pages to render per frame (limits jank during fast scroll)
const MAX_RENDERS_PER_FRAME: usize = 3;

/// Minimum and maximum supported zoom percentages.
const MIN_ZOOM_PERCENT: u32 = 25;
const MAX_ZOOM_PERCENT: u32 = 400;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoomMode {
    Percent,
    FitWidth,
    FitPage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PageNavTarget {
    First,
    Prev,
    Next,
    Last,
}

fn clamp_zoom(zoom: u32) -> u32 {
    zoom.clamp(MIN_ZOOM_PERCENT, MAX_ZOOM_PERCENT)
}

fn fit_width_percent(canvas_width: f32, page_width: f32) -> u32 {
    if canvas_width <= 0.0 || page_width <= 0.0 {
        return 100;
    }

    let usable_width = (canvas_width - PAGE_GAP * 2.0).max(1.0);
    clamp_zoom(((usable_width / page_width) * 100.0).round() as u32)
}

fn fit_page_percent(
    canvas_width: f32,
    canvas_height: f32,
    page_width: f32,
    page_height: f32,
) -> u32 {
    if canvas_width <= 0.0 || canvas_height <= 0.0 || page_width <= 0.0 || page_height <= 0.0 {
        return 100;
    }

    let usable_width = (canvas_width - PAGE_GAP * 2.0).max(1.0);
    let usable_height = (canvas_height - PAGE_GAP * 2.0).max(1.0);
    let width_ratio = usable_width / page_width;
    let height_ratio = usable_height / page_height;
    clamp_zoom((width_ratio.min(height_ratio) * 100.0).round() as u32)
}

fn resolve_page_nav_target(current_page: u16, page_count: u16, target: PageNavTarget) -> u16 {
    if page_count == 0 {
        return 0;
    }

    let max_index = page_count - 1;
    match target {
        PageNavTarget::First => 0,
        PageNavTarget::Prev => current_page.saturating_sub(1),
        PageNavTarget::Next => (current_page + 1).min(max_index),
        PageNavTarget::Last => max_index,
    }
}

/// Rendered page ready for display
#[derive(Clone)]
#[allow(dead_code)]
struct DisplayPage {
    page_index: u16,
    width: u32,
    height: u32,
    y_offset: f32,
    image: Arc<gpui::RenderImage>,
}

/// Page layout info (before rendering)
#[derive(Clone)]
struct PageLayout {
    page_index: u16,
    width: f32,
    height: f32,
    y_offset: f32,
}

/// PDF Viewport state
pub struct PdfViewport {
    /// Loaded PDF document
    document: Option<Arc<PdfDocument>>,
    /// Effective zoom level (percentage, e.g. 100 = 100%)
    pub zoom_level: u32,
    /// How zoom level should react to canvas size changes.
    zoom_mode: ZoomMode,
    /// Scroll offset Y (for continuous scroll)
    scroll_y: f32,
    /// Page layouts (computed on load/zoom)
    page_layouts: Vec<PageLayout>,
    /// Total document height
    total_height: f32,
    /// GPU texture cache
    cache: PageCache,
    /// Pages ready for display
    display_pages: Vec<DisplayPage>,
    /// Viewport height (for visibility calculation)
    viewport_height: f32,
    /// Canvas width used for fit calculations.
    canvas_width: f32,
    /// Canvas height used for fit calculations.
    canvas_height: f32,
    /// Is dragging (for pan)
    is_dragging: bool,
    /// Last mouse position during drag
    last_drag_pos: Option<Point<Pixels>>,
    /// Focus handle for keyboard input
    focus_handle: FocusHandle,
    /// Callback for page change
    on_page_change: Option<Box<dyn Fn(u16, &mut gpui::App) + 'static>>,
    /// Pending pages to render (for deferred rendering)
    pending_renders: Vec<PageLayout>,
    /// Display scale factor (for Retina support)
    scale_factor: f32,
}

impl PdfViewport {
    pub fn new(cx: &mut gpui::Context<Self>) -> Self {
        Self {
            document: None,
            zoom_level: 100,
            zoom_mode: ZoomMode::Percent,
            scroll_y: 0.0,
            page_layouts: Vec::new(),
            total_height: 0.0,
            cache: PageCache::new(),
            display_pages: Vec::new(),
            viewport_height: 768.0,
            canvas_width: 1024.0,
            canvas_height: 768.0,
            is_dragging: false,
            last_drag_pos: None,
            focus_handle: cx.focus_handle(),
            on_page_change: None,
            pending_renders: Vec::new(),
            scale_factor: 1.0, // Updated from window.scale_factor() on first render
        }
    }

    /// Set callback for page changes
    pub fn set_on_page_change<F>(&mut self, callback: F)
    where
        F: Fn(u16, &mut gpui::App) + 'static,
    {
        self.on_page_change = Some(Box::new(callback));
    }

    /// Load a PDF from file path (synchronous - blocks UI)
    #[allow(dead_code)]
    pub fn load_pdf(&mut self, path: PathBuf, cx: &mut gpui::Context<Self>) -> Result<(), String> {
        match PdfDocument::open(&path) {
            Ok(doc) => {
                let doc = Arc::new(doc);
                self.set_document(doc, cx);
                Ok(())
            }
            Err(e) => Err(format!("Failed to load PDF: {}", e)),
        }
    }

    /// Set the document (used after async loading completes)
    pub fn set_document(&mut self, doc: Arc<PdfDocument>, cx: &mut gpui::Context<Self>) {
        self.document = Some(doc);
        self.scroll_y = 0.0;
        self.cache.clear();
        self.pending_renders.clear();
        self.compute_layout();

        match self.zoom_mode {
            ZoomMode::FitWidth => {
                self.apply_fit_width_zoom();
            }
            ZoomMode::FitPage => {
                self.apply_fit_page_zoom();
            }
            ZoomMode::Percent => {}
        }

        self.update_visible_pages();
        cx.notify();
    }

    /// Set canvas metrics used by fit calculations and visibility.
    pub fn set_canvas_metrics(&mut self, width: f32, height: f32, cx: &mut gpui::Context<Self>) {
        let width = width.max(1.0);
        let height = height.max(1.0);
        let changed =
            (self.canvas_width - width).abs() > 0.5 || (self.canvas_height - height).abs() > 0.5;

        if !changed {
            return;
        }

        self.canvas_width = width;
        self.canvas_height = height;
        self.viewport_height = height;

        let zoom_changed = match self.zoom_mode {
            ZoomMode::FitWidth => self.apply_fit_width_zoom(),
            ZoomMode::FitPage => self.apply_fit_page_zoom(),
            ZoomMode::Percent => false,
        };

        if !zoom_changed {
            self.clamp_scroll();
            self.update_visible_pages();
        }

        cx.notify();
    }

    /// Get the document (for sharing with sidebar)
    #[allow(dead_code)]
    pub fn document(&self) -> Option<Arc<PdfDocument>> {
        self.document.clone()
    }

    pub fn has_document(&self) -> bool {
        self.document.is_some()
    }

    /// Get page count
    pub fn page_count(&self) -> u16 {
        self.document.as_ref().map(|d| d.page_count()).unwrap_or(0)
    }

    /// Get current page based on scroll position (0-based)
    pub fn current_page(&self) -> u16 {
        let center_y = self.scroll_y + self.viewport_height / 2.0;
        for layout in &self.page_layouts {
            if center_y >= layout.y_offset && center_y < layout.y_offset + layout.height {
                return layout.page_index;
            }
        }
        self.page_layouts.last().map(|l| l.page_index).unwrap_or(0)
    }

    /// Get current page (1-based for display)
    #[allow(dead_code)]
    pub fn current_page_display(&self) -> u16 {
        self.current_page() + 1
    }

    /// Scroll to a specific page
    pub fn go_to_page(&mut self, page: u16, cx: &mut gpui::Context<Self>) {
        if let Some(layout) = self.page_layouts.get(page as usize) {
            self.scroll_y = layout.y_offset;
            self.clamp_scroll();
            self.update_visible_pages();
            cx.notify();
        }
    }

    pub fn first_page(&mut self, cx: &mut gpui::Context<Self>) {
        if self.page_count() > 0 {
            self.go_to_page(
                resolve_page_nav_target(
                    self.current_page(),
                    self.page_count(),
                    PageNavTarget::First,
                ),
                cx,
            );
        }
    }

    /// Go to next page
    pub fn next_page(&mut self, cx: &mut gpui::Context<Self>) {
        let current = self.current_page();
        let target = resolve_page_nav_target(current, self.page_count(), PageNavTarget::Next);
        if target != current {
            self.go_to_page(target, cx);
        }
    }

    /// Go to previous page
    pub fn prev_page(&mut self, cx: &mut gpui::Context<Self>) {
        let current = self.current_page();
        let target = resolve_page_nav_target(current, self.page_count(), PageNavTarget::Prev);
        if target != current {
            self.go_to_page(target, cx);
        }
    }

    pub fn last_page(&mut self, cx: &mut gpui::Context<Self>) {
        let current = self.current_page();
        let target = resolve_page_nav_target(current, self.page_count(), PageNavTarget::Last);
        if target != current {
            self.go_to_page(target, cx);
        }
    }

    /// Set zoom level in percent mode.
    pub fn set_zoom(&mut self, zoom: u32, cx: &mut gpui::Context<Self>) {
        self.zoom_mode = ZoomMode::Percent;
        if self.set_zoom_internal(zoom) {
            cx.notify();
        }
    }

    pub fn reset_zoom(&mut self, cx: &mut gpui::Context<Self>) {
        self.zoom_mode = ZoomMode::Percent;
        if self.set_zoom_internal(100) {
            cx.notify();
        }
    }

    pub fn fit_width(&mut self, cx: &mut gpui::Context<Self>) {
        let mode_changed = self.zoom_mode != ZoomMode::FitWidth;
        self.zoom_mode = ZoomMode::FitWidth;
        let zoom_changed = self.apply_fit_width_zoom();
        if mode_changed || zoom_changed {
            cx.notify();
        }
    }

    pub fn fit_page(&mut self, cx: &mut gpui::Context<Self>) {
        let mode_changed = self.zoom_mode != ZoomMode::FitPage;
        self.zoom_mode = ZoomMode::FitPage;
        let zoom_changed = self.apply_fit_page_zoom();
        if mode_changed || zoom_changed {
            cx.notify();
        }
    }

    /// Zoom in by 25%
    pub fn zoom_in(&mut self, cx: &mut gpui::Context<Self>) {
        self.zoom_mode = ZoomMode::Percent;
        if self.set_zoom_internal(self.zoom_level + 25) {
            cx.notify();
        }
    }

    /// Zoom out by 25%
    pub fn zoom_out(&mut self, cx: &mut gpui::Context<Self>) {
        self.zoom_mode = ZoomMode::Percent;
        if self.set_zoom_internal(self.zoom_level.saturating_sub(25)) {
            cx.notify();
        }
    }

    fn current_page_size_points(&self) -> Option<(f32, f32)> {
        let doc = self.document.as_ref()?;
        let page_index = self.current_page();
        let page = doc.get_page(page_index).ok()?;
        Some((page.width().value, page.height().value))
    }

    fn apply_fit_width_zoom(&mut self) -> bool {
        let Some((page_width, _)) = self.current_page_size_points() else {
            return false;
        };
        let fit = fit_width_percent(self.canvas_width, page_width);
        self.set_zoom_internal(fit)
    }

    fn apply_fit_page_zoom(&mut self) -> bool {
        let Some((page_width, page_height)) = self.current_page_size_points() else {
            return false;
        };
        let fit = fit_page_percent(self.canvas_width, self.canvas_height, page_width, page_height);
        self.set_zoom_internal(fit)
    }

    fn set_zoom_internal(&mut self, zoom: u32) -> bool {
        let new_zoom = clamp_zoom(zoom);
        if new_zoom == self.zoom_level {
            return false;
        }

        // Preserve relative scroll position
        let scroll_ratio =
            if self.total_height > 0.0 { self.scroll_y / self.total_height } else { 0.0 };

        self.zoom_level = new_zoom;
        self.compute_layout();

        // Restore relative position
        self.scroll_y = scroll_ratio * self.total_height;
        self.clamp_scroll();
        self.update_visible_pages();
        true
    }

    /// Compute page layouts
    fn compute_layout(&mut self) {
        self.page_layouts.clear();
        self.total_height = 0.0;

        let Some(doc) = &self.document else { return };
        let zoom_factor = self.zoom_level as f32 / 100.0;

        let mut y_offset = PAGE_GAP;

        for page_index in 0..doc.page_count() {
            if let Ok(page) = doc.get_page(page_index) {
                let width = page.width().value * zoom_factor;
                let height = page.height().value * zoom_factor;

                self.page_layouts.push(PageLayout { page_index, width, height, y_offset });

                y_offset += height + PAGE_GAP;
            }
        }

        self.total_height = y_offset;
    }

    /// Update visible pages - uses cache and renders missing pages
    fn update_visible_pages(&mut self) {
        let Some(doc) = self.document.clone() else {
            self.display_pages.clear();
            return;
        };

        let visible_start = self.scroll_y - RENDER_BUFFER;
        let visible_end = self.scroll_y + self.viewport_height + RENDER_BUFFER;

        let mut new_display = Vec::new();
        let mut pages_to_render = Vec::new();

        // First pass: collect cached pages and identify what needs rendering
        for layout in &self.page_layouts {
            let page_end = layout.y_offset + layout.height;
            let is_visible = page_end >= visible_start && layout.y_offset <= visible_end;

            if !is_visible {
                continue;
            }

            let cache_key = CacheKey::new(layout.page_index, self.zoom_level);

            if let Some((image, width, height)) = self.cache.get(cache_key) {
                new_display.push(DisplayPage {
                    page_index: layout.page_index,
                    width,
                    height,
                    y_offset: layout.y_offset,
                    image,
                });
            } else {
                pages_to_render.push(layout.clone());
            }
        }

        // Second pass: render missing pages (limit per frame to avoid jank)
        let renders_this_frame = pages_to_render.len().min(MAX_RENDERS_PER_FRAME);
        for layout in pages_to_render.iter().take(renders_this_frame) {
            if let Some(display_page) = self.render_and_cache(&doc, layout) {
                new_display.push(display_page);
            }
        }

        // Store remaining pages for deferred rendering
        self.pending_renders = pages_to_render.into_iter().skip(renders_this_frame).collect();

        // Sort by y_offset for proper display order
        new_display.sort_by(|a, b| a.y_offset.partial_cmp(&b.y_offset).unwrap());

        self.display_pages = new_display;
    }

    /// Process pending renders (called on subsequent frames)
    fn process_pending_renders(&mut self, cx: &mut gpui::Context<Self>) {
        if self.pending_renders.is_empty() {
            return;
        }

        let Some(doc) = self.document.clone() else {
            return;
        };

        let visible_start = self.scroll_y - RENDER_BUFFER;
        let visible_end = self.scroll_y + self.viewport_height + RENDER_BUFFER;

        // Filter to only still-visible pages
        let still_visible: Vec<_> = self
            .pending_renders
            .drain(..)
            .filter(|layout| {
                let page_end = layout.y_offset + layout.height;
                page_end >= visible_start && layout.y_offset <= visible_end
            })
            .collect();

        let renders_this_frame = still_visible.len().min(MAX_RENDERS_PER_FRAME);
        let mut rendered_any = false;

        for layout in still_visible.iter().take(renders_this_frame) {
            if let Some(display_page) = self.render_and_cache(&doc, layout) {
                // Insert in sorted position
                let pos = self
                    .display_pages
                    .iter()
                    .position(|p| p.y_offset > display_page.y_offset)
                    .unwrap_or(self.display_pages.len());
                self.display_pages.insert(pos, display_page);
                rendered_any = true;
            }
        }

        // Keep remaining for next frame
        self.pending_renders = still_visible.into_iter().skip(renders_this_frame).collect();

        if rendered_any || !self.pending_renders.is_empty() {
            cx.notify();
        }
    }

    /// Render a page and add to cache
    fn render_and_cache(&mut self, doc: &PdfDocument, layout: &PageLayout) -> Option<DisplayPage> {
        // Render at physical pixel size (logical * scale_factor) for Retina sharpness
        let render_width = (layout.width * self.scale_factor) as u32;
        let render_height = (layout.height * self.scale_factor) as u32;

        let rgba_pixels =
            doc.render_page_rgba(layout.page_index, render_width, render_height).ok()?;

        let image = create_render_image(rgba_pixels, render_width, render_height)?;

        // Store logical dimensions for display (GPUI will handle the scaling)
        let display_width = layout.width as u32;
        let display_height = layout.height as u32;

        let cache_key = CacheKey::new(layout.page_index, self.zoom_level);
        self.cache.insert(cache_key, image.clone(), display_width, display_height);

        Some(DisplayPage {
            page_index: layout.page_index,
            width: display_width,
            height: display_height,
            y_offset: layout.y_offset,
            image,
        })
    }

    /// Handle scroll for panning
    fn handle_scroll(&mut self, delta: ScrollDelta, cx: &mut gpui::Context<Self>) {
        let old_page = self.current_page();

        match delta {
            ScrollDelta::Pixels(point) => {
                self.scroll_y -= point.y.0;
            }
            ScrollDelta::Lines(point) => {
                self.scroll_y -= point.y * 40.0;
            }
        }
        self.clamp_scroll();
        self.update_visible_pages();

        // Notify page change
        let new_page = self.current_page();
        if new_page != old_page {
            if let Some(callback) = &self.on_page_change {
                callback(new_page, cx);
            }

            if matches!(self.zoom_mode, ZoomMode::FitWidth | ZoomMode::FitPage) {
                match self.zoom_mode {
                    ZoomMode::FitWidth => {
                        let _ = self.apply_fit_width_zoom();
                    }
                    ZoomMode::FitPage => {
                        let _ = self.apply_fit_page_zoom();
                    }
                    ZoomMode::Percent => {}
                }
            }
        }

        cx.notify();
    }

    /// Start dragging for pan
    fn start_drag(&mut self, pos: Point<Pixels>, cx: &mut gpui::Context<Self>) {
        self.is_dragging = true;
        self.last_drag_pos = Some(pos);
        cx.notify();
    }

    /// Update drag position
    fn update_drag(&mut self, pos: Point<Pixels>, cx: &mut gpui::Context<Self>) {
        if self.is_dragging {
            let old_page = self.current_page();

            if let Some(last_pos) = self.last_drag_pos {
                self.scroll_y -= (pos.y - last_pos.y).0;
                self.clamp_scroll();
                self.update_visible_pages();
            }
            self.last_drag_pos = Some(pos);

            // Notify page change
            let new_page = self.current_page();
            if new_page != old_page {
                if let Some(callback) = &self.on_page_change {
                    callback(new_page, cx);
                }
            }

            cx.notify();
        }
    }

    /// End dragging
    fn end_drag(&mut self, cx: &mut gpui::Context<Self>) {
        self.is_dragging = false;
        self.last_drag_pos = None;
        cx.notify();
    }

    /// Clamp scroll to valid bounds
    fn clamp_scroll(&mut self) {
        self.scroll_y = self.scroll_y.max(0.0);
        let max_scroll = (self.total_height - self.viewport_height).max(0.0);
        self.scroll_y = self.scroll_y.min(max_scroll);
    }
}

impl Focusable for PdfViewport {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PdfViewport {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        // Update scale factor for Retina support
        let new_scale = window.scale_factor();
        if (new_scale - self.scale_factor).abs() > 0.01 {
            self.scale_factor = new_scale;
            // Clear cache when scale changes - pages need re-rendering
            self.cache.clear();
            self.display_pages.clear();
            self.update_visible_pages();
        }

        // Process any pending renders
        self.process_pending_renders(cx);

        let theme = current_theme(window, cx);
        let has_document = self.document.is_some();
        let display_pages = self.display_pages.clone();
        let scroll_y = self.scroll_y;
        let total_height = self.total_height;

        div()
            .id("pdf-viewport")
            .key_context("PdfViewport")
            .track_focus(&self.focus_handle)
            .flex()
            .flex_1()
            .size_full()
            .bg(theme.elevated_surface)
            .overflow_hidden()
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _window, cx| {
                this.handle_scroll(event.delta, cx);
            }))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, _window, cx| {
                    this.start_drag(event.position, cx);
                }),
            )
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, _event: &gpui::MouseUpEvent, _window, cx| {
                    this.end_drag(cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                if this.is_dragging {
                    this.update_drag(event.position, cx);
                }
            }))
            .child(if has_document {
                // Container for all pages
                div()
                    .absolute()
                    .w_full()
                    .h(px(total_height))
                    .top(px(-scroll_y))
                    .children(display_pages.into_iter().map(|page| {
                        div()
                            .absolute()
                            .top(px(page.y_offset))
                            .w_full()
                            .flex()
                            .justify_center()
                            .child(
                                div().shadow_sm().child(
                                    img(ImageSource::Render(page.image.clone()))
                                        .w(px(page.width as f32))
                                        .h(px(page.height as f32)),
                                ),
                            )
                    }))
                    .into_any_element()
            } else {
                // Empty state - matches Zed's empty editor appearance
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .text_color(theme.text_muted)
                    .child("No PDF loaded. Use File > Open or ⌘O to open a PDF.")
                    .into_any_element()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{fit_page_percent, fit_width_percent, resolve_page_nav_target, PageNavTarget};

    #[test]
    fn fit_width_clamps_to_zoom_limits() {
        assert_eq!(fit_width_percent(1000.0, 500.0), 192);
        assert_eq!(fit_width_percent(10_000.0, 10.0), 400);
        assert_eq!(fit_width_percent(10.0, 10_000.0), 25);
    }

    #[test]
    fn fit_page_uses_smallest_ratio() {
        let zoom = fit_page_percent(1000.0, 800.0, 400.0, 2000.0);
        assert_eq!(zoom, 38);
    }

    #[test]
    fn page_navigation_targets_are_bounded() {
        assert_eq!(resolve_page_nav_target(0, 0, PageNavTarget::First), 0);
        assert_eq!(resolve_page_nav_target(0, 1, PageNavTarget::Prev), 0);
        assert_eq!(resolve_page_nav_target(0, 5, PageNavTarget::Next), 1);
        assert_eq!(resolve_page_nav_target(4, 5, PageNavTarget::Next), 4);
        assert_eq!(resolve_page_nav_target(2, 5, PageNavTarget::Last), 4);
    }
}
