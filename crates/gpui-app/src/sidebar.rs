//! Thumbnail sidebar component
//!
//! Displays page thumbnails for navigation.

#![allow(dead_code)]
#![allow(clippy::type_complexity)]

use crate::components::{scrollbar_gutter, ScrollbarController};
use crate::current_theme;
use crate::ui::color;
use butterpaper_render::PdfDocument;
use gpui::{div, img, prelude::*, px, FocusHandle, Focusable, ImageSource, MouseMoveEvent};
use image::{ImageBuffer, Rgba};
use smallvec::SmallVec;
use std::sync::Arc;

/// Thumbnail width in pixels
const THUMBNAIL_WIDTH: u32 = 120;

/// Sidebar width in pixels
pub const SIDEBAR_WIDTH: f32 = 220.0;

/// Rendered thumbnail for a page
#[derive(Clone)]
pub struct Thumbnail {
    pub page_index: u16,
    pub width: u32,
    pub height: u32,
    pub image: Arc<gpui::RenderImage>,
}

/// Thumbnail sidebar state
pub struct ThumbnailSidebar {
    /// Document reference (shared with viewport)
    document: Option<Arc<PdfDocument>>,
    /// Rendered thumbnails
    thumbnails: Vec<Thumbnail>,
    /// Currently selected page
    selected_page: u16,
    /// Focus handle
    focus_handle: FocusHandle,
    /// Shared scrollbar controller for custom gutter + drag.
    scrollbar: ScrollbarController,
    /// Callback entity for page selection
    on_page_select: Option<Box<dyn Fn(u16, &mut gpui::App) + 'static>>,
}

impl ThumbnailSidebar {
    pub fn new(cx: &mut gpui::Context<Self>) -> Self {
        Self {
            document: None,
            thumbnails: Vec::new(),
            selected_page: 0,
            focus_handle: cx.focus_handle(),
            scrollbar: ScrollbarController::new(),
            on_page_select: None,
        }
    }

    /// Set the document and render thumbnails
    pub fn set_document(&mut self, doc: Option<Arc<PdfDocument>>, cx: &mut gpui::Context<Self>) {
        self.document = doc;
        self.thumbnails.clear();
        self.selected_page = 0;
        self.render_thumbnails();
        cx.notify();
    }

    /// Set callback for page selection
    pub fn set_on_page_select<F>(&mut self, callback: F)
    where
        F: Fn(u16, &mut gpui::App) + 'static,
    {
        self.on_page_select = Some(Box::new(callback));
    }

    /// Update selected page (called from viewport)
    pub fn set_selected_page(&mut self, page: u16, cx: &mut gpui::Context<Self>) {
        if self.selected_page != page {
            self.selected_page = page;
            cx.notify();
        }
    }

    /// Render all thumbnails
    fn render_thumbnails(&mut self) {
        let Some(doc) = &self.document else { return };

        for page_index in 0..doc.page_count() {
            if let Some(thumb) = self.render_thumbnail(doc, page_index) {
                self.thumbnails.push(thumb);
            }
        }
    }

    /// Render a single page thumbnail
    fn render_thumbnail(&self, doc: &PdfDocument, page_index: u16) -> Option<Thumbnail> {
        let page = doc.get_page(page_index).ok()?;
        let page_width = page.width().value;
        let page_height = page.height().value;

        // Calculate height maintaining aspect ratio
        let scale = THUMBNAIL_WIDTH as f32 / page_width;
        let thumb_height = (page_height * scale) as u32;

        // Render at thumbnail size
        let rgba_pixels = doc.render_page_rgba(page_index, THUMBNAIL_WIDTH, thumb_height).ok()?;

        // Convert RGBA to BGRA
        let mut bgra_pixels = rgba_pixels;
        for pixel in bgra_pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }

        // Create image
        let buffer =
            ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(THUMBNAIL_WIDTH, thumb_height, bgra_pixels)?;
        let frame = image::Frame::new(buffer);
        let render_image = Arc::new(gpui::RenderImage::new(SmallVec::from_elem(frame, 1)));

        Some(Thumbnail {
            page_index,
            width: THUMBNAIL_WIDTH,
            height: thumb_height,
            image: render_image,
        })
    }

    fn start_scrollbar_drag(&mut self, mouse_y_window: f32, cx: &mut gpui::Context<Self>) {
        if self.scrollbar.start_drag(mouse_y_window) {
            cx.notify();
        }
    }

    fn update_scrollbar_drag(&mut self, mouse_y_window: f32, cx: &mut gpui::Context<Self>) {
        if self.scrollbar.update_drag(mouse_y_window) {
            cx.notify();
        }
    }

    fn end_scrollbar_drag(&mut self) {
        self.scrollbar.end_drag();
    }
}

impl Focusable for ThumbnailSidebar {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ThumbnailSidebar {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let theme = current_theme(window, cx);
        let selected_page = self.selected_page;
        let thumbnails = self.thumbnails.clone();
        let text_muted = theme.text_muted;
        let element_hover = theme.element_hover;
        let selected_bg = theme.element_selected;
        let selected_border = color::subtle_border(theme.accent);
        let scroll_handle = self.scrollbar.handle();
        let scrollbar = self.scrollbar.metrics();
        let entity = cx.entity().downgrade();

        div()
            .id("thumbnail-sidebar")
            .flex()
            .flex_row()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .bg(theme.surface)
            // No right border - content column provides left border for clean corner connection
            .overflow_hidden()
            .on_mouse_move({
                let entity = entity.clone();
                move |event: &MouseMoveEvent, _window, cx| {
                    if let Some(sidebar) = entity.upgrade() {
                        sidebar.update(cx, |this, cx| {
                            this.update_scrollbar_drag(event.position.y.0, cx);
                        });
                    }
                }
            })
            .on_mouse_up(gpui::MouseButton::Left, {
                let entity = entity.clone();
                move |_event: &gpui::MouseUpEvent, _window, cx| {
                    if let Some(sidebar) = entity.upgrade() {
                        sidebar.update(cx, |this, _cx| {
                            this.end_scrollbar_drag();
                        });
                    }
                }
            })
            // Scrollable thumbnail list
            .child(
                div()
                    .id("thumbnail-scroll-container")
                    .h_full()
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&scroll_handle)
                    .child(div().id("thumbnail-list").flex().flex_col().gap_2().p_2().children(
                        thumbnails.into_iter().enumerate().map(move |(idx, thumb)| {
                            let page_index = thumb.page_index;
                            let is_selected = page_index == selected_page;

                            div()
                                .id(("thumbnail", idx))
                                .flex()
                                .flex_col()
                                .items_center()
                                .p_1()
                                .rounded_lg()
                                .border_1()
                                .border_color(gpui::Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.0 })
                                .cursor_pointer()
                                .when(is_selected, move |s| {
                                    s.bg(selected_bg).border_color(selected_border).shadow_sm()
                                })
                                .hover(move |s| if is_selected { s } else { s.bg(element_hover) })
                                .on_click(cx.listener(move |this, _, _window, cx| {
                                    this.selected_page = page_index;
                                    if let Some(callback) = &this.on_page_select {
                                        callback(page_index, cx);
                                    }
                                    cx.notify();
                                }))
                                .child(
                                    div().rounded_sm().overflow_hidden().shadow_sm().child(
                                        img(ImageSource::Render(thumb.image.clone()))
                                            .w(px(thumb.width as f32))
                                            .h(px(thumb.height as f32)),
                                    ),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(if is_selected {
                                            theme.text
                                        } else {
                                            text_muted
                                        })
                                        .mt_1()
                                        .child(format!("{}", page_index + 1)),
                                )
                        }),
                    )),
            )
            .when_some(scrollbar, |d, metrics| {
                d.child(scrollbar_gutter(
                    "thumbnail-scrollbar-gutter",
                    &theme,
                    metrics,
                    {
                        let entity = entity.clone();
                        move |event: &gpui::MouseDownEvent, _window, cx| {
                            if let Some(sidebar) = entity.upgrade() {
                                sidebar.update(cx, |this, cx| {
                                    this.start_scrollbar_drag(event.position.y.0, cx);
                                });
                            }
                        }
                    },
                    {
                        let entity = entity.clone();
                        move |_event: &gpui::MouseUpEvent, _window, cx| {
                            if let Some(sidebar) = entity.upgrade() {
                                sidebar.update(cx, |this, _cx| {
                                    this.end_scrollbar_drag();
                                });
                            }
                        }
                    },
                    {
                        let entity = entity.clone();
                        move |event: &gpui::MouseMoveEvent, _window, cx| {
                            if let Some(sidebar) = entity.upgrade() {
                                sidebar.update(cx, |this, cx| {
                                    this.update_scrollbar_drag(event.position.y.0, cx);
                                });
                            }
                        }
                    },
                ))
            })
    }
}
