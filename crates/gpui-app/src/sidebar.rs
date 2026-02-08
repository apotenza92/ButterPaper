//! Thumbnail sidebar component
//!
//! Uses windowed, lazy thumbnail rendering instead of eager full-document render.

#![allow(dead_code)]
#![allow(clippy::type_complexity)]

use crate::cache::{
    create_render_image, ByteLruCache, CacheBudget, CachedImage, MemoryPressureState,
    RenderCacheKey,
};
use crate::components::{scrollbar_gutter, ScrollbarController};
use crate::current_theme;
use crate::preview_cache::SharedPreviewCache;
use crate::ui::{color, sizes};
use butterpaper_render::{PdfDocument, RenderQuality};
use gpui::{div, img, prelude::*, px, FocusHandle, Focusable, ImageSource, MouseMoveEvent};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Display width for thumbnail cards in the sidebar.
const THUMBNAIL_WIDTH: u32 = 120;
/// Overscan rows around the estimated visible window.
const THUMBNAIL_OVERSCAN_ROWS: usize = 8;
/// Request HQ target dimensions and let `LqThumb` downscale to display size.
const THUMBNAIL_RENDER_MULTIPLIER: u32 = 4;
/// Keep thumbnail rendering low-priority relative to viewport upgrades.
const MAX_THUMBNAIL_INFLIGHT_ACTIVE: usize = 1;
const MAX_THUMBNAIL_INFLIGHT_IDLE: usize = 2;
const MAX_THUMBNAIL_INFLIGHT_BOOTSTRAP: usize = 4;
/// Cap queued thumbnail requests so we do not create an unbounded backlog.
const MAX_QUEUED_THUMBNAILS: usize = 24;
const ACTIVE_SCROLL_THUMBNAIL_DISPATCH_INTERVAL: Duration = Duration::from_millis(200);
const THUMBNAIL_HOTSET_RADIUS: i32 = 1;
const JUMP_PREEMPT_MIN_DELTA: usize = 12;
const JUMP_FORCE_DISPATCH_WINDOW: Duration = Duration::from_millis(300);
const PREWARM_BOOTSTRAP_WINDOW: Duration = Duration::from_secs(2);
const PREWARM_QUEUE_TARGET: usize = 16;

#[derive(Clone, Debug, Default)]
pub struct ThumbnailPerfSnapshot {
    pub current_thumbnail_decoded_bytes: u64,
    pub current_thumbnail_texture_bytes: u64,
    pub peak_thumbnail_decoded_bytes: u64,
    pub peak_thumbnail_texture_bytes: u64,
    pub visible_blank_frames: u64,
    pub visible_frame_samples: u64,
}

#[derive(Clone, Copy)]
struct ThumbnailSpec {
    page_index: u16,
    width: u32,
    height: u32,
}

#[derive(Clone)]
struct ThumbnailCell {
    page_index: u16,
    width: u32,
    height: u32,
    image: Option<Arc<gpui::RenderImage>>,
    is_skeleton: bool,
}

#[derive(Clone, Copy)]
struct QueuedThumbnail {
    page_index: u16,
    generation: u64,
}

/// Thumbnail sidebar state.
pub struct ThumbnailSidebar {
    /// Document reference (shared with viewport).
    document: Option<Arc<PdfDocument>>,
    /// Per-page thumbnail layout specs.
    thumbnail_specs: Vec<ThumbnailSpec>,
    /// Per-row top offsets used for exact visible window mapping.
    row_tops: Vec<f32>,
    /// Per-row heights used for exact visible window mapping.
    row_heights: Vec<f32>,
    /// Byte-bounded thumbnail cache.
    thumbnail_cache: ByteLruCache,
    /// Shared ultra-LQ preview cache consumed by sidebar + viewport.
    preview_cache: Arc<Mutex<SharedPreviewCache>>,
    /// Pages currently rendering.
    inflight_pages: HashMap<u16, u64>,
    /// Pending pages to render in priority order.
    queued_pages: VecDeque<QueuedThumbnail>,
    /// De-duplication set for pending pages.
    queued_page_set: HashSet<u16>,
    /// Stable document fingerprint for cache keys.
    doc_fingerprint: u64,
    /// Approximate row height used to estimate visible index range.
    average_row_height: f32,
    /// Currently selected page.
    selected_page: u16,
    /// Focus handle.
    focus_handle: FocusHandle,
    /// Shared scrollbar controller for custom gutter + drag.
    scrollbar: ScrollbarController,
    /// Callback entity for page selection.
    on_page_select: Option<Arc<dyn Fn(u16, &mut gpui::App) + 'static>>,
    /// Memory pressure fed by viewport/controller.
    pressure_state: MemoryPressureState,
    /// Whether viewport is actively scrolling.
    active_scroll: bool,
    /// Last queue dispatch timestamp.
    last_dispatch_at: Instant,
    /// Peak decoded bytes observed in thumbnail cache for current document session.
    peak_thumbnail_decoded_bytes: u64,
    /// Peak texture bytes observed in thumbnail cache for current document session.
    peak_thumbnail_texture_bytes: u64,
    /// Counts strict-visible frames where at least one thumbnail had no image.
    visible_blank_frames: u64,
    /// Counts strict-visible sampling frames.
    visible_frame_samples: u64,
    /// Number of currently visible thumbnail slots without an image.
    visible_missing_count: usize,
    /// Last known visible-center index for jump detection.
    last_visible_center: Option<usize>,
    /// Generation token for queue preemption.
    thumbnail_generation: u64,
    /// Temporary dispatch bypass window after deep jumps.
    force_immediate_dispatch_until: Instant,
    /// Document-open bootstrap window to rapidly seed shared ultra-LQ previews.
    prewarm_fast_until: Instant,
    /// Number of pages left in the background prewarm permutation.
    prewarm_remaining: usize,
    /// Next page index in prewarm permutation.
    prewarm_next_index: usize,
    /// Coprime stride for prewarm permutation.
    prewarm_stride: usize,
}

impl ThumbnailSidebar {
    pub fn new(cx: &mut gpui::Context<Self>) -> Self {
        let preview_budget =
            SharedPreviewCache::preview_budget_bytes(CacheBudget::adaptive().max_bytes);
        let preview_cache = Arc::new(Mutex::new(SharedPreviewCache::new(preview_budget)));
        Self::new_with_preview_cache(preview_cache, cx)
    }

    pub fn new_with_preview_cache(
        preview_cache: Arc<Mutex<SharedPreviewCache>>,
        cx: &mut gpui::Context<Self>,
    ) -> Self {
        Self {
            document: None,
            thumbnail_specs: Vec::new(),
            row_tops: Vec::new(),
            row_heights: Vec::new(),
            thumbnail_cache: ByteLruCache::new(CacheBudget::adaptive().thumbnail_bytes),
            preview_cache,
            inflight_pages: HashMap::new(),
            queued_pages: VecDeque::new(),
            queued_page_set: HashSet::new(),
            doc_fingerprint: 0,
            average_row_height: 150.0,
            selected_page: 0,
            focus_handle: cx.focus_handle(),
            scrollbar: ScrollbarController::new(),
            on_page_select: None,
            pressure_state: MemoryPressureState::Normal,
            active_scroll: false,
            last_dispatch_at: Instant::now(),
            peak_thumbnail_decoded_bytes: 0,
            peak_thumbnail_texture_bytes: 0,
            visible_blank_frames: 0,
            visible_frame_samples: 0,
            visible_missing_count: 0,
            last_visible_center: None,
            thumbnail_generation: 1,
            force_immediate_dispatch_until: Instant::now(),
            prewarm_fast_until: Instant::now(),
            prewarm_remaining: 0,
            prewarm_next_index: 0,
            prewarm_stride: 1,
        }
    }

    /// Set the document and reset thumbnail state.
    pub fn set_document(&mut self, doc: Option<Arc<PdfDocument>>, cx: &mut gpui::Context<Self>) {
        self.document = doc;
        self.selected_page = 0;
        self.inflight_pages.clear();
        self.queued_pages.clear();
        self.queued_page_set.clear();
        self.thumbnail_cache.clear();
        self.thumbnail_specs.clear();
        self.row_tops.clear();
        self.row_heights.clear();
        self.average_row_height = 150.0;
        self.last_dispatch_at = Instant::now();
        self.peak_thumbnail_decoded_bytes = 0;
        self.peak_thumbnail_texture_bytes = 0;
        self.visible_blank_frames = 0;
        self.visible_frame_samples = 0;
        self.visible_missing_count = 0;
        self.last_visible_center = None;
        self.thumbnail_generation = self.thumbnail_generation.wrapping_add(1).max(1);
        self.force_immediate_dispatch_until = Instant::now();
        self.prewarm_fast_until = Instant::now();
        self.prewarm_remaining = 0;
        self.prewarm_next_index = 0;
        self.prewarm_stride = 1;

        if let Some(document) = self.document.clone() {
            self.doc_fingerprint = Arc::as_ptr(&document) as usize as u64;
            self.build_thumbnail_specs(&document);
            self.initialize_prewarm_plan();
        } else {
            self.doc_fingerprint = 0;
        }

        self.trim_thumbnail_cache_by_distance();
        self.schedule_visible_thumbnails(cx);
        cx.notify();
    }

    pub fn set_performance_state(
        &mut self,
        pressure_state: MemoryPressureState,
        active_scroll: bool,
        cx: &mut gpui::Context<Self>,
    ) {
        if self.pressure_state == pressure_state && self.active_scroll == active_scroll {
            return;
        }
        self.pressure_state = pressure_state;
        self.active_scroll = active_scroll;
        self.trim_thumbnail_cache_by_distance();
        self.schedule_visible_thumbnails(cx);
        cx.notify();
    }

    /// Set callback for page selection.
    pub fn set_on_page_select<F>(&mut self, callback: F)
    where
        F: Fn(u16, &mut gpui::App) + 'static,
    {
        self.on_page_select = Some(Arc::new(callback));
    }

    /// Update selected page (called from viewport).
    pub fn set_selected_page(&mut self, page: u16, cx: &mut gpui::Context<Self>) {
        if self.selected_page != page {
            self.selected_page = page;
        }
        // Keep selected page warm even if offscreen.
        self.trim_thumbnail_cache_by_distance();
        self.enqueue_thumbnail_render(page, true);
        self.pump_thumbnail_queue(cx);
        cx.notify();
    }

    fn build_thumbnail_specs(&mut self, doc: &PdfDocument) {
        let mut total_row_height = 0.0f32;
        let mut count = 0usize;
        let mut row_top = 0.0f32;

        for page_index in 0..doc.page_count() {
            let (page_width, page_height) = if let Ok(page) = doc.page_dimensions(page_index) {
                (page.width, page.height)
            } else {
                (612.0, 792.0)
            };

            let scale = THUMBNAIL_WIDTH as f32 / page_width.max(1.0);
            let thumb_height = (page_height * scale).round().max(1.0) as u32;
            self.thumbnail_specs.push(ThumbnailSpec {
                page_index,
                width: THUMBNAIL_WIDTH,
                height: thumb_height,
            });

            let row_height = Self::estimated_row_height(thumb_height as f32);
            self.row_tops.push(row_top);
            self.row_heights.push(row_height);
            row_top += row_height;
            total_row_height += row_height;
            count += 1;
        }

        if count > 0 {
            self.average_row_height = (total_row_height / count as f32).max(96.0);
        }
    }

    fn estimated_row_height(thumb_height: f32) -> f32 {
        // Card vertical chrome + list gap approximation.
        sizes::SPACE_2.0 * 2.0 + thumb_height + sizes::SPACE_2.0 + sizes::SPACE_1.0 + 14.0 + 10.0
    }

    fn thumbnail_key(&self, page_index: u16) -> RenderCacheKey {
        RenderCacheKey::new(self.doc_fingerprint, page_index, 100, 0, RenderQuality::LqThumb, 100)
    }

    fn dynamic_overscan_rows(&self) -> usize {
        match self.pressure_state {
            MemoryPressureState::Normal => THUMBNAIL_OVERSCAN_ROWS,
            MemoryPressureState::Warm => THUMBNAIL_OVERSCAN_ROWS / 2,
            MemoryPressureState::Hot => 2,
            MemoryPressureState::Critical => 0,
        }
    }

    fn max_inflight(&self) -> usize {
        if self.visible_missing_count > 0
            && matches!(self.pressure_state, MemoryPressureState::Normal | MemoryPressureState::Warm)
        {
            return MAX_THUMBNAIL_INFLIGHT_BOOTSTRAP;
        }
        if !self.active_scroll
            && Instant::now() < self.prewarm_fast_until
            && matches!(self.pressure_state, MemoryPressureState::Normal | MemoryPressureState::Warm)
        {
            return MAX_THUMBNAIL_INFLIGHT_BOOTSTRAP;
        }
        if self.active_scroll {
            return MAX_THUMBNAIL_INFLIGHT_ACTIVE;
        }
        match self.pressure_state {
            MemoryPressureState::Normal => MAX_THUMBNAIL_INFLIGHT_IDLE,
            MemoryPressureState::Warm => MAX_THUMBNAIL_INFLIGHT_ACTIVE,
            MemoryPressureState::Hot => MAX_THUMBNAIL_INFLIGHT_ACTIVE,
            MemoryPressureState::Critical => MAX_THUMBNAIL_INFLIGHT_ACTIVE,
        }
    }

    fn row_window_for_scroll(
        row_tops: &[f32],
        row_heights: &[f32],
        scroll_y: f32,
        viewport_height: f32,
        overscan_rows: usize,
    ) -> Option<(usize, usize)> {
        let count = row_tops.len();
        if count == 0 || row_heights.len() != count {
            return None;
        }

        let visible_start = scroll_y.max(0.0);
        let visible_end = (scroll_y + viewport_height.max(1.0)).max(visible_start + 1.0);

        let mut lo = 0usize;
        let mut hi = count;
        while lo < hi {
            let mid = (lo + hi) / 2;
            let row_end = row_tops[mid] + row_heights[mid];
            if row_end <= visible_start {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let mut start = lo;
        if start >= count {
            start = count.saturating_sub(1);
        }

        lo = 0;
        hi = count;
        while lo < hi {
            let mid = (lo + hi) / 2;
            if row_tops[mid] < visible_end {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let mut end = lo;
        if end == 0 {
            end = 1;
        }
        end = end.saturating_sub(1).min(count.saturating_sub(1));
        if end < start {
            end = start;
        }

        let start = start.saturating_sub(overscan_rows);
        let end = end.saturating_add(overscan_rows).min(count.saturating_sub(1));
        Some((start, end))
    }

    fn visible_index_window(&self, overscan_rows: usize) -> Option<(usize, usize)> {
        if self.thumbnail_specs.is_empty() {
            return None;
        }

        let scroll_y = (-self.scrollbar.offset_y()).max(0.0);
        let viewport_height =
            self.scrollbar.handle().bounds().size.height.0.max(self.average_row_height * 6.0);
        Self::row_window_for_scroll(
            &self.row_tops,
            &self.row_heights,
            scroll_y,
            viewport_height,
            overscan_rows,
        )
    }

    fn visible_window_center(window: (usize, usize)) -> usize {
        let (start, end) = window;
        start + ((end.saturating_sub(start)) / 2)
    }

    fn gcd(mut a: usize, mut b: usize) -> usize {
        while b != 0 {
            let t = b;
            b = a % b;
            a = t;
        }
        a
    }

    fn choose_prewarm_stride(page_count: usize) -> usize {
        if page_count <= 2 {
            return 1;
        }
        let mut stride = ((page_count / 2).max(1)) | 1;
        while stride < page_count {
            if Self::gcd(stride, page_count) == 1 {
                return stride;
            }
            stride += 2;
        }
        1
    }

    fn initialize_prewarm_plan(&mut self) {
        let page_count = self.thumbnail_specs.len();
        self.prewarm_stride = Self::choose_prewarm_stride(page_count);
        self.prewarm_next_index = 0;
        self.prewarm_remaining = page_count;
        self.prewarm_fast_until = Instant::now() + PREWARM_BOOTSTRAP_WINDOW;
    }

    fn enqueue_background_prewarm(&mut self) {
        let page_count = self.thumbnail_specs.len();
        if page_count == 0 || self.prewarm_remaining == 0 {
            return;
        }

        let mut attempts = 0usize;
        let max_attempts = page_count.saturating_mul(2).max(1);
        while self.queued_pages.len() < PREWARM_QUEUE_TARGET
            && self.prewarm_remaining > 0
            && attempts < max_attempts
        {
            attempts += 1;
            let index = self.prewarm_next_index % page_count;
            self.prewarm_next_index = (self.prewarm_next_index + self.prewarm_stride) % page_count;
            self.prewarm_remaining = self.prewarm_remaining.saturating_sub(1);

            let page_index = self.thumbnail_specs[index].page_index;
            self.enqueue_thumbnail_render(page_index, false);
        }
    }

    fn is_deep_jump(
        previous_center: Option<usize>,
        next_center: usize,
        overscan_rows: usize,
    ) -> bool {
        let Some(previous_center) = previous_center else {
            return false;
        };
        let delta = previous_center.abs_diff(next_center);
        let threshold = JUMP_PREEMPT_MIN_DELTA.max(overscan_rows.saturating_mul(2));
        delta > threshold
    }

    fn clear_queue(&mut self) {
        self.queued_pages.clear();
        self.queued_page_set.clear();
    }

    fn rebuild_queue_for_window(&mut self, start: usize, end: usize, center: usize) {
        self.clear_queue();

        let page_count = self.thumbnail_specs.len();
        let max_page = page_count.saturating_sub(1);
        let mut staged = Vec::new();

        for hot in Self::protected_hotset_pages(self.selected_page, page_count) {
            if self.queued_page_set.insert(hot) {
                staged.push(hot);
            }
        }
        for index in start..=end.min(max_page) {
            let page = index as u16;
            if self.queued_page_set.insert(page) {
                staged.push(page);
            }
        }

        let mut remaining: Vec<usize> = (0..page_count)
            .filter(|index| !self.queued_page_set.contains(&(*index as u16)))
            .collect();
        remaining.sort_by_key(|index| index.abs_diff(center));
        for index in remaining {
            let page = index as u16;
            if self.queued_page_set.insert(page) {
                staged.push(page);
            }
            if staged.len() >= MAX_QUEUED_THUMBNAILS {
                break;
            }
        }

        for page in staged.into_iter().take(MAX_QUEUED_THUMBNAILS) {
            self.queued_pages.push_back(QueuedThumbnail {
                page_index: page,
                generation: self.thumbnail_generation,
            });
        }
    }

    fn mark_deep_jump(&mut self, start: usize, end: usize, center: usize) {
        self.rebuild_queue_for_window(start, end, center);
        self.force_immediate_dispatch_until = Instant::now() + JUMP_FORCE_DISPATCH_WINDOW;
    }

    fn schedule_visible_thumbnails(&mut self, cx: &mut gpui::Context<Self>) {
        let overscan_rows = self.dynamic_overscan_rows();
        self.trim_thumbnail_cache_by_distance();
        let Some((start, end)) = self.visible_index_window(overscan_rows) else {
            // First frame before scroll bounds are known.
            let warmup = self.thumbnail_specs.len().min(overscan_rows.saturating_mul(2).max(2));
            for index in 0..warmup {
                let page_index = self.thumbnail_specs[index].page_index;
                self.enqueue_thumbnail_render(page_index, false);
            }
            self.enqueue_thumbnail_render(self.selected_page, true);
            self.pump_thumbnail_queue(cx);
            return;
        };

        let center = Self::visible_window_center((start, end));
        if Self::is_deep_jump(self.last_visible_center, center, overscan_rows) {
            self.mark_deep_jump(start, end, center);
        }
        self.last_visible_center = Some(center);

        for index in start..=end {
            let page_index = self.thumbnail_specs[index].page_index;
            self.enqueue_thumbnail_render(page_index, true);
        }
        self.enqueue_thumbnail_render(self.selected_page, true);
        self.enqueue_background_prewarm();
        self.pump_thumbnail_queue(cx);
    }

    fn enqueue_thumbnail_render(&mut self, page_index: u16, high_priority: bool) {
        let key = self.thumbnail_key(page_index);
        if self.doc_fingerprint == 0
            || self.thumbnail_cache.contains(&key)
            || self.inflight_pages.contains_key(&page_index)
            || self.queued_page_set.contains(&page_index)
        {
            return;
        }

        if self.queued_pages.len() >= MAX_QUEUED_THUMBNAILS {
            if !high_priority {
                return;
            }
            if let Some(evicted) = self.queued_pages.pop_back() {
                self.queued_page_set.remove(&evicted.page_index);
            }
        }

        if high_priority {
            self.queued_pages.push_front(QueuedThumbnail {
                page_index,
                generation: self.thumbnail_generation,
            });
        } else {
            self.queued_pages.push_back(QueuedThumbnail {
                page_index,
                generation: self.thumbnail_generation,
            });
        }
        self.queued_page_set.insert(page_index);
    }

    fn pump_thumbnail_queue(&mut self, cx: &mut gpui::Context<Self>) {
        if self.doc_fingerprint == 0 {
            return;
        }

        let force_dispatch = Instant::now() < self.force_immediate_dispatch_until;
        let bootstrap_dispatch = Instant::now() < self.prewarm_fast_until;
        let urgent_visible_fill = self.visible_missing_count > 0;
        if self.active_scroll
            && !force_dispatch
            && !bootstrap_dispatch
            && !urgent_visible_fill
            && self.last_dispatch_at.elapsed() < ACTIVE_SCROLL_THUMBNAIL_DISPATCH_INTERVAL
        {
            return;
        }

        while self.inflight_pages.len() < self.max_inflight() {
            let Some(queued) = self.queued_pages.pop_front() else {
                break;
            };
            let page_index = queued.page_index;
            self.queued_page_set.remove(&page_index);
            self.spawn_thumbnail_render(page_index, queued.generation, cx);
            self.last_dispatch_at = Instant::now();
        }
    }

    fn spawn_thumbnail_render(
        &mut self,
        page_index: u16,
        generation: u64,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(doc) = self.document.clone() else {
            return;
        };
        let Some(spec) = self.thumbnail_specs.get(page_index as usize).copied() else {
            return;
        };

        let key = self.thumbnail_key(page_index);
        if self.thumbnail_cache.contains(&key)
            || self.inflight_pages.contains_key(&page_index)
            || self.doc_fingerprint == 0
        {
            return;
        }

        self.inflight_pages.insert(page_index, generation);
        let doc_fingerprint = self.doc_fingerprint;
        let render_width = spec.width.saturating_mul(THUMBNAIL_RENDER_MULTIPLIER);
        let render_height = spec.height.saturating_mul(THUMBNAIL_RENDER_MULTIPLIER);

        cx.spawn(move |this: gpui::WeakEntity<ThumbnailSidebar>, cx: &mut gpui::AsyncApp| {
            let mut async_cx = cx.clone();
            async move {
                let result = async_cx
                    .background_executor()
                    .spawn(async move {
                        doc.render_page_rgba_with_quality(
                            page_index,
                            render_width,
                            render_height,
                            RenderQuality::LqThumb,
                        )
                    })
                    .await;

                let _ = this.update(&mut async_cx, move |sidebar, cx| {
                    sidebar.finish_thumbnail_render(
                        page_index,
                        generation,
                        key,
                        doc_fingerprint,
                        spec.width,
                        spec.height,
                        render_width,
                        render_height,
                        result,
                        cx,
                    );
                });
            }
        })
        .detach();
    }

    fn finish_thumbnail_render(
        &mut self,
        page_index: u16,
        _generation: u64,
        key: RenderCacheKey,
        doc_fingerprint: u64,
        display_width: u32,
        display_height: u32,
        render_width: u32,
        render_height: u32,
        result: Result<Vec<u8>, butterpaper_render::PdfError>,
        cx: &mut gpui::Context<Self>,
    ) {
        self.inflight_pages.remove(&page_index);

        if self.doc_fingerprint != doc_fingerprint {
            self.pump_thumbnail_queue(cx);
            return;
        }

        if let Ok(rgba_pixels) = result {
            let pixel_width = ((render_width as f32) * 0.25).round().max(1.0) as u32;
            let pixel_height = ((render_height as f32) * 0.25).round().max(1.0) as u32;
            if let Some(image) = create_render_image(rgba_pixels, pixel_width, pixel_height) {
                let cached = CachedImage::from_image(
                    image,
                    pixel_width,
                    pixel_height,
                    display_width,
                    display_height,
                );
                let _ = self.thumbnail_cache.insert(key, cached.clone(), None);
                if let Ok(mut preview_cache) = self.preview_cache.lock() {
                    preview_cache.insert(self.doc_fingerprint, page_index, cached);
                }
                self.refresh_thumbnail_perf_peaks();
            }
        }

        self.trim_thumbnail_cache_by_distance();
        self.enqueue_background_prewarm();
        self.pump_thumbnail_queue(cx);
        cx.notify();
    }

    fn refresh_thumbnail_perf_peaks(&mut self) {
        self.peak_thumbnail_decoded_bytes =
            self.peak_thumbnail_decoded_bytes.max(self.thumbnail_cache.decoded_bytes());
        self.peak_thumbnail_texture_bytes =
            self.peak_thumbnail_texture_bytes.max(self.thumbnail_cache.texture_bytes());
    }

    fn protected_hotset_pages(selected_page: u16, page_count: usize) -> HashSet<u16> {
        if page_count == 0 {
            return HashSet::new();
        }
        let max_page = page_count.saturating_sub(1) as i32;
        ((selected_page as i32 - THUMBNAIL_HOTSET_RADIUS)
            ..=(selected_page as i32 + THUMBNAIL_HOTSET_RADIUS))
            .filter(|page| *page >= 0 && *page <= max_page)
            .map(|page| page as u16)
            .collect()
    }

    fn trim_keep_pages(
        page_count: usize,
        selected_page: u16,
        pressure_state: MemoryPressureState,
        overscan_rows: usize,
        visible_window: Option<(usize, usize)>,
        queued_page_set: &HashSet<u16>,
        inflight_pages: &HashMap<u16, u64>,
    ) -> HashSet<u16> {
        let mut keep_pages = Self::protected_hotset_pages(selected_page, page_count);
        keep_pages.extend(queued_page_set.iter().copied());
        keep_pages.extend(inflight_pages.keys().copied());

        let Some((visible_start, visible_end)) = visible_window else {
            return keep_pages;
        };

        let extra = match pressure_state {
            MemoryPressureState::Normal => overscan_rows.saturating_mul(2),
            MemoryPressureState::Warm => overscan_rows,
            MemoryPressureState::Hot | MemoryPressureState::Critical => 0,
        };

        let start = visible_start.saturating_sub(extra);
        let end = visible_end.saturating_add(extra).min(page_count.saturating_sub(1));
        for index in start..=end {
            keep_pages.insert(index as u16);
        }

        keep_pages
    }

    fn trim_thumbnail_cache_by_distance(&mut self) {
        if self.doc_fingerprint == 0 {
            return;
        }

        let overscan_rows = self.dynamic_overscan_rows();
        let visible_window = self.visible_index_window(overscan_rows);
        let keep_pages = Self::trim_keep_pages(
            self.thumbnail_specs.len(),
            self.selected_page,
            self.pressure_state,
            overscan_rows,
            visible_window,
            &self.queued_page_set,
            &self.inflight_pages,
        );

        self.thumbnail_cache.retain(|key, _| {
            key.doc_fingerprint == self.doc_fingerprint && keep_pages.contains(&key.page_index)
        });
        if let Ok(mut preview_cache) = self.preview_cache.lock() {
            preview_cache.trim_to_budget(&keep_pages, self.doc_fingerprint);
        }
    }

    pub fn perf_snapshot(&self) -> ThumbnailPerfSnapshot {
        ThumbnailPerfSnapshot {
            current_thumbnail_decoded_bytes: self.thumbnail_cache.decoded_bytes(),
            current_thumbnail_texture_bytes: self.thumbnail_cache.texture_bytes(),
            peak_thumbnail_decoded_bytes: self.peak_thumbnail_decoded_bytes,
            peak_thumbnail_texture_bytes: self.peak_thumbnail_texture_bytes,
            visible_blank_frames: self.visible_blank_frames,
            visible_frame_samples: self.visible_frame_samples,
        }
    }

    pub fn has_visible_thumbnail_gaps(&self) -> bool {
        self.visible_missing_count > 0
    }

    fn thumbnail_cells(&mut self) -> Vec<ThumbnailCell> {
        let overscan_rows = self.dynamic_overscan_rows();
        let visible_window = self.visible_index_window(overscan_rows).or_else(|| {
            if self.thumbnail_specs.is_empty() {
                None
            } else {
                Some((0, self.thumbnail_specs.len().min(4).saturating_sub(1)))
            }
        });
        let mut preview_candidate_pages =
            Self::protected_hotset_pages(self.selected_page, self.thumbnail_specs.len());
        if let Some((start, end)) = visible_window {
            for index in start..=end {
                preview_candidate_pages.insert(index as u16);
            }
        }

        let specs = self.thumbnail_specs.clone();
        let mut cells = Vec::with_capacity(specs.len());
        for spec in specs {
            let key = self.thumbnail_key(spec.page_index);
            let mut image = self.thumbnail_cache.get(&key).map(|cached| cached.image);
            if image.is_none() && preview_candidate_pages.contains(&spec.page_index) {
                if let Ok(mut preview_cache) = self.preview_cache.lock() {
                    image = preview_cache.get(self.doc_fingerprint, spec.page_index);
                }
            }
            let is_skeleton = image.is_none();
            cells.push(ThumbnailCell {
                page_index: spec.page_index,
                width: spec.width,
                height: spec.height,
                image,
                is_skeleton,
            });
        }
        cells
    }

    fn start_scrollbar_drag(&mut self, mouse_y_window: f32, cx: &mut gpui::Context<Self>) {
        if self.scrollbar.start_drag(mouse_y_window) {
            self.schedule_visible_thumbnails(cx);
            cx.notify();
        }
    }

    fn update_scrollbar_drag(&mut self, mouse_y_window: f32, cx: &mut gpui::Context<Self>) {
        if self.scrollbar.update_drag(mouse_y_window) {
            self.schedule_visible_thumbnails(cx);
            cx.notify();
        }
    }

    fn end_scrollbar_drag(&mut self, cx: &mut gpui::Context<Self>) {
        self.scrollbar.end_drag();
        self.schedule_visible_thumbnails(cx);
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
        self.schedule_visible_thumbnails(cx);

        let theme = current_theme(window, cx);
        let selected_page = self.selected_page;
        let cells = self.thumbnail_cells();
        if let Some((start, end)) = self.visible_index_window(0) {
            self.visible_frame_samples = self.visible_frame_samples.saturating_add(1);
            let missing_count = cells
                .iter()
                .enumerate()
                .filter(|(idx, _)| *idx >= start && *idx <= end)
                .filter(|(_, cell)| cell.is_skeleton)
                .count();
            self.visible_missing_count = missing_count;
            if missing_count > 0 {
                self.visible_blank_frames = self.visible_blank_frames.saturating_add(1);
            }
        } else {
            self.visible_missing_count = 0;
        }
        let text_muted = theme.text_muted;
        let element_hover = theme.element_hover;
        let selected_bg = theme.element_selected;
        let selected_border = color::subtle_border(theme.border);
        let scroll_handle = self.scrollbar.handle();
        let scrollbar = self.scrollbar.metrics();
        let entity = cx.entity().downgrade();

        div()
            .id("thumbnail-sidebar")
            .flex()
            .flex_row()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(theme.surface)
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
                        sidebar.update(cx, |this, cx| {
                            this.end_scrollbar_drag(cx);
                        });
                    }
                }
            })
            .child(
                div()
                    .id("thumbnail-scroll-container")
                    .h_full()
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&scroll_handle)
                    .child(div().id("thumbnail-list").flex().flex_col().gap_2().p_2().children(
                        cells.into_iter().enumerate().map(move |(idx, cell)| {
                            let page_index = cell.page_index;
                            let is_selected = page_index == selected_page;

                            div().id(("thumbnail", idx)).flex().justify_center().child(
                                div()
                                    .id(("thumbnail-card", idx))
                                    .w(px(sizes::THUMBNAIL_CARD_WIDTH_PX))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap(sizes::SPACE_2)
                                    .px(sizes::SPACE_2)
                                    .py(sizes::SPACE_2)
                                    .rounded_lg()
                                    .border_1()
                                    .border_color(color::transparent())
                                    .cursor_pointer()
                                    .when(is_selected, move |s| {
                                        s.bg(selected_bg).border_color(selected_border).shadow_sm()
                                    })
                                    .hover(
                                        move |s| if is_selected { s } else { s.bg(element_hover) },
                                    )
                                    .on_click(cx.listener(move |this, _, _window, cx| {
                                        this.selected_page = page_index;
                                        this.enqueue_thumbnail_render(page_index, true);
                                        this.pump_thumbnail_queue(cx);
                                        let on_page_select = this.on_page_select.clone();
                                        if let Some(callback) = on_page_select {
                                            cx.defer(move |cx| callback(page_index, cx));
                                        }
                                        cx.notify();
                                    }))
                                    .child(
                                        div().w_full().flex().justify_center().child(
                                            match cell.image {
                                                Some(image) => div()
                                                    .rounded_sm()
                                                    .overflow_hidden()
                                                    .shadow_sm()
                                                    .child(
                                                        img(ImageSource::Render(image))
                                                            .w(px(cell.width as f32))
                                                            .h(px(cell.height as f32)),
                                                    )
                                                    .into_any_element(),
                                                None => div()
                                                    .w(px(cell.width as f32))
                                                    .h(px(cell.height as f32))
                                                    .rounded_sm()
                                                    .border_1()
                                                    .border_color(theme.border)
                                                    .bg(theme.elevated_surface)
                                                    .into_any_element(),
                                            },
                                        ),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .mt(sizes::SPACE_1)
                                            .text_xs()
                                            .text_center()
                                            .text_color(if is_selected {
                                                theme.text
                                            } else {
                                                text_muted
                                            })
                                            .child(format!("{}", page_index + 1)),
                                    ),
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
                                sidebar.update(cx, |this, cx| {
                                    this.end_scrollbar_drag(cx);
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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, px, size, ScrollDelta, ScrollWheelEvent, TestAppContext};
    use std::path::PathBuf;
    use std::sync::Arc;
    use butterpaper_render::PdfDocument;

    fn fixture_pdf_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures").join(name)
    }

    #[test]
    fn protected_hotset_keeps_selected_plus_minus_one() {
        let keep = ThumbnailSidebar::protected_hotset_pages(5, 20);
        assert!(keep.contains(&4));
        assert!(keep.contains(&5));
        assert!(keep.contains(&6));
        assert_eq!(keep.len(), 3);
    }

    #[test]
    fn hot_and_critical_trim_only_keep_visible_and_hotset() {
        let queued = HashSet::new();
        let inflight = HashMap::new();
        let keep = ThumbnailSidebar::trim_keep_pages(
            30,
            10,
            MemoryPressureState::Hot,
            8,
            Some((12, 14)),
            &queued,
            &inflight,
        );
        assert!(keep.contains(&9));
        assert!(keep.contains(&10));
        assert!(keep.contains(&11));
        assert!(keep.contains(&12));
        assert!(keep.contains(&13));
        assert!(keep.contains(&14));
        assert!(!keep.contains(&2));
        assert!(!keep.contains(&25));
    }

    #[test]
    fn trim_keep_pages_preserves_queued_and_inflight() {
        let queued = HashSet::from([22_u16]);
        let inflight = HashMap::from([(23_u16, 1_u64)]);
        let keep = ThumbnailSidebar::trim_keep_pages(
            30,
            5,
            MemoryPressureState::Critical,
            0,
            Some((5, 6)),
            &queued,
            &inflight,
        );
        assert!(keep.contains(&22));
        assert!(keep.contains(&23));
    }

    #[test]
    fn deep_jump_detection_uses_min_threshold_or_overscan_multiple() {
        assert!(!ThumbnailSidebar::is_deep_jump(None, 40, 8));
        assert!(!ThumbnailSidebar::is_deep_jump(Some(20), 30, 8));
        assert!(ThumbnailSidebar::is_deep_jump(Some(10), 40, 8));
        assert!(ThumbnailSidebar::is_deep_jump(Some(0), 25, 4));
    }

    #[gpui::test]
    fn deep_jump_reorders_queue_without_generation_invalidation(cx: &mut TestAppContext) {
        let (sidebar, cx) = cx.add_window_view(|_, cx| ThumbnailSidebar::new(cx));

        cx.update(|_, app| {
            sidebar.update(app, |sidebar, _cx| {
                sidebar.thumbnail_specs = (0..30)
                    .map(|page_index| ThumbnailSpec { page_index, width: 120, height: 160 })
                    .collect();
                sidebar.selected_page = 10;
                sidebar.thumbnail_generation = 9;
                sidebar.mark_deep_jump(12, 15, 13);

                assert_eq!(sidebar.thumbnail_generation, 9);
                assert!(!sidebar.queued_pages.is_empty());
            });
        });
    }

    #[test]
    fn prewarm_stride_is_coprime_and_non_zero() {
        for pages in [1_usize, 2, 3, 8, 17, 64, 121] {
            let stride = ThumbnailSidebar::choose_prewarm_stride(pages);
            assert!(stride >= 1);
            if pages > 1 {
                assert_eq!(ThumbnailSidebar::gcd(stride, pages), 1);
            }
        }
    }

    #[test]
    fn row_window_tracks_variable_heights_without_drift() {
        let row_heights = vec![100.0_f32, 180.0, 120.0, 220.0, 140.0];
        let mut row_tops = Vec::new();
        let mut y = 0.0_f32;
        for h in &row_heights {
            row_tops.push(y);
            y += *h;
        }

        let w0 = ThumbnailSidebar::row_window_for_scroll(&row_tops, &row_heights, 0.0, 90.0, 0)
            .expect("window");
        assert_eq!(w0, (0, 0));

        let w1 = ThumbnailSidebar::row_window_for_scroll(&row_tops, &row_heights, 130.0, 90.0, 0)
            .expect("window");
        assert_eq!(w1, (1, 1));

        let w2 = ThumbnailSidebar::row_window_for_scroll(&row_tops, &row_heights, 320.0, 200.0, 0)
            .expect("window");
        assert_eq!(w2, (2, 3));
    }

    #[gpui::test]
    fn deep_scroll_window_pages_are_enqueued(cx: &mut TestAppContext) {
        let (sidebar, cx) = cx.add_window_view(|_, cx| ThumbnailSidebar::new(cx));
        cx.simulate_resize(size(px(1200.0), px(900.0)));

        let doc = Arc::new(
            PdfDocument::open(&fixture_pdf_path("large.pdf")).expect("fixture PDF should open"),
        );

        cx.update(|_, app| {
            sidebar.update(app, |sidebar, cx| {
                sidebar.set_document(Some(doc), cx);
                let count = sidebar.thumbnail_specs.len();
                assert!(count > 0);

                // Force a deep sidebar scroll and validate that the mapped window pages
                // are inserted into the active render queue immediately.
                let deep_index = count.saturating_mul(3) / 4;
                let deep_top = sidebar
                    .row_tops
                    .get(deep_index)
                    .copied()
                    .unwrap_or_else(|| sidebar.row_tops.last().copied().unwrap_or(0.0));
                sidebar.scrollbar.set_offset_y(-deep_top);
                sidebar.schedule_visible_thumbnails(cx);

                let (start, end) = sidebar
                    .visible_index_window(sidebar.dynamic_overscan_rows())
                    .expect("visible window");
                let mut missing = Vec::new();
                for idx in start..=end {
                    let page = sidebar.thumbnail_specs[idx].page_index;
                    let key = sidebar.thumbnail_key(page);
                    let queued = sidebar.queued_page_set.contains(&page);
                    let inflight = sidebar.inflight_pages.contains_key(&page);
                    let cached = sidebar.thumbnail_cache.contains(&key);
                    if !(queued || inflight || cached) {
                        missing.push(page);
                    }
                }
                assert!(
                    missing.is_empty(),
                    "visible deep-window pages were not scheduled: {:?}",
                    missing
                );
            });
        });
    }

    #[gpui::test]
    fn wheel_event_scrolls_thumbnail_sidebar(cx: &mut TestAppContext) {
        let (sidebar, cx) = cx.add_window_view(|_, cx| ThumbnailSidebar::new(cx));
        cx.simulate_resize(size(px(320.0), px(900.0)));

        let doc = Arc::new(
            PdfDocument::open(&fixture_pdf_path("large.pdf")).expect("fixture PDF should open"),
        );

        cx.update(|_, app| {
            sidebar.update(app, |sidebar, cx| {
                sidebar.set_document(Some(doc), cx);
            });
        });
        cx.run_until_parked();

        let (before_offset, before_window) = cx.read_entity(&sidebar, |sidebar, _| {
            (
                sidebar.scrollbar.offset_y(),
                sidebar.visible_index_window(0).expect("window before"),
            )
        });

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(150.0), px(500.0)),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-480.0))),
            ..Default::default()
        });
        cx.run_until_parked();

        let (after_offset, after_window) = cx.read_entity(&sidebar, |sidebar, _| {
            (
                sidebar.scrollbar.offset_y(),
                sidebar.visible_index_window(0).expect("window after"),
            )
        });

        assert_ne!(after_offset, before_offset, "sidebar scroll offset should change");
        assert!(
            after_window.0 >= before_window.0,
            "visible window should advance after downward wheel scroll"
        );
    }

    #[gpui::test]
    fn scrollbar_drag_scrolls_thumbnail_sidebar_with_logged_windows(cx: &mut TestAppContext) {
        let (sidebar, cx) = cx.add_window_view(|_, cx| ThumbnailSidebar::new(cx));
        cx.simulate_resize(size(px(320.0), px(900.0)));

        let doc = Arc::new(
            PdfDocument::open(&fixture_pdf_path("large.pdf")).expect("fixture PDF should open"),
        );

        cx.update(|_, app| {
            sidebar.update(app, |sidebar, cx| {
                sidebar.set_document(Some(doc), cx);
            });
        });
        cx.run_until_parked();

        let (before_offset, before_window, bounds_top, bounds_height, metrics) = cx.read_entity(
            &sidebar,
            |sidebar, _| {
            (
                sidebar.scrollbar.offset_y(),
                sidebar.visible_index_window(0).expect("window before"),
                sidebar.scrollbar.handle().bounds().origin.y.0,
                sidebar.scrollbar.handle().bounds().size.height.0,
                sidebar.scrollbar.metrics().expect("scrollbar metrics"),
            )
        });

        eprintln!(
            "sidebar_drag_test before: offset_y={}, window={:?}, thumb_top={}, thumb_height={}, bounds_height={}",
            before_offset,
            before_window,
            metrics.thumb_top,
            metrics.thumb_height,
            bounds_height
        );

        let start_y = bounds_top + metrics.thumb_top + (metrics.thumb_height * 0.5);
        let end_y = bounds_top + bounds_height - 2.0;

        cx.update(|_, app| {
            sidebar.update(app, |sidebar, cx| {
                sidebar.start_scrollbar_drag(start_y, cx);
                sidebar.update_scrollbar_drag(end_y, cx);
                sidebar.end_scrollbar_drag(cx);
            });
        });
        cx.run_until_parked();

        let (after_offset, after_window) = cx.read_entity(&sidebar, |sidebar, _| {
            (
                sidebar.scrollbar.offset_y(),
                sidebar.visible_index_window(0).expect("window after"),
            )
        });

        eprintln!(
            "sidebar_drag_test after: offset_y={}, window={:?}, start_y={}, end_y={}",
            after_offset, after_window, start_y, end_y
        );

        assert_ne!(after_offset, before_offset, "sidebar drag should change scroll offset");
        assert!(
            after_window.0 > before_window.0,
            "visible window should advance after dragging thumb down"
        );
    }
}
