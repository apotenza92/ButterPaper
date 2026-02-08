//! Shared custom vertical scrollbar helpers.

use gpui::{
    div, point, prelude::*, px, App, ElementId, MouseButton, MouseDownEvent, MouseUpEvent,
    ScrollHandle, Window,
};

use crate::ui::color;
use crate::ui::sizes;
use crate::Theme;
pub const SCROLLBAR_GUTTER_WIDTH: f32 = sizes::SCROLLBAR_GUTTER_WIDTH_PX;
pub const SCROLLBAR_VISUAL_WIDTH: f32 = sizes::SCROLLBAR_VISUAL_WIDTH_PX;

#[derive(Clone)]
struct ScrollbarDragToken;

struct ScrollbarDragGhost;

impl Render for ScrollbarDragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(0.0)).h(px(0.0))
    }
}

#[derive(Clone, Copy)]
pub struct ScrollbarMetrics {
    pub thumb_top: f32,
    pub thumb_height: f32,
    track_height: f32,
    max_offset: f32,
}

#[derive(Clone)]
pub struct ScrollbarController {
    scroll_handle: ScrollHandle,
    drag_offset_y: Option<f32>,
}

impl ScrollbarController {
    pub fn new() -> Self {
        Self { scroll_handle: ScrollHandle::new(), drag_offset_y: None }
    }

    pub fn handle(&self) -> ScrollHandle {
        self.scroll_handle.clone()
    }

    pub fn offset_y(&self) -> f32 {
        self.scroll_handle.offset().y.0
    }

    pub fn set_offset_y(&self, y: f32) {
        self.scroll_handle.set_offset(point(px(0.0), px(y)));
    }

    pub fn metrics(&self) -> Option<ScrollbarMetrics> {
        let viewport_height = self.scroll_handle.bounds().size.height.0;
        let child_count = self.scroll_handle.children_count();
        if child_count == 0 {
            return None;
        }

        let first = self.scroll_handle.bounds_for_item(0)?;
        let last = self.scroll_handle.bounds_for_item(child_count - 1)?;
        let content_height = (last.bottom() - first.top()).0.max(0.0);
        let max_offset = (content_height - viewport_height).max(0.0);
        if viewport_height <= 0.0 || max_offset <= 0.0 {
            return None;
        }

        let track_height = (viewport_height - sizes::SCROLLBAR_TRACK_INSET_PX * 2.0).max(1.0);
        let thumb_height = ((viewport_height / content_height) * track_height)
            .clamp(sizes::SCROLLBAR_MIN_THUMB_HEIGHT_PX, track_height);
        let offset = (-self.scroll_handle.offset().y.0).clamp(0.0, max_offset);
        let ratio = if max_offset > 0.0 { offset / max_offset } else { 0.0 };
        let thumb_top = sizes::SCROLLBAR_TRACK_INSET_PX + ratio * (track_height - thumb_height);

        Some(ScrollbarMetrics { thumb_top, thumb_height, track_height, max_offset })
    }

    pub fn start_drag(&mut self, mouse_y_window: f32) -> bool {
        let Some(metrics) = self.metrics() else {
            return false;
        };
        let viewport_top = self.scroll_handle.bounds().origin.y.0;
        let pointer_y = mouse_y_window - viewport_top;
        let drag_offset = if pointer_y >= metrics.thumb_top
            && pointer_y <= metrics.thumb_top + metrics.thumb_height
        {
            pointer_y - metrics.thumb_top
        } else {
            metrics.thumb_height / 2.0
        };
        self.drag_offset_y = Some(drag_offset);
        self.apply_drag(mouse_y_window, drag_offset)
    }

    pub fn update_drag(&mut self, mouse_y_window: f32) -> bool {
        let Some(drag_offset) = self.drag_offset_y else {
            return false;
        };
        self.apply_drag(mouse_y_window, drag_offset)
    }

    pub fn end_drag(&mut self) {
        self.drag_offset_y = None;
    }

    fn apply_drag(&mut self, mouse_y_window: f32, drag_offset: f32) -> bool {
        let Some(metrics) = self.metrics() else {
            return false;
        };
        let viewport_top = self.scroll_handle.bounds().origin.y.0;
        let movable = (metrics.track_height - metrics.thumb_height).max(1.0);
        let drag_top = (mouse_y_window - viewport_top - drag_offset)
            .clamp(sizes::SCROLLBAR_TRACK_INSET_PX, sizes::SCROLLBAR_TRACK_INSET_PX + movable);
        let ratio = (drag_top - sizes::SCROLLBAR_TRACK_INSET_PX) / movable;
        let next_offset = -ratio * metrics.max_offset;
        let prev_offset = self.scroll_handle.offset().y.0;
        self.scroll_handle.set_offset(point(px(0.0), px(next_offset)));
        (next_offset - prev_offset).abs() > 0.1
    }
}

pub fn scrollbar_gutter<Down, Up, Move>(
    id: impl Into<ElementId>,
    theme: &Theme,
    metrics: ScrollbarMetrics,
    on_mouse_down: Down,
    on_mouse_up_out: Up,
    on_drag_move: Move,
) -> impl IntoElement
where
    Down: Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    Up: Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    Move: Fn(&gpui::MouseMoveEvent, &mut Window, &mut App) + 'static,
{
    let thumb_bg = color::with_alpha(theme.text_muted, sizes::SCROLLBAR_THUMB_BG_ALPHA);
    let thumb_border = color::with_alpha(theme.border, sizes::SCROLLBAR_THUMB_BORDER_ALPHA);

    div()
        .id(id)
        .flex()
        .justify_center()
        .w(px(SCROLLBAR_GUTTER_WIDTH))
        .h_full()
        .on_mouse_down(MouseButton::Left, on_mouse_down)
        .on_drag(ScrollbarDragToken, |_, _offset, _window, cx| cx.new(|_| ScrollbarDragGhost))
        .on_drag_move::<ScrollbarDragToken>(move |event, window, cx| {
            on_drag_move(&event.event, window, cx);
        })
        .on_mouse_up_out(MouseButton::Left, on_mouse_up_out)
        .child(
            div()
                .h_full()
                .w(px(SCROLLBAR_VISUAL_WIDTH))
                .relative()
                .rounded_full()
                .bg(color::with_alpha(theme.background, sizes::SCROLLBAR_TRACK_ALPHA))
                .child(
                    div()
                        .absolute()
                        .left(px(0.0))
                        .right(px(0.0))
                        .top(px(metrics.thumb_top))
                        .h(px(metrics.thumb_height))
                        .rounded_full()
                        .bg(thumb_bg)
                        .border_1()
                        .border_color(thumb_border),
                ),
        )
}
