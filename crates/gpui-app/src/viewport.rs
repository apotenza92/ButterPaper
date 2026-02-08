//! PDF Viewport component
//!
//! Displays rendered PDF pages with continuous vertical scroll
//! and GPU texture caching.

#![allow(clippy::type_complexity)]
#![allow(clippy::arc_with_non_send_sync)]

use crate::cache::{
    create_render_image, AdaptiveMemoryBudget, ByteLruCache, CachedImage, MemoryPressureState,
    RenderCacheKey,
};
use crate::components::{scrollbar_gutter, ScrollbarController};
use crate::current_theme;
use crate::preview_cache::SharedPreviewCache;
use crate::process_memory;
use butterpaper_render::{PdfDocument, RenderQuality};
use gpui::{
    div, img, prelude::*, px, FocusHandle, Focusable, ImageSource, MouseMoveEvent, ScrollWheelEvent,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Gap between pages in pixels
const PAGE_GAP: f32 = 20.0;

/// Buffer above/below viewport for pre-rendering
const RENDER_BUFFER_NORMAL: f32 = 400.0;
const RENDER_BUFFER_WARM: f32 = 260.0;
const RENDER_BUFFER_HOT: f32 = 120.0;
const RENDER_BUFFER_CRITICAL: f32 = 40.0;

/// Scheduler frame budgets.
const MAX_LQ_JOBS_PER_FRAME: usize = 2;
const MAX_HQ_JOBS_PER_FRAME: usize = 2;
const MAX_QUEUED_LQ_JOBS: usize = 12;
const MAX_QUEUED_HQ_JOBS: usize = 6;
const HQ_RING_RADIUS: i32 = 2;

/// Quality and render safety limits.
const SCROLL_IDLE_DEBOUNCE: Duration = Duration::from_millis(80);
const MAX_RENDER_EDGE_PX: u32 = 8192;
const MAX_RENDER_MEGAPIXELS: u64 = 32;
const MICRO_SCROLL_HYSTERESIS_PX: f32 = 24.0;
const MEMORY_PRESSURE_NORMAL_MAX: f64 = 0.70;
const MEMORY_PRESSURE_WARM_MAX: f64 = 0.82;
const MEMORY_PRESSURE_HOT_MAX: f64 = 0.92;
const MEMORY_PRESSURE_HYSTERESIS: f64 = 0.03;
const IDLE_TRIM_COOLDOWN: Duration = Duration::from_millis(250);
const IDLE_SETTLE_DEBOUNCE: Duration = Duration::from_millis(500);
const MIN_ACTIVE_TARGET_BYTES: u64 = 512 * 1024 * 1024;
const ACTIVE_TARGET_BUDGET_FLOOR_RATIO: f64 = 0.25;
const ACTIVE_TARGET_WORKING_SET_MULTIPLIER: f64 = 2.4;
/// Scroll-wheel delta (in px) required to advance one page in single-page mode.
const SINGLE_PAGE_WHEEL_THRESHOLD_PX: f32 = 48.0;
/// Single-page max-scroll values at or below this are considered fit-page and flip immediately.
const SINGLE_PAGE_IMMEDIATE_FLIP_SCROLL_EPSILON_PX: f32 = 4.0;
/// Enable verbose single-page wheel diagnostics with `BUTTERPAPER_DEBUG_WHEEL=1`.
const WHEEL_DEBUG_ENV: &str = "BUTTERPAPER_DEBUG_WHEEL";
const WHEEL_DEBUG_LOG_PATH: &str = "/tmp/butterpaper-wheel.log";

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
pub enum ViewMode {
    Continuous,
    SinglePage,
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

fn continuous_scroll_for_single_page(layout_y_offset: f32, single_scroll_y: f32) -> f32 {
    (layout_y_offset - PAGE_GAP + single_scroll_y).max(0.0)
}

fn single_page_scroll_for_continuous(layout_y_offset: f32, continuous_scroll_y: f32) -> f32 {
    (continuous_scroll_y + PAGE_GAP - layout_y_offset).max(0.0)
}

/// Rendered page ready for display
#[derive(Clone)]
#[allow(dead_code)]
struct DisplayPage {
    page_index: u16,
    width: u32,
    height: u32,
    y_offset: f32,
    image: Option<Arc<gpui::RenderImage>>,
    quality_state: PageQualityState,
}

/// Page layout info (before rendering)
#[derive(Clone)]
struct PageLayout {
    page_index: u16,
    width: f32,
    height: f32,
    y_offset: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PageQualityState {
    Skeleton,
    UltraLqReady,
    LqReady,
    HqReady,
    Upgrading,
}

#[derive(Clone)]
struct RenderRequest {
    key: RenderCacheKey,
    page_index: u16,
    quality: RenderQuality,
    generation: u64,
    render_width: u32,
    render_height: u32,
    display_width: u32,
    display_height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InflightJob {
    job_id: u64,
    generation: u64,
    estimated_bytes: u64,
}

#[derive(Clone, Copy, Debug)]
struct RetryState {
    attempts: u8,
    next_retry_at: Instant,
    suppressed_generation: Option<u64>,
}

#[derive(Clone, Debug, Default)]
struct PerfStats {
    session_started_at: Option<Instant>,
    first_lq_ready_ms: Option<u64>,
    first_hq_ready_ms: Option<u64>,
    frame_cpu_samples_ms: Vec<f32>,
    peak_decoded_bytes: u64,
    peak_texture_bytes: u64,
    lq_jobs_scheduled: u64,
    hq_jobs_scheduled: u64,
    jobs_canceled: u64,
    peak_rss_bytes: u64,
    end_rss_bytes: u64,
    hq_suppression_count: u64,
    current_viewport_decoded_bytes: u64,
    current_viewport_texture_bytes: u64,
    current_inflight_estimated_bytes: u64,
    current_owned_bytes: u64,
    peak_owned_bytes: u64,
    end_owned_bytes: u64,
    active_target_bytes: u64,
    idle_target_bytes: u64,
    visible_blank_frames: u64,
    visible_ultra_lq_frames: u64,
    visible_frame_samples: u64,
    hq_visible_latency_samples_ms: Vec<f32>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default)]
pub struct MemoryTargets {
    pub working_set_floor_bytes: u64,
    pub active_target_bytes: u64,
    pub idle_target_bytes: u64,
    pub pressure_budget_bytes: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct PressureDurationsMs {
    pub normal_ms: u64,
    pub warm_ms: u64,
    pub hot_ms: u64,
    pub critical_ms: u64,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug, Serialize)]
pub struct PerfSnapshot {
    pub first_lq_ms: Option<u64>,
    pub first_hq_ms: Option<u64>,
    pub p95_ui_frame_cpu_ms: Option<f32>,
    pub p99_ui_frame_cpu_ms: Option<f32>,
    pub peak_decoded_bytes: u64,
    pub peak_texture_bytes: u64,
    pub viewport_cache_cap_bytes: u64,
    pub lq_jobs_scheduled: u64,
    pub hq_jobs_scheduled: u64,
    pub jobs_canceled: u64,
    pub peak_rss_bytes: u64,
    pub end_rss_bytes: u64,
    pub pressure_state_durations_ms: PressureDurationsMs,
    pub hq_suppression_count: u64,
    pub memory_pressure_state: MemoryPressureState,
    pub memory_budget_total_bytes: u64,
    pub current_viewport_decoded_bytes: u64,
    pub current_viewport_texture_bytes: u64,
    pub current_inflight_estimated_bytes: u64,
    pub current_owned_bytes: u64,
    pub peak_owned_bytes: u64,
    pub end_owned_bytes: u64,
    pub active_target_bytes: u64,
    pub idle_target_bytes: u64,
    pub visible_blank_frames: u64,
    pub visible_ultra_lq_frames: u64,
    pub visible_frame_samples: u64,
    pub hq_visible_latency_p95_ms: Option<f32>,
}

/// PDF Viewport state
pub struct PdfViewport {
    /// Loaded PDF document
    document: Option<Arc<PdfDocument>>,
    /// Effective zoom level (percentage, e.g. 100 = 100%)
    pub zoom_level: u32,
    /// How zoom level should react to canvas size changes.
    zoom_mode: ZoomMode,
    /// Whether the viewport shows all pages or a single page at a time.
    view_mode: ViewMode,
    /// Active page index for single-page mode and fit calculations.
    current_page_index: u16,
    /// Accumulates wheel delta to convert smooth scrolling into page jumps.
    single_page_wheel_accum_px: f32,
    /// Scroll offset Y (for continuous scroll)
    scroll_y: f32,
    /// Page layouts (computed on load/zoom)
    page_layouts: Vec<PageLayout>,
    /// Total document height
    total_height: f32,
    /// Byte-bounded viewport surface cache.
    cache: ByteLruCache,
    /// Shared ultra-LQ preview cache consumed by viewport + sidebar.
    preview_cache: Arc<Mutex<SharedPreviewCache>>,
    /// Pages ready for display
    display_pages: Vec<DisplayPage>,
    /// Viewport height (for visibility calculation)
    viewport_height: f32,
    /// Canvas width used for fit calculations.
    canvas_width: f32,
    /// Canvas height used for fit calculations.
    canvas_height: f32,
    /// Focus handle for keyboard input
    focus_handle: FocusHandle,
    /// Shared scrollbar controller for custom gutter + drag.
    scrollbar: ScrollbarController,
    /// Callback for page change
    on_page_change: Option<Box<dyn Fn(u16, &mut gpui::App) + 'static>>,
    /// Display scale factor (for Retina support)
    scale_factor: f32,
    /// Stable per-document cache namespace.
    doc_fingerprint: u64,
    /// Generation token invalidating stale render results.
    render_generation: u64,
    /// Sequence for inflight job IDs.
    next_job_id: u64,
    /// Render jobs currently executing.
    inflight_jobs: HashMap<RenderCacheKey, InflightJob>,
    /// LQ queue for current frame budget.
    lq_queue: VecDeque<RenderRequest>,
    /// HQ queue for current frame budget.
    hq_queue: VecDeque<RenderRequest>,
    /// Fast lookup for de-duplicating queued keys.
    queued_lq_keys: HashSet<RenderCacheKey>,
    /// Fast lookup for de-duplicating queued keys.
    queued_hq_keys: HashSet<RenderCacheKey>,
    /// Retry/backoff state for failed jobs.
    retry_state: HashMap<RenderCacheKey, RetryState>,
    /// Last scroll or viewport movement timestamp.
    last_scroll_activity: Instant,
    /// Last known scroll direction: -1 up, 0 unknown, 1 down.
    scroll_direction: i8,
    /// Scroll anchor where HQ was last stably shown.
    last_hq_anchor_scroll_y: f32,
    /// Performance telemetry used by perf smoke tests and diagnostics.
    perf_stats: PerfStats,
    /// Adaptive memory budget for pressure control.
    memory_budget: AdaptiveMemoryBudget,
    /// Dynamic targets derived from workload + adaptive budget.
    memory_targets: MemoryTargets,
    /// Current memory pressure state.
    memory_pressure_state: MemoryPressureState,
    /// Timestamp when the pressure state was entered.
    memory_pressure_started_at: Instant,
    /// Accumulated time spent in each pressure state.
    pressure_durations_ms: PressureDurationsMs,
    /// Last time idle trim pass ran.
    last_idle_trim_at: Instant,
    /// Tracks when strict-visible pages started waiting for HQ.
    hq_visible_pending_since: HashMap<u16, Instant>,
    /// Indicates sidebar has visible thumbnail gaps and needs renderer bandwidth.
    sidebar_thumbnail_backpressure: bool,
}

impl PdfViewport {
    fn wheel_debug_enabled() -> bool {
        std::env::var(WHEEL_DEBUG_ENV)
            .map(|value| {
                let value = value.trim().to_ascii_lowercase();
                matches!(value.as_str(), "1" | "true" | "yes" | "on")
            })
            .unwrap_or(false)
    }

    fn wheel_debug_log(message: &str) {
        eprintln!("{message}");
        if let Ok(mut file) =
            OpenOptions::new().create(true).append(true).open(WHEEL_DEBUG_LOG_PATH)
        {
            let _ = writeln!(file, "{message}");
        }
    }

    fn perf_reset(&mut self) {
        self.perf_stats = PerfStats::default();
        self.perf_stats.session_started_at = Some(Instant::now());
        self.pressure_durations_ms = PressureDurationsMs::default();
        self.memory_pressure_started_at = Instant::now();
    }

    fn perf_elapsed_ms(&self) -> Option<u64> {
        self.perf_stats.session_started_at.map(|started| started.elapsed().as_millis() as u64)
    }

    fn perf_record_frame_cpu_ms(&mut self, frame_ms: f32) {
        self.perf_stats.frame_cpu_samples_ms.push(frame_ms);
        if self.perf_stats.frame_cpu_samples_ms.len() > 4096 {
            let drain = self.perf_stats.frame_cpu_samples_ms.len() - 4096;
            self.perf_stats.frame_cpu_samples_ms.drain(0..drain);
        }
    }

    fn perf_record_cache_peak(&mut self) {
        self.perf_stats.peak_decoded_bytes =
            self.perf_stats.peak_decoded_bytes.max(self.cache.decoded_bytes());
        self.perf_stats.peak_texture_bytes =
            self.perf_stats.peak_texture_bytes.max(self.cache.texture_bytes());
    }

    fn current_inflight_estimated_bytes(&self) -> u64 {
        self.inflight_jobs.values().fold(0_u64, |acc, job| acc.saturating_add(job.estimated_bytes))
    }

    fn current_owned_bytes(&self) -> u64 {
        self.cache
            .decoded_bytes()
            .saturating_add(self.cache.texture_bytes())
            .saturating_add(self.current_inflight_estimated_bytes())
    }

    fn pressure_owned_bytes(&self) -> u64 {
        self.cache
            .decoded_bytes()
            .max(self.cache.texture_bytes())
            .saturating_add(self.current_inflight_estimated_bytes())
    }

    fn refresh_perf_memory_counters(&mut self) {
        let current_viewport_decoded_bytes = self.cache.decoded_bytes();
        let current_viewport_texture_bytes = self.cache.texture_bytes();
        let current_inflight_estimated_bytes = self.current_inflight_estimated_bytes();
        let current_owned_bytes = current_viewport_decoded_bytes
            .saturating_add(current_viewport_texture_bytes)
            .saturating_add(current_inflight_estimated_bytes);

        self.perf_stats.current_viewport_decoded_bytes = current_viewport_decoded_bytes;
        self.perf_stats.current_viewport_texture_bytes = current_viewport_texture_bytes;
        self.perf_stats.current_inflight_estimated_bytes = current_inflight_estimated_bytes;
        self.perf_stats.current_owned_bytes = current_owned_bytes;
        self.perf_stats.peak_owned_bytes =
            self.perf_stats.peak_owned_bytes.max(current_owned_bytes);
        self.perf_stats.end_owned_bytes = current_owned_bytes;
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn percentile(values: &[f32], percentile: f32) -> Option<f32> {
        if values.is_empty() {
            return None;
        }
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let index = ((sorted.len() as f32 - 1.0) * percentile).round() as usize;
        sorted.get(index).copied()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn perf_snapshot(&self) -> PerfSnapshot {
        let mut pressure_durations = self.pressure_durations_ms.clone();
        let active_elapsed = self.memory_pressure_started_at.elapsed().as_millis() as u64;
        match self.memory_pressure_state {
            MemoryPressureState::Normal => {
                pressure_durations.normal_ms =
                    pressure_durations.normal_ms.saturating_add(active_elapsed);
            }
            MemoryPressureState::Warm => {
                pressure_durations.warm_ms =
                    pressure_durations.warm_ms.saturating_add(active_elapsed);
            }
            MemoryPressureState::Hot => {
                pressure_durations.hot_ms =
                    pressure_durations.hot_ms.saturating_add(active_elapsed);
            }
            MemoryPressureState::Critical => {
                pressure_durations.critical_ms =
                    pressure_durations.critical_ms.saturating_add(active_elapsed);
            }
        }
        PerfSnapshot {
            first_lq_ms: self.perf_stats.first_lq_ready_ms,
            first_hq_ms: self.perf_stats.first_hq_ready_ms,
            p95_ui_frame_cpu_ms: Self::percentile(&self.perf_stats.frame_cpu_samples_ms, 0.95),
            p99_ui_frame_cpu_ms: Self::percentile(&self.perf_stats.frame_cpu_samples_ms, 0.99),
            peak_decoded_bytes: self.perf_stats.peak_decoded_bytes,
            peak_texture_bytes: self.perf_stats.peak_texture_bytes,
            viewport_cache_cap_bytes: self.cache.max_bytes(),
            lq_jobs_scheduled: self.perf_stats.lq_jobs_scheduled,
            hq_jobs_scheduled: self.perf_stats.hq_jobs_scheduled,
            jobs_canceled: self.perf_stats.jobs_canceled,
            peak_rss_bytes: self.perf_stats.peak_rss_bytes,
            end_rss_bytes: self.perf_stats.end_rss_bytes,
            pressure_state_durations_ms: pressure_durations,
            hq_suppression_count: self.perf_stats.hq_suppression_count,
            memory_pressure_state: self.memory_pressure_state,
            memory_budget_total_bytes: self.memory_budget.total_budget_bytes,
            current_viewport_decoded_bytes: self.perf_stats.current_viewport_decoded_bytes,
            current_viewport_texture_bytes: self.perf_stats.current_viewport_texture_bytes,
            current_inflight_estimated_bytes: self.perf_stats.current_inflight_estimated_bytes,
            current_owned_bytes: self.perf_stats.current_owned_bytes,
            peak_owned_bytes: self.perf_stats.peak_owned_bytes,
            end_owned_bytes: self.perf_stats.end_owned_bytes,
            active_target_bytes: self.perf_stats.active_target_bytes,
            idle_target_bytes: self.perf_stats.idle_target_bytes,
            visible_blank_frames: self.perf_stats.visible_blank_frames,
            visible_ultra_lq_frames: self.perf_stats.visible_ultra_lq_frames,
            visible_frame_samples: self.perf_stats.visible_frame_samples,
            hq_visible_latency_p95_ms: Self::percentile(
                &self.perf_stats.hq_visible_latency_samples_ms,
                0.95,
            ),
        }
    }

    #[allow(dead_code)]
    pub fn new(cx: &mut gpui::Context<Self>) -> Self {
        let memory_budget = AdaptiveMemoryBudget::detect();
        let preview_budget_bytes =
            SharedPreviewCache::preview_budget_bytes(memory_budget.total_budget_bytes);
        let preview_cache = Arc::new(Mutex::new(SharedPreviewCache::new(preview_budget_bytes)));
        Self::new_with_preview_cache(preview_cache, cx)
    }

    pub fn new_with_preview_cache(
        preview_cache: Arc<Mutex<SharedPreviewCache>>,
        cx: &mut gpui::Context<Self>,
    ) -> Self {
        if Self::wheel_debug_enabled() {
            Self::wheel_debug_log("[wheel] debug enabled v3");
        }
        let memory_budget = AdaptiveMemoryBudget::detect();
        Self {
            document: None,
            zoom_level: 100,
            zoom_mode: ZoomMode::FitPage,
            view_mode: ViewMode::SinglePage,
            current_page_index: 0,
            single_page_wheel_accum_px: 0.0,
            scroll_y: 0.0,
            page_layouts: Vec::new(),
            total_height: 0.0,
            cache: ByteLruCache::new(memory_budget.viewport_budget_bytes),
            preview_cache,
            display_pages: Vec::new(),
            viewport_height: 768.0,
            canvas_width: 1024.0,
            canvas_height: 768.0,
            focus_handle: cx.focus_handle(),
            scrollbar: ScrollbarController::new(),
            on_page_change: None,
            scale_factor: 1.0, // Updated from window.scale_factor() on first render
            doc_fingerprint: 0,
            render_generation: 1,
            next_job_id: 1,
            inflight_jobs: HashMap::new(),
            lq_queue: VecDeque::new(),
            hq_queue: VecDeque::new(),
            queued_lq_keys: HashSet::new(),
            queued_hq_keys: HashSet::new(),
            retry_state: HashMap::new(),
            last_scroll_activity: Instant::now(),
            scroll_direction: 0,
            last_hq_anchor_scroll_y: 0.0,
            perf_stats: PerfStats::default(),
            memory_budget,
            memory_targets: MemoryTargets::default(),
            memory_pressure_state: MemoryPressureState::Normal,
            memory_pressure_started_at: Instant::now(),
            pressure_durations_ms: PressureDurationsMs::default(),
            last_idle_trim_at: Instant::now(),
            hq_visible_pending_since: HashMap::new(),
            sidebar_thumbnail_backpressure: false,
        }
    }

    /// Set callback for page changes
    pub fn set_on_page_change<F>(&mut self, callback: F)
    where
        F: Fn(u16, &mut gpui::App) + 'static,
    {
        self.on_page_change = Some(Box::new(callback));
    }

    fn bump_generation(&mut self) {
        self.perf_stats.jobs_canceled = self.perf_stats.jobs_canceled.saturating_add(
            self.inflight_jobs.len() as u64
                + self.lq_queue.len() as u64
                + self.hq_queue.len() as u64,
        );
        self.render_generation = self.render_generation.wrapping_add(1);
        if self.render_generation == 0 {
            self.render_generation = 1;
        }
        self.inflight_jobs.clear();
        self.lq_queue.clear();
        self.hq_queue.clear();
        self.queued_lq_keys.clear();
        self.queued_hq_keys.clear();
    }

    fn note_scroll_activity(&mut self, delta_y: f32) {
        self.last_scroll_activity = Instant::now();
        self.scroll_direction = if delta_y > 0.1 {
            -1
        } else if delta_y < -0.1 {
            1
        } else {
            self.scroll_direction
        };
    }

    fn is_scroll_idle(&self) -> bool {
        self.last_scroll_activity.elapsed() >= SCROLL_IDLE_DEBOUNCE
    }

    fn is_idle_settled(&self) -> bool {
        self.last_scroll_activity.elapsed() >= IDLE_SETTLE_DEBOUNCE
    }

    pub fn is_scroll_active_for_performance(&self) -> bool {
        !self.is_scroll_idle()
    }

    pub fn memory_pressure_state(&self) -> MemoryPressureState {
        self.memory_pressure_state
    }

    fn current_render_buffer(&self) -> f32 {
        match self.memory_pressure_state {
            MemoryPressureState::Normal => RENDER_BUFFER_NORMAL,
            MemoryPressureState::Warm => RENDER_BUFFER_WARM,
            MemoryPressureState::Hot => RENDER_BUFFER_HOT,
            MemoryPressureState::Critical => RENDER_BUFFER_CRITICAL,
        }
    }

    fn transition_memory_pressure_state(&mut self, next: MemoryPressureState) {
        if next == self.memory_pressure_state {
            return;
        }

        let elapsed_ms = self.memory_pressure_started_at.elapsed().as_millis() as u64;
        match self.memory_pressure_state {
            MemoryPressureState::Normal => {
                self.pressure_durations_ms.normal_ms =
                    self.pressure_durations_ms.normal_ms.saturating_add(elapsed_ms);
            }
            MemoryPressureState::Warm => {
                self.pressure_durations_ms.warm_ms =
                    self.pressure_durations_ms.warm_ms.saturating_add(elapsed_ms);
            }
            MemoryPressureState::Hot => {
                self.pressure_durations_ms.hot_ms =
                    self.pressure_durations_ms.hot_ms.saturating_add(elapsed_ms);
            }
            MemoryPressureState::Critical => {
                self.pressure_durations_ms.critical_ms =
                    self.pressure_durations_ms.critical_ms.saturating_add(elapsed_ms);
            }
        }
        self.memory_pressure_state = next;
        self.memory_pressure_started_at = Instant::now();
    }

    fn derive_memory_pressure_state(&self, rss_ratio: f64, queue_hot: bool) -> MemoryPressureState {
        let mut target = if rss_ratio >= MEMORY_PRESSURE_HOT_MAX {
            MemoryPressureState::Critical
        } else if rss_ratio >= MEMORY_PRESSURE_WARM_MAX {
            MemoryPressureState::Hot
        } else if rss_ratio >= MEMORY_PRESSURE_NORMAL_MAX {
            MemoryPressureState::Warm
        } else {
            MemoryPressureState::Normal
        };

        if queue_hot && matches!(target, MemoryPressureState::Normal) {
            target = MemoryPressureState::Warm;
        }

        // Hysteresis when moving to a less severe state.
        match (self.memory_pressure_state, target) {
            (MemoryPressureState::Critical, MemoryPressureState::Hot)
                if rss_ratio > MEMORY_PRESSURE_HOT_MAX - MEMORY_PRESSURE_HYSTERESIS =>
            {
                MemoryPressureState::Critical
            }
            (MemoryPressureState::Hot, MemoryPressureState::Warm)
                if rss_ratio > MEMORY_PRESSURE_WARM_MAX - MEMORY_PRESSURE_HYSTERESIS =>
            {
                MemoryPressureState::Hot
            }
            (MemoryPressureState::Warm, MemoryPressureState::Normal)
                if rss_ratio > MEMORY_PRESSURE_NORMAL_MAX - MEMORY_PRESSURE_HYSTERESIS =>
            {
                MemoryPressureState::Warm
            }
            _ => target,
        }
    }

    fn visible_lq_estimate_bytes(&self) -> u64 {
        let visible_pages = self.strict_visible_page_indexes();
        self.page_layouts.iter().filter(|layout| visible_pages.contains(&layout.page_index)).fold(
            0_u64,
            |acc, layout| {
                let target_width = (layout.width * self.scale_factor).round().max(1.0) as u32;
                let target_height = (layout.height * self.scale_factor).round().max(1.0) as u32;
                let (render_width, render_height) =
                    Self::clamp_render_dimensions(target_width, target_height);
                let (lq_width, lq_height) =
                    Self::quality_scaled_dims(render_width, render_height, RenderQuality::LqScroll);
                let bytes = (lq_width as u64).saturating_mul(lq_height as u64).saturating_mul(4);
                acc.saturating_add(bytes)
            },
        )
    }

    fn update_memory_targets(&mut self) {
        let total_budget = self.memory_budget.total_budget_bytes.max(1);
        let inflight_estimated_bytes = self.current_inflight_estimated_bytes();
        let working_set_floor_bytes =
            self.visible_lq_estimate_bytes().saturating_add(inflight_estimated_bytes);

        let active_from_working_set = ((working_set_floor_bytes as f64)
            * ACTIVE_TARGET_WORKING_SET_MULTIPLIER)
            .round() as u64;
        let active_from_budget_floor =
            ((total_budget as f64) * ACTIVE_TARGET_BUDGET_FLOOR_RATIO).round() as u64;
        let active_target_bytes = active_from_working_set
            .max(active_from_budget_floor)
            .max(MIN_ACTIVE_TARGET_BYTES)
            .min(total_budget)
            .max(1);

        let idle_from_working_set = ((working_set_floor_bytes as f64) * 1.1).round() as u64;
        let idle_from_active = ((active_target_bytes as f64) * 0.65).round() as u64;
        let idle_target_bytes =
            idle_from_working_set.max(idle_from_active).min(active_target_bytes).max(1);

        let pressure_budget_bytes =
            if self.is_scroll_idle() { idle_target_bytes } else { active_target_bytes };

        self.memory_targets = MemoryTargets {
            working_set_floor_bytes,
            active_target_bytes,
            idle_target_bytes,
            pressure_budget_bytes,
        };
        self.perf_stats.active_target_bytes = active_target_bytes;
        self.perf_stats.idle_target_bytes = idle_target_bytes;
    }

    fn maybe_trim_idle_cache(&mut self) {
        if !self.is_idle_settled() {
            return;
        }
        if self.last_idle_trim_at.elapsed() < IDLE_TRIM_COOLDOWN {
            return;
        }
        self.last_idle_trim_at = Instant::now();

        let idle_target_bytes = self.memory_targets.idle_target_bytes.max(1);
        if self.current_owned_bytes() <= idle_target_bytes {
            return;
        }

        let strict_visible = self.strict_visible_page_indexes();
        let hq_keep_pages = if matches!(
            self.memory_pressure_state,
            MemoryPressureState::Hot | MemoryPressureState::Critical
        ) {
            strict_visible.clone()
        } else {
            self.hq_target_page_indexes(&strict_visible)
        };
        let within_render_window = self.page_indexes_within_window(self.current_render_buffer());
        let hq_pages_in_window: HashSet<u16> = self
            .cache
            .keys()
            .into_iter()
            .filter(|key| {
                key.doc_fingerprint == self.doc_fingerprint
                    && matches!(key.quality, RenderQuality::HqFinal)
                    && within_render_window.contains(&key.page_index)
            })
            .map(|key| key.page_index)
            .collect();
        let hq_pages_all: HashSet<u16> = self
            .cache
            .keys()
            .into_iter()
            .filter(|key| {
                key.doc_fingerprint == self.doc_fingerprint
                    && matches!(key.quality, RenderQuality::HqFinal)
            })
            .map(|key| key.page_index)
            .collect();

        // Always drop redundant LQ copies when HQ for the same page exists.
        while self.cache.evict_one_where(|key, _| {
            key.doc_fingerprint == self.doc_fingerprint
                && matches!(key.quality, RenderQuality::LqScroll)
                && hq_pages_all.contains(&key.page_index)
        }) {}

        // Keep HQ only for strict-visible pages once fully settled.
        while self.cache.evict_one_where(|key, _| {
            key.doc_fingerprint == self.doc_fingerprint
                && matches!(key.quality, RenderQuality::HqFinal)
                && !hq_keep_pages.contains(&key.page_index)
        }) {}

        loop {
            if self.current_owned_bytes() <= idle_target_bytes {
                break;
            }

            let evicted_duplicate_lq = self.cache.evict_one_where(|key, _| {
                if key.doc_fingerprint != self.doc_fingerprint
                    || !matches!(key.quality, RenderQuality::LqScroll)
                    || !within_render_window.contains(&key.page_index)
                {
                    return false;
                }
                hq_pages_in_window.contains(&key.page_index)
            });
            if evicted_duplicate_lq {
                continue;
            }

            let evicted_hq = self.cache.evict_one_where(|key, _| {
                key.doc_fingerprint == self.doc_fingerprint
                    && matches!(key.quality, RenderQuality::HqFinal)
                    && !hq_keep_pages.contains(&key.page_index)
            });
            if evicted_hq {
                continue;
            }

            let evicted_offscreen_lq = self.cache.evict_one_where(|key, _| {
                key.doc_fingerprint == self.doc_fingerprint
                    && matches!(key.quality, RenderQuality::LqScroll | RenderQuality::LqThumb)
                    && !strict_visible.contains(&key.page_index)
                    && !within_render_window.contains(&key.page_index)
            });
            if !evicted_offscreen_lq {
                break;
            }
        }
    }

    fn evaluate_memory_pressure(&mut self) {
        self.refresh_perf_memory_counters();
        self.update_memory_targets();

        let total_budget = self.memory_budget.total_budget_bytes.max(1);
        let pressure_budget = self.memory_targets.pressure_budget_bytes.max(1);
        let owned_ratio = self.pressure_owned_bytes() as f64 / pressure_budget as f64;

        let rss_ratio = if let Some(rss_bytes) = process_memory::current_rss_bytes() {
            self.perf_stats.peak_rss_bytes = self.perf_stats.peak_rss_bytes.max(rss_bytes);
            self.perf_stats.end_rss_bytes = rss_bytes;
            rss_bytes as f64 / total_budget as f64
        } else {
            0.0
        };

        let queue_depth = self.lq_queue.len() + self.hq_queue.len() + self.inflight_jobs.len();
        let queue_hot = queue_depth > (MAX_QUEUED_LQ_JOBS / 2);
        let effective_ratio = owned_ratio.max(rss_ratio);
        let next = self.derive_memory_pressure_state(effective_ratio, queue_hot);
        self.transition_memory_pressure_state(next);

        self.maybe_trim_idle_cache();
        self.refresh_perf_memory_counters();
    }

    fn dpr_bucket(&self) -> u16 {
        (self.scale_factor * 100.0).round().max(1.0) as u16
    }

    fn cache_key(&self, page_index: u16, quality: RenderQuality) -> RenderCacheKey {
        RenderCacheKey::new(
            self.doc_fingerprint,
            page_index,
            self.zoom_level,
            0,
            quality,
            self.dpr_bucket(),
        )
    }

    fn quality_scaled_dims(width: u32, height: u32, quality: RenderQuality) -> (u32, u32) {
        let scale = match quality {
            RenderQuality::LqThumb => 0.25,
            RenderQuality::LqScroll => 0.5,
            RenderQuality::HqFinal => 1.0,
        };
        let w = ((width as f32) * scale).round().max(1.0) as u32;
        let h = ((height as f32) * scale).round().max(1.0) as u32;
        (w, h)
    }

    fn clamp_render_dimensions(width: u32, height: u32) -> (u32, u32) {
        let mut width = width.max(1);
        let mut height = height.max(1);

        if width > MAX_RENDER_EDGE_PX || height > MAX_RENDER_EDGE_PX {
            let edge_scale = (MAX_RENDER_EDGE_PX as f32 / width as f32)
                .min(MAX_RENDER_EDGE_PX as f32 / height as f32);
            width = ((width as f32) * edge_scale).round().max(1.0) as u32;
            height = ((height as f32) * edge_scale).round().max(1.0) as u32;
        }

        let max_pixels = MAX_RENDER_MEGAPIXELS * 1_000_000;
        let pixels = width as u64 * height as u64;
        if pixels > max_pixels {
            let area_scale = (max_pixels as f64 / pixels as f64).sqrt() as f32;
            width = ((width as f32) * area_scale).round().max(1.0) as u32;
            height = ((height as f32) * area_scale).round().max(1.0) as u32;
        }

        (width, height)
    }

    fn request_inflight_estimated_bytes(request: &RenderRequest) -> u64 {
        (request.render_width as u64).saturating_mul(request.render_height as u64).saturating_mul(4)
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
        self.memory_budget = AdaptiveMemoryBudget::detect();
        self.memory_targets = MemoryTargets::default();
        self.memory_pressure_state = MemoryPressureState::Normal;
        self.memory_pressure_started_at = Instant::now();
        self.last_idle_trim_at = Instant::now();
        self.doc_fingerprint = Arc::as_ptr(&doc) as usize as u64;
        self.bump_generation();
        self.perf_reset();
        self.document = Some(doc);
        self.current_page_index = 0;
        self.single_page_wheel_accum_px = 0.0;
        self.scroll_y = 0.0;
        self.last_hq_anchor_scroll_y = 0.0;
        self.hq_visible_pending_since.clear();
        self.note_scroll_activity(0.0);
        self.sync_scroll_handle_to_state();
        self.cache = ByteLruCache::new(self.memory_budget.viewport_budget_bytes);
        self.retry_state.clear();
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
        self.dispatch_render_jobs(cx);
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
            self.sync_scroll_handle_to_state();
            self.update_visible_pages();
            self.dispatch_render_jobs(cx);
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

    pub fn set_sidebar_thumbnail_backpressure(&mut self, enabled: bool) {
        self.sidebar_thumbnail_backpressure = enabled;
    }

    pub fn zoom_mode(&self) -> ZoomMode {
        self.zoom_mode
    }

    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    pub fn set_view_mode(&mut self, mode: ViewMode, cx: &mut gpui::Context<Self>) {
        if self.view_mode == mode {
            return;
        }

        let previous_mode = self.view_mode;
        let previous_scroll_y = self.scroll_y;
        let previous_page = self.current_page().min(self.page_count().saturating_sub(1));

        self.view_mode = mode;
        self.bump_generation();
        self.current_page_index = previous_page;
        self.single_page_wheel_accum_px = 0.0;
        self.note_scroll_activity(0.0);

        if let Some(layout) = self.page_layouts.get(self.current_page_index as usize) {
            self.scroll_y = match (previous_mode, self.view_mode) {
                (ViewMode::SinglePage, ViewMode::Continuous) => {
                    continuous_scroll_for_single_page(layout.y_offset, previous_scroll_y)
                }
                (ViewMode::Continuous, ViewMode::SinglePage) => {
                    single_page_scroll_for_continuous(layout.y_offset, previous_scroll_y)
                }
                _ => previous_scroll_y,
            };
        }

        self.clamp_scroll();
        self.sync_scroll_handle_to_state();
        self.update_visible_pages();
        self.dispatch_render_jobs(cx);
        cx.notify();
    }

    /// Get page count
    pub fn page_count(&self) -> u16 {
        self.document.as_ref().map(|d| d.page_count()).unwrap_or(0)
    }

    /// Get current page based on scroll position (0-based)
    pub fn current_page(&self) -> u16 {
        if self.view_mode == ViewMode::SinglePage {
            return self.current_page_index;
        }
        self.page_for_scroll()
    }

    fn page_for_scroll(&self) -> u16 {
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
        if self.page_count() == 0 {
            return;
        }

        let old_page = self.current_page();
        let old_scroll = self.scroll_y;
        let target = page.min(self.page_count().saturating_sub(1));
        self.current_page_index = target;

        if matches!(self.view_mode, ViewMode::Continuous) {
            if let Some(layout) = self.page_layouts.get(target as usize) {
                self.scroll_y = layout.y_offset;
            }
        } else {
            self.scroll_y = 0.0;
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

        self.clamp_scroll();
        self.note_scroll_activity(self.scroll_y - old_scroll);
        self.sync_scroll_handle_to_state();
        self.update_visible_pages();
        self.dispatch_render_jobs(cx);

        let new_page = self.current_page();
        if new_page != old_page {
            if let Some(callback) = &self.on_page_change {
                callback(new_page, cx);
            }
        }
        cx.notify();
    }

    pub fn scroll_by_pixels(&mut self, delta_pixels: f32, cx: &mut gpui::Context<Self>) {
        if self.document.is_none() || !matches!(self.view_mode, ViewMode::Continuous) {
            return;
        }
        let old_scroll = self.scroll_y;
        self.scroll_y = (self.scroll_y + delta_pixels).max(0.0);
        self.clamp_scroll();
        if (self.scroll_y - old_scroll).abs() < 0.5 {
            return;
        }
        self.note_scroll_activity(self.scroll_y - old_scroll);
        self.sync_scroll_handle_to_state();
        self.update_visible_pages();
        self.dispatch_render_jobs(cx);
        cx.notify();
    }

    pub fn benchmark_jump_to_fraction(
        &mut self,
        fraction: f32,
        cx: &mut gpui::Context<Self>,
    ) {
        if self.document.is_none() || !matches!(self.view_mode, ViewMode::Continuous) {
            return;
        }
        let clamped_fraction = fraction.clamp(0.0, 1.0);
        let max_scroll = (self.total_height - self.viewport_height).max(0.0);
        let target_scroll = max_scroll * clamped_fraction;
        let old_scroll = self.scroll_y;
        self.scroll_y = target_scroll;
        self.clamp_scroll();
        self.note_scroll_activity(self.scroll_y - old_scroll);
        self.sync_scroll_handle_to_state();
        self.update_visible_pages();
        self.dispatch_render_jobs(cx);
        cx.notify();
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
            self.dispatch_render_jobs(cx);
            cx.notify();
        }
    }

    pub fn reset_zoom(&mut self, cx: &mut gpui::Context<Self>) {
        self.zoom_mode = ZoomMode::Percent;
        if self.set_zoom_internal(100) {
            self.dispatch_render_jobs(cx);
            cx.notify();
        }
    }

    pub fn fit_width(&mut self, cx: &mut gpui::Context<Self>) {
        let mode_changed = self.zoom_mode != ZoomMode::FitWidth;
        self.zoom_mode = ZoomMode::FitWidth;
        let zoom_changed = self.apply_fit_width_zoom();
        if mode_changed || zoom_changed {
            self.dispatch_render_jobs(cx);
            cx.notify();
        }
    }

    pub fn fit_page(&mut self, cx: &mut gpui::Context<Self>) {
        let mode_changed = self.zoom_mode != ZoomMode::FitPage;
        self.zoom_mode = ZoomMode::FitPage;
        let zoom_changed = self.apply_fit_page_zoom();
        if mode_changed || zoom_changed {
            self.dispatch_render_jobs(cx);
            cx.notify();
        }
    }

    /// Zoom in by 25%
    pub fn zoom_in(&mut self, cx: &mut gpui::Context<Self>) {
        self.zoom_mode = ZoomMode::Percent;
        if self.set_zoom_internal(self.zoom_level + 25) {
            self.dispatch_render_jobs(cx);
            cx.notify();
        }
    }

    /// Zoom out by 25%
    pub fn zoom_out(&mut self, cx: &mut gpui::Context<Self>) {
        self.zoom_mode = ZoomMode::Percent;
        if self.set_zoom_internal(self.zoom_level.saturating_sub(25)) {
            self.dispatch_render_jobs(cx);
            cx.notify();
        }
    }

    fn current_page_size_points(&self) -> Option<(f32, f32)> {
        let doc = self.document.as_ref()?;
        let page_index = self.current_page();
        let dims = doc.page_dimensions(page_index).ok()?;
        Some((dims.width, dims.height))
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

        let page_before_zoom = self.current_page();
        self.bump_generation();
        // Preserve relative scroll position
        let scroll_ratio =
            if self.total_height > 0.0 { self.scroll_y / self.total_height } else { 0.0 };

        self.zoom_level = new_zoom;
        self.compute_layout();
        self.current_page_index = page_before_zoom.min(self.page_count().saturating_sub(1));

        // Restore relative position in continuous mode.
        self.scroll_y = if matches!(self.view_mode, ViewMode::Continuous) {
            scroll_ratio * self.total_height
        } else {
            self.scroll_y
        };
        self.clamp_scroll();
        self.note_scroll_activity(0.0);
        self.sync_scroll_handle_to_state();
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
            if let Ok(page) = doc.page_dimensions(page_index) {
                let width = page.width * zoom_factor;
                let height = page.height * zoom_factor;

                self.page_layouts.push(PageLayout { page_index, width, height, y_offset });

                y_offset += height + PAGE_GAP;
            }
        }

        self.total_height = y_offset;
    }

    fn page_indexes_within_window(&self, buffer: f32) -> HashSet<u16> {
        if matches!(self.view_mode, ViewMode::SinglePage) {
            return HashSet::from([self.current_page_index]);
        }
        let start = self.scroll_y - buffer;
        let end = self.scroll_y + self.viewport_height + buffer;
        self.page_layouts
            .iter()
            .filter(|layout| {
                let page_end = layout.y_offset + layout.height;
                page_end >= start && layout.y_offset <= end
            })
            .map(|layout| layout.page_index)
            .collect()
    }

    fn prune_cache_by_pressure(&mut self) {
        let keep_pages = match self.memory_pressure_state {
            MemoryPressureState::Normal => return,
            MemoryPressureState::Warm => self.page_indexes_within_window(RENDER_BUFFER_NORMAL),
            MemoryPressureState::Hot => self.strict_visible_page_indexes(),
            MemoryPressureState::Critical => self.strict_visible_page_indexes(),
        };

        let critical = matches!(self.memory_pressure_state, MemoryPressureState::Critical);
        self.cache.retain(|key, _| {
            if key.doc_fingerprint != self.doc_fingerprint {
                return false;
            }
            if !keep_pages.contains(&key.page_index) {
                return false;
            }
            if critical
                && !matches!(key.quality, RenderQuality::LqScroll | RenderQuality::LqThumb)
            {
                return false;
            }
            true
        });
    }

    fn visible_layouts(&self) -> Vec<PageLayout> {
        match self.view_mode {
            ViewMode::Continuous => {
                let render_buffer = self.current_render_buffer();
                let visible_start = self.scroll_y - render_buffer;
                let visible_end = self.scroll_y + self.viewport_height + render_buffer;
                self.page_layouts
                    .iter()
                    .filter(|layout| {
                        let page_end = layout.y_offset + layout.height;
                        page_end >= visible_start && layout.y_offset <= visible_end
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            }
            ViewMode::SinglePage => self
                .page_layouts
                .get(self.current_page_index as usize)
                .map(|layout| PageLayout {
                    page_index: layout.page_index,
                    width: layout.width,
                    height: layout.height,
                    y_offset: PAGE_GAP,
                })
                .into_iter()
                .collect::<Vec<_>>(),
        }
    }

    fn strict_visible_page_indexes(&self) -> HashSet<u16> {
        match self.view_mode {
            ViewMode::SinglePage => HashSet::from([self.current_page_index]),
            ViewMode::Continuous => {
                let start = self.scroll_y;
                let end = self.scroll_y + self.viewport_height;
                self.page_layouts
                    .iter()
                    .filter(|layout| {
                        let page_end = layout.y_offset + layout.height;
                        page_end >= start && layout.y_offset <= end
                    })
                    .map(|layout| layout.page_index)
                    .collect()
            }
        }
    }

    fn hq_target_page_indexes(&self, strict_visible: &HashSet<u16>) -> HashSet<u16> {
        let total_pages = self.page_layouts.len() as i32;
        let mut targets = strict_visible.clone();
        for page in strict_visible {
            let page = *page as i32;
            for delta in -HQ_RING_RADIUS..=HQ_RING_RADIUS {
                let candidate = page + delta;
                if candidate >= 0 && candidate < total_pages {
                    targets.insert(candidate as u16);
                }
            }
        }
        targets
    }

    fn preview_image(&mut self, page_index: u16) -> Option<Arc<gpui::RenderImage>> {
        self.preview_cache
            .lock()
            .ok()
            .and_then(|mut cache| cache.get(self.doc_fingerprint, page_index))
    }

    fn visible_lq_debt_count(&self, strict_visible: &HashSet<u16>) -> usize {
        strict_visible
            .iter()
            .filter(|page_index| {
                let hq_key = self.cache_key(**page_index, RenderQuality::HqFinal);
                let lq_key = self.cache_key(**page_index, RenderQuality::LqScroll);
                let has_cached = self.cache.contains(&hq_key) || self.cache.contains(&lq_key);
                let has_inflight =
                    self.inflight_jobs.contains_key(&hq_key) || self.inflight_jobs.contains_key(&lq_key);
                !has_cached && !has_inflight
            })
            .count()
    }

    /// Update visible pages and rebuild the current render queues.
    fn update_visible_pages(&mut self) {
        if self.document.is_none() {
            self.display_pages.clear();
            self.lq_queue.clear();
            self.hq_queue.clear();
            self.queued_lq_keys.clear();
            self.queued_hq_keys.clear();
            return;
        }

        self.prune_cache_by_pressure();

        let visible_layouts = self.visible_layouts();
        let idle = self.is_scroll_idle();
        let strict_visible = self.strict_visible_page_indexes();
        let hq_target_pages = self.hq_target_page_indexes(&strict_visible);
        let pressure = self.memory_pressure_state;
        let lq_debt = self.visible_lq_debt_count(&strict_visible);
        let allow_hq_visible = !matches!(pressure, MemoryPressureState::Critical)
            && (idle
                || (matches!(pressure, MemoryPressureState::Normal | MemoryPressureState::Warm)
                    && lq_debt <= 1));
        let allow_hq_ring =
            idle && matches!(pressure, MemoryPressureState::Normal | MemoryPressureState::Warm);
        let should_preserve_hq =
            (self.scroll_y - self.last_hq_anchor_scroll_y).abs() <= MICRO_SCROLL_HYSTERESIS_PX;
        let now = Instant::now();
        self.hq_visible_pending_since
            .retain(|page_index, _| strict_visible.contains(page_index));
        for page_index in &strict_visible {
            self.hq_visible_pending_since.entry(*page_index).or_insert(now);
        }

        self.lq_queue.clear();
        self.hq_queue.clear();
        self.queued_lq_keys.clear();
        self.queued_hq_keys.clear();

        let mut new_display = Vec::with_capacity(visible_layouts.len());

        for layout in &visible_layouts {
            let hq_key = self.cache_key(layout.page_index, RenderQuality::HqFinal);
            let lq_key = self.cache_key(layout.page_index, RenderQuality::LqScroll);
            let hq_cached = self.cache.get(&hq_key);
            let lq_cached = self.cache.get(&lq_key);
            let has_lq_cached = lq_cached.is_some();
            let preview_cached = if hq_cached.is_none() && lq_cached.is_none() {
                self.preview_image(layout.page_index)
            } else {
                None
            };

            let strict_visible_page = strict_visible.contains(&layout.page_index);
            let hq_target_page = hq_target_pages.contains(&layout.page_index);
            let can_upgrade_page = if strict_visible_page {
                allow_hq_visible
            } else {
                allow_hq_ring && hq_target_page
            };

            let (image, state) = if let Some(hq) = hq_cached {
                self.last_hq_anchor_scroll_y = self.scroll_y;
                if strict_visible_page {
                    if let Some(since) = self.hq_visible_pending_since.remove(&layout.page_index) {
                        let latency_ms = since.elapsed().as_secs_f32() * 1000.0;
                        self.perf_stats.hq_visible_latency_samples_ms.push(latency_ms);
                        if self.perf_stats.hq_visible_latency_samples_ms.len() > 4096 {
                            let drain = self.perf_stats.hq_visible_latency_samples_ms.len() - 4096;
                            self.perf_stats.hq_visible_latency_samples_ms.drain(0..drain);
                        }
                    }
                }
                (Some(hq.image), PageQualityState::HqReady)
            } else if let Some(lq) = lq_cached {
                let state = if can_upgrade_page || should_preserve_hq {
                    PageQualityState::Upgrading
                } else {
                    PageQualityState::LqReady
                };
                (Some(lq.image), state)
            } else if let Some(preview) = preview_cached {
                let state =
                    if can_upgrade_page { PageQualityState::Upgrading } else { PageQualityState::UltraLqReady };
                (Some(preview), state)
            } else {
                (None, PageQualityState::Skeleton)
            };

            if image.is_none() {
                self.enqueue_page_request(layout, RenderQuality::LqThumb);
            }
            if !has_lq_cached && (strict_visible_page || idle) {
                self.enqueue_page_request(layout, RenderQuality::LqScroll);
            }
            if can_upgrade_page && hq_target_page {
                self.enqueue_page_request(layout, RenderQuality::HqFinal);
            } else if strict_visible_page {
                self.perf_stats.hq_suppression_count =
                    self.perf_stats.hq_suppression_count.saturating_add(1);
            }

            new_display.push(DisplayPage {
                page_index: layout.page_index,
                width: layout.width.max(1.0).round() as u32,
                height: layout.height.max(1.0).round() as u32,
                y_offset: layout.y_offset,
                image,
                quality_state: state,
            });
        }

        if allow_hq_ring {
            let visible_page_indexes: HashSet<u16> =
                visible_layouts.iter().map(|layout| layout.page_index).collect();
            for target_page in hq_target_pages {
                if visible_page_indexes.contains(&target_page)
                    || strict_visible.contains(&target_page)
                {
                    continue;
                }
                if let Some(layout) = self.page_layouts.get(target_page as usize).cloned() {
                    self.enqueue_page_request(&layout, RenderQuality::HqFinal);
                }
            }
        }

        let strict_display_pages = new_display
            .iter()
            .filter(|page| strict_visible.contains(&page.page_index))
            .collect::<Vec<_>>();
        if !strict_display_pages.is_empty() {
            self.perf_stats.visible_frame_samples =
                self.perf_stats.visible_frame_samples.saturating_add(1);
            if strict_display_pages
                .iter()
                .any(|page| {
                    page.image.is_none() && !matches!(page.quality_state, PageQualityState::Skeleton)
                })
            {
                self.perf_stats.visible_blank_frames =
                    self.perf_stats.visible_blank_frames.saturating_add(1);
            }
            if strict_display_pages
                .iter()
                .any(|page| matches!(page.quality_state, PageQualityState::UltraLqReady))
            {
                self.perf_stats.visible_ultra_lq_frames =
                    self.perf_stats.visible_ultra_lq_frames.saturating_add(1);
            }
        }

        new_display.sort_by(|a, b| a.y_offset.partial_cmp(&b.y_offset).unwrap());
        self.display_pages = new_display;
    }

    fn enqueue_page_request(&mut self, layout: &PageLayout, quality: RenderQuality) {
        if matches!(quality, RenderQuality::LqThumb) {
            let already_previewed = self
                .preview_cache
                .lock()
                .ok()
                .map(|cache| cache.contains(self.doc_fingerprint, layout.page_index))
                .unwrap_or(false);
            if already_previewed {
                return;
            }
        }

        let key = self.cache_key(layout.page_index, quality);
        if self.cache.contains(&key) || self.inflight_jobs.contains_key(&key) {
            return;
        }

        let now = Instant::now();
        if matches!(quality, RenderQuality::HqFinal) {
            if let Some(retry) = self.retry_state.get(&key) {
                if retry.suppressed_generation == Some(self.render_generation) {
                    return;
                }
                if now < retry.next_retry_at {
                    return;
                }
            }
        }

        let target_width = (layout.width * self.scale_factor).round().max(1.0) as u32;
        let target_height = (layout.height * self.scale_factor).round().max(1.0) as u32;
        let (render_width, render_height) =
            Self::clamp_render_dimensions(target_width, target_height);
        let request = RenderRequest {
            key,
            page_index: layout.page_index,
            quality,
            generation: self.render_generation,
            render_width,
            render_height,
            display_width: layout.width.max(1.0).round() as u32,
            display_height: layout.height.max(1.0).round() as u32,
        };

        match quality {
            RenderQuality::LqScroll => {
                if self.queued_lq_keys.contains(&request.key)
                    || self.lq_queue.len() >= MAX_QUEUED_LQ_JOBS
                {
                    return;
                }
                self.queued_lq_keys.insert(request.key);
                self.lq_queue.push_back(request);
            }
            RenderQuality::HqFinal => {
                if self.queued_hq_keys.contains(&request.key)
                    || self.hq_queue.len() >= MAX_QUEUED_HQ_JOBS
                {
                    return;
                }
                self.queued_hq_keys.insert(request.key);
                self.hq_queue.push_back(request);
            }
            RenderQuality::LqThumb => {
                if self.queued_lq_keys.contains(&request.key)
                    || self.lq_queue.len() >= MAX_QUEUED_LQ_JOBS
                {
                    return;
                }
                self.queued_lq_keys.insert(request.key);
                self.lq_queue.push_back(request);
            }
        }
    }

    fn next_hq_request(&mut self) -> Option<RenderRequest> {
        while let Some(request) = self.hq_queue.pop_front() {
            self.queued_hq_keys.remove(&request.key);
            if request.generation != self.render_generation {
                continue;
            }
            if self.cache.contains(&request.key) || self.inflight_jobs.contains_key(&request.key) {
                continue;
            }
            return Some(request);
        }
        None
    }

    fn next_lq_request(&mut self) -> Option<RenderRequest> {
        while let Some(request) = self.lq_queue.pop_front() {
            self.queued_lq_keys.remove(&request.key);
            if request.generation != self.render_generation {
                continue;
            }
            if self.cache.contains(&request.key) || self.inflight_jobs.contains_key(&request.key) {
                continue;
            }
            return Some(request);
        }
        None
    }

    /// Dispatch queued render jobs under per-frame budget rules.
    fn dispatch_render_jobs(&mut self, cx: &mut gpui::Context<Self>) {
        if self.document.is_none() {
            return;
        }

        if matches!(self.memory_pressure_state, MemoryPressureState::Critical) {
            if !self.hq_queue.is_empty() {
                self.perf_stats.hq_suppression_count =
                    self.perf_stats.hq_suppression_count.saturating_add(self.hq_queue.len() as u64);
            }
            self.hq_queue.clear();
            self.queued_hq_keys.clear();
        }

        let mut started_lq = 0;
        while started_lq < MAX_LQ_JOBS_PER_FRAME {
            let Some(request) = self.next_lq_request() else {
                break;
            };
            self.perf_stats.lq_jobs_scheduled = self.perf_stats.lq_jobs_scheduled.saturating_add(1);
            self.spawn_render_job(request, cx);
            started_lq += 1;
        }

        let can_dispatch_hq = if self.is_scroll_idle() {
            !self.sidebar_thumbnail_backpressure
                && !matches!(self.memory_pressure_state, MemoryPressureState::Critical)
        } else {
            let strict_visible = self.strict_visible_page_indexes();
            let lq_debt = self.visible_lq_debt_count(&strict_visible);
            lq_debt == 0
                && !self.sidebar_thumbnail_backpressure
                && matches!(self.memory_pressure_state, MemoryPressureState::Normal | MemoryPressureState::Warm)
        };
        if !can_dispatch_hq {
            return;
        }

        let mut started_hq = 0;
        while started_hq < MAX_HQ_JOBS_PER_FRAME {
            let Some(request) = self.next_hq_request() else {
                break;
            };
            self.perf_stats.hq_jobs_scheduled =
                self.perf_stats.hq_jobs_scheduled.saturating_add(1);
            self.spawn_render_job(request, cx);
            started_hq += 1;
        }
    }

    fn spawn_render_job(&mut self, request: RenderRequest, cx: &mut gpui::Context<Self>) {
        let Some(doc) = self.document.clone() else {
            return;
        };
        if self.inflight_jobs.contains_key(&request.key) {
            return;
        }

        let job_id = self.next_job_id;
        self.next_job_id = self.next_job_id.wrapping_add(1);
        let estimated_bytes = Self::request_inflight_estimated_bytes(&request);
        self.inflight_jobs.insert(
            request.key,
            InflightJob { job_id, generation: request.generation, estimated_bytes },
        );

        cx.spawn(move |this: gpui::WeakEntity<PdfViewport>, cx: &mut gpui::AsyncApp| {
            let mut async_cx = cx.clone();
            let request_for_render = request.clone();
            async move {
                let render_result = async_cx
                    .background_executor()
                    .spawn(async move {
                        doc.render_page_rgba_with_quality(
                            request_for_render.page_index,
                            request_for_render.render_width,
                            request_for_render.render_height,
                            request_for_render.quality,
                        )
                    })
                    .await;

                let _ = this.update(&mut async_cx, move |viewport, cx| {
                    viewport.finish_render_job(request, job_id, render_result, cx);
                });
            }
        })
        .detach();
    }

    fn retry_backoff(attempt: u8) -> Duration {
        match attempt {
            1 => Duration::from_millis(250),
            2 => Duration::from_millis(500),
            _ => Duration::from_millis(1000),
        }
    }

    fn register_render_failure(&mut self, key: RenderCacheKey, quality: RenderQuality) {
        if !matches!(quality, RenderQuality::HqFinal) {
            return;
        }
        let now = Instant::now();
        let state = self.retry_state.entry(key).or_insert(RetryState {
            attempts: 0,
            next_retry_at: now,
            suppressed_generation: None,
        });
        state.attempts = state.attempts.saturating_add(1);
        state.next_retry_at = now + Self::retry_backoff(state.attempts);
        if state.attempts >= 3 {
            state.suppressed_generation = Some(self.render_generation);
        }
    }

    fn finish_render_job(
        &mut self,
        request: RenderRequest,
        job_id: u64,
        result: Result<Vec<u8>, butterpaper_render::PdfError>,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(inflight) = self.inflight_jobs.get(&request.key).copied() else {
            return;
        };
        if inflight.job_id != job_id {
            return;
        }
        self.inflight_jobs.remove(&request.key);

        if inflight.generation != self.render_generation {
            return;
        }

        match result {
            Ok(rgba_pixels) => {
                let (pixel_width, pixel_height) = Self::quality_scaled_dims(
                    request.render_width,
                    request.render_height,
                    request.quality,
                );
                if let Some(image) = create_render_image(rgba_pixels, pixel_width, pixel_height) {
                    let cached = CachedImage::from_image(
                        image,
                        pixel_width,
                        pixel_height,
                        request.display_width,
                        request.display_height,
                    );
                    let inserted = if matches!(request.quality, RenderQuality::LqThumb) {
                        if let Ok(mut preview_cache) = self.preview_cache.lock() {
                            preview_cache.insert(self.doc_fingerprint, request.page_index, cached);
                        }
                        true
                    } else {
                        self.cache.insert(request.key, cached, Some(RenderQuality::HqFinal))
                    };
                    if inserted {
                        self.perf_record_cache_peak();
                        match request.quality {
                            RenderQuality::LqScroll => {
                                if self.perf_stats.first_lq_ready_ms.is_none() {
                                    self.perf_stats.first_lq_ready_ms = self.perf_elapsed_ms();
                                }
                            }
                            RenderQuality::HqFinal => {
                                if self.perf_stats.first_hq_ready_ms.is_none() {
                                    self.perf_stats.first_hq_ready_ms = self.perf_elapsed_ms();
                                }
                            }
                            RenderQuality::LqThumb => {
                                if self.perf_stats.first_lq_ready_ms.is_none() {
                                    self.perf_stats.first_lq_ready_ms = self.perf_elapsed_ms();
                                }
                            }
                        }
                    }
                    self.retry_state.remove(&request.key);
                } else {
                    self.register_render_failure(request.key, request.quality);
                }
            }
            Err(_error) => {
                self.register_render_failure(request.key, request.quality);
            }
        }

        self.update_visible_pages();
        self.dispatch_render_jobs(cx);
        cx.notify();
    }

    fn single_page_max_scroll(&self) -> f32 {
        let Some(layout) = self.page_layouts.get(self.current_page_index as usize) else {
            return 0.0;
        };
        (layout.height + PAGE_GAP * 2.0 - self.viewport_height).max(0.0)
    }

    fn sync_scroll_from_handle(&mut self, cx: &mut gpui::Context<Self>) {
        let old_scroll = self.scroll_y;
        self.scroll_y = (-self.scrollbar.offset_y()).max(0.0);
        self.clamp_scroll();
        if (self.scroll_y - old_scroll).abs() < 0.5 {
            return;
        }
        self.note_scroll_activity(self.scroll_y - old_scroll);

        if matches!(self.view_mode, ViewMode::SinglePage) {
            self.update_visible_pages();
            self.dispatch_render_jobs(cx);
            cx.notify();
            return;
        }

        let old_page = self.current_page();
        self.update_visible_pages();
        self.dispatch_render_jobs(cx);

        // Notify page change
        let new_page = self.current_page();
        self.current_page_index = new_page;
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

    fn sync_scroll_handle_to_state(&self) {
        self.scrollbar.set_offset_y(-self.scroll_y);
    }

    fn start_scrollbar_drag(&mut self, mouse_y_window: f32, cx: &mut gpui::Context<Self>) {
        if matches!(self.view_mode, ViewMode::SinglePage) {
            return;
        }
        if self.scrollbar.start_drag(mouse_y_window) {
            self.sync_scroll_from_handle(cx);
        }
    }

    fn update_scrollbar_drag(&mut self, mouse_y_window: f32, cx: &mut gpui::Context<Self>) {
        if matches!(self.view_mode, ViewMode::SinglePage) {
            return;
        }
        if self.scrollbar.update_drag(mouse_y_window) {
            self.sync_scroll_from_handle(cx);
        }
    }

    fn end_scrollbar_drag(&mut self) {
        self.scrollbar.end_drag();
    }

    fn handle_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let delta_y = event.delta.pixel_delta(window.line_height()).y.0;
        self.note_scroll_activity(delta_y);
        if Self::wheel_debug_enabled() {
            Self::wheel_debug_log(&format!(
                "[wheel] event: view_mode={:?} page={}/{} scroll_y={:.2} delta_y={:.2}",
                self.view_mode,
                self.current_page() + 1,
                self.page_count(),
                self.scroll_y,
                delta_y
            ));
        }
        if self.handle_single_page_wheel_delta(delta_y, cx) {
            cx.stop_propagation();
        }
    }

    fn handle_single_page_wheel_delta(
        &mut self,
        delta_y: f32,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        if !matches!(self.view_mode, ViewMode::SinglePage) || self.page_count() == 0 {
            if Self::wheel_debug_enabled() {
                Self::wheel_debug_log(&format!(
                    "[wheel] ignored: single-page-only handler, mode={:?}, page_count={}",
                    self.view_mode,
                    self.page_count()
                ));
            }
            return false;
        }
        if delta_y.abs() < f32::EPSILON {
            if Self::wheel_debug_enabled() {
                Self::wheel_debug_log("[wheel] ignored: zero-ish delta");
            }
            return false;
        }

        let effective_delta_y = delta_y;
        let max_scroll = self.single_page_max_scroll();
        let scrolling_down = effective_delta_y < 0.0;
        if Self::wheel_debug_enabled() {
            Self::wheel_debug_log(&format!(
                "[wheel] single-page start: page={}/{} scroll_y={:.2} max_scroll={:.2} raw_delta={:.2} effective_delta={:.2} down={} accum={:.2}",
                self.current_page() + 1,
                self.page_count(),
                self.scroll_y,
                max_scroll,
                delta_y,
                effective_delta_y,
                scrolling_down,
                self.single_page_wheel_accum_px
            ));
        }

        if max_scroll <= SINGLE_PAGE_IMMEDIATE_FLIP_SCROLL_EPSILON_PX {
            if scrolling_down {
                self.next_page(cx);
            } else {
                self.prev_page(cx);
            }

            self.single_page_wheel_accum_px = 0.0;
            if Self::wheel_debug_enabled() {
                Self::wheel_debug_log(&format!(
                    "[wheel] immediate flip result: page now {}",
                    self.current_page() + 1
                ));
            }
            return true;
        }

        let old_scroll = self.scroll_y;
        let next_scroll = (self.scroll_y - effective_delta_y).clamp(0.0, max_scroll);
        let applied_scroll_delta = (next_scroll - self.scroll_y).abs();
        self.scroll_y = next_scroll;
        if applied_scroll_delta > 0.5 {
            self.sync_scroll_handle_to_state();
            cx.notify();
        }
        if Self::wheel_debug_enabled() {
            Self::wheel_debug_log(&format!(
                "[wheel] in-page scroll: {:.2} -> {:.2}, applied={:.2}",
                old_scroll, self.scroll_y, applied_scroll_delta
            ));
        }

        // Consume wheel movement for page jumps only when we've exhausted scrolling within
        // the active page and continue pushing against an edge.
        let remaining_delta_px = (effective_delta_y.abs() - applied_scroll_delta).max(0.0);
        if remaining_delta_px <= f32::EPSILON {
            self.single_page_wheel_accum_px = 0.0;
            if Self::wheel_debug_enabled() {
                Self::wheel_debug_log(
                    "[wheel] no remaining delta for edge flip; reset accumulator",
                );
            }
            return true;
        }

        let at_top = self.scroll_y <= 0.5;
        let at_bottom = self.scroll_y >= (max_scroll - 0.5).max(0.0);
        let scrolling_up = effective_delta_y > 0.0;
        let pushing_past_edge = (scrolling_down && at_bottom) || (scrolling_up && at_top);
        if Self::wheel_debug_enabled() {
            Self::wheel_debug_log(&format!(
                "[wheel] edge check: at_top={} at_bottom={} down={} up={} pushing={} remaining={:.2}",
                at_top,
                at_bottom,
                scrolling_down,
                scrolling_up,
                pushing_past_edge,
                remaining_delta_px
            ));
        }

        if !pushing_past_edge {
            self.single_page_wheel_accum_px = 0.0;
            if Self::wheel_debug_enabled() {
                Self::wheel_debug_log("[wheel] not pushing edge; reset accumulator");
            }
            return true;
        }

        self.single_page_wheel_accum_px += remaining_delta_px;
        if Self::wheel_debug_enabled() {
            Self::wheel_debug_log(&format!(
                "[wheel] pushing edge; accumulator now {:.2} (threshold {:.2})",
                self.single_page_wheel_accum_px, SINGLE_PAGE_WHEEL_THRESHOLD_PX
            ));
        }

        while self.single_page_wheel_accum_px >= SINGLE_PAGE_WHEEL_THRESHOLD_PX {
            let before = self.current_page();
            if scrolling_down {
                self.next_page(cx);
            } else {
                self.prev_page(cx);
                if self.current_page() != before {
                    self.scroll_y = self.single_page_max_scroll();
                    self.clamp_scroll();
                    self.sync_scroll_handle_to_state();
                    cx.notify();
                }
            }

            self.single_page_wheel_accum_px -= SINGLE_PAGE_WHEEL_THRESHOLD_PX;
            if Self::wheel_debug_enabled() {
                Self::wheel_debug_log(&format!(
                    "[wheel] threshold flip: page {} -> {}, accumulator {:.2}",
                    before + 1,
                    self.current_page() + 1,
                    self.single_page_wheel_accum_px
                ));
            }
            if self.current_page() == before {
                self.single_page_wheel_accum_px = 0.0;
                if Self::wheel_debug_enabled() {
                    Self::wheel_debug_log(
                        "[wheel] boundary after threshold flip; reset accumulator",
                    );
                }
                break;
            }
        }

        true
    }

    /// Clamp scroll to valid bounds
    fn clamp_scroll(&mut self) {
        if matches!(self.view_mode, ViewMode::SinglePage) {
            let max_scroll = self.single_page_max_scroll();
            self.scroll_y = self.scroll_y.max(0.0).min(max_scroll);
            return;
        }
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
        let frame_start = Instant::now();
        self.evaluate_memory_pressure();

        // Update scale factor for Retina support
        let new_scale = window.scale_factor();
        if (new_scale - self.scale_factor).abs() > 0.01 {
            self.scale_factor = new_scale;
            self.bump_generation();
            // Clear cache when scale changes - pages need re-rendering
            self.cache.clear();
            self.display_pages.clear();
            self.retry_state.clear();
            self.update_visible_pages();
        }

        // Process scroll and dispatch staged render work.
        self.sync_scroll_from_handle(cx);
        self.update_visible_pages();
        self.dispatch_render_jobs(cx);

        let theme = current_theme(window, cx);
        let has_document = self.document.is_some();
        let display_pages = self.display_pages.clone();
        let view_mode = self.view_mode;
        let total_height = if matches!(view_mode, ViewMode::Continuous) {
            self.total_height
        } else {
            display_pages
                .first()
                .map(|page| page.height as f32 + PAGE_GAP * 2.0)
                .unwrap_or((self.viewport_height + PAGE_GAP * 2.0).max(PAGE_GAP * 2.0))
        };
        let scroll_handle = self.scrollbar.handle();
        let scrollbar =
            if matches!(view_mode, ViewMode::Continuous) { self.scrollbar.metrics() } else { None };

        let viewport = div()
            .id("pdf-viewport-shell")
            .flex()
            .flex_row()
            .flex_1()
            .size_full()
            .bg(theme.elevated_surface)
            .on_scroll_wheel(cx.listener(Self::handle_scroll_wheel))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                this.update_scrollbar_drag(event.position.y.0, cx);
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, _event: &gpui::MouseUpEvent, _window, _cx| {
                    this.end_scrollbar_drag();
                }),
            )
            .child(
                div()
                    .id("pdf-viewport")
                    .key_context("PdfViewport")
                    .track_focus(&self.focus_handle)
                    .flex()
                    .flex_1()
                    .size_full()
                    .on_scroll_wheel(cx.listener(Self::handle_scroll_wheel))
                    .when(matches!(view_mode, ViewMode::Continuous), |d| {
                        d.overflow_y_scroll().track_scroll(&scroll_handle)
                    })
                    .when(matches!(view_mode, ViewMode::SinglePage), |d| {
                        d.overflow_y_scroll().track_scroll(&scroll_handle)
                    })
                    .child(if has_document {
                        // Container for all pages
                        div()
                            .relative()
                            .w_full()
                            .h(px(total_height))
                            .children(display_pages.into_iter().map(|page| {
                                let page_width = px(page.width as f32);
                                let page_height = px(page.height as f32);
                                div()
                                    .absolute()
                                    .top(px(page.y_offset))
                                    .w_full()
                                    .flex()
                                    .justify_center()
                                    .on_scroll_wheel(cx.listener(Self::handle_scroll_wheel))
                                    .child(match page.image {
                                        Some(image) => div()
                                            .shadow_sm()
                                            .child(
                                                img(ImageSource::Render(image))
                                                    .w(page_width)
                                                    .h(page_height),
                                            )
                                            .into_any_element(),
                                        None => div()
                                            .w(page_width)
                                            .h(page_height)
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(theme.border)
                                            .bg(match page.quality_state {
                                                PageQualityState::Skeleton => theme.surface,
                                                PageQualityState::UltraLqReady => {
                                                    theme.elevated_surface
                                                }
                                                PageQualityState::LqReady => theme.elevated_surface,
                                                PageQualityState::HqReady => theme.elevated_surface,
                                                PageQualityState::Upgrading => theme.element_hover,
                                            })
                                            .into_any_element(),
                                    })
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
                    }),
            )
            .when_some(scrollbar, |d, metrics| {
                d.child(scrollbar_gutter(
                    "pdf-viewport-scrollbar-gutter",
                    &theme,
                    metrics,
                    cx.listener(|this, event: &gpui::MouseDownEvent, _window, cx| {
                        this.start_scrollbar_drag(event.position.y.0, cx);
                    }),
                    cx.listener(|this, _event: &gpui::MouseUpEvent, _window, _cx| {
                        this.end_scrollbar_drag();
                    }),
                    cx.listener(|this, event: &gpui::MouseMoveEvent, _window, cx| {
                        this.update_scrollbar_drag(event.position.y.0, cx);
                    }),
                ))
            });

        let frame_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
        self.perf_record_frame_cpu_ms(frame_ms);
        viewport
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashSet,
        path::PathBuf,
        sync::Arc,
        time::{Duration, Instant},
    };

    use crate::cache::{
        create_render_image, ByteLruCache, CachedImage, MemoryPressureState, RenderCacheKey,
    };
    use butterpaper_render::{PdfDocument, RenderQuality};
    use gpui::{px, size, AppContext as _, Entity, TestAppContext};

    use super::{
        continuous_scroll_for_single_page, fit_page_percent, fit_width_percent,
        resolve_page_nav_target, single_page_scroll_for_continuous, PageLayout, PageNavTarget,
        PageQualityState, PdfViewport, ViewMode, IDLE_SETTLE_DEBOUNCE, IDLE_TRIM_COOLDOWN,
        PAGE_GAP, SCROLL_IDLE_DEBOUNCE, SINGLE_PAGE_IMMEDIATE_FLIP_SCROLL_EPSILON_PX,
    };

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

    #[test]
    fn mode_switch_scroll_translation_preserves_page_top_padding() {
        let layout_y = 1260.0;
        let single_scroll = 0.0;
        let continuous_scroll = continuous_scroll_for_single_page(layout_y, single_scroll);
        assert_eq!(continuous_scroll, 1240.0);

        let restored_single = single_page_scroll_for_continuous(layout_y, continuous_scroll);
        assert_eq!(restored_single, single_scroll);
    }

    #[test]
    fn mode_switch_scroll_translation_preserves_in_page_offset() {
        let layout_y = 700.0;
        let single_scroll = 142.5;
        let continuous_scroll = continuous_scroll_for_single_page(layout_y, single_scroll);
        let restored_single = single_page_scroll_for_continuous(layout_y, continuous_scroll);
        assert!((restored_single - single_scroll).abs() < 0.001);
    }

    fn fixture_pdf_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures").join(name)
    }

    fn dummy_image(width: u32, height: u32) -> Arc<gpui::RenderImage> {
        let rgba = vec![255u8; (width * height * 4) as usize];
        create_render_image(rgba, width, height).expect("dummy image should be creatable")
    }

    fn run_render_cycles(
        cx: &mut TestAppContext,
        viewport: &Entity<PdfViewport>,
        max_cycles: usize,
    ) {
        for _ in 0..max_cycles {
            cx.run_until_parked();
            let pending = cx.read_entity(viewport, |viewport, _| {
                !viewport.inflight_jobs.is_empty()
                    || !viewport.lq_queue.is_empty()
                    || !viewport.hq_queue.is_empty()
            });
            if !pending {
                return;
            }
        }

        panic!("render pipeline did not drain within {max_cycles} cycles");
    }

    #[gpui::test]
    fn single_page_fit_page_wheel_advances_and_reverses_page(cx: &mut TestAppContext) {
        let (viewport, cx) = cx.add_window_view(|_, cx| PdfViewport::new(cx));
        cx.simulate_resize(size(px(1200.0), px(900.0)));

        let doc = Arc::new(
            PdfDocument::open(&fixture_pdf_path("medium.pdf")).expect("fixture PDF should open"),
        );

        cx.update(|_, app| {
            viewport.update(app, |viewport, cx| {
                viewport.set_document(doc, cx);
                viewport.fit_page(cx);
                viewport.set_view_mode(ViewMode::SinglePage, cx);
            });
        });
        cx.run_until_parked();

        let (page_count, before_page, max_scroll) = cx.read_entity(&viewport, |viewport, _| {
            (viewport.page_count(), viewport.current_page(), viewport.single_page_max_scroll())
        });
        assert!(page_count > 1, "fixture must include multiple pages");
        assert!(
            max_scroll <= SINGLE_PAGE_IMMEDIATE_FLIP_SCROLL_EPSILON_PX,
            "fit-page in single-page mode should stay within immediate-flip threshold; got {max_scroll}"
        );

        cx.update(|_, app| {
            viewport.update(app, |viewport, cx| {
                let handled = viewport.handle_single_page_wheel_delta(-160.0, cx);
                assert!(handled);
            });
        });

        let after_down = cx.read_entity(&viewport, |viewport, _| viewport.current_page());
        assert_eq!(after_down, (before_page + 1).min(page_count - 1));

        cx.update(|_, app| {
            viewport.update(app, |viewport, cx| {
                let handled = viewport.handle_single_page_wheel_delta(160.0, cx);
                assert!(handled);
            });
        });

        let after_up = cx.read_entity(&viewport, |viewport, _| viewport.current_page());
        assert_eq!(after_up, before_page);
    }

    #[gpui::test]
    fn perf_snapshot_records_lq_then_hq_milestones(cx: &mut TestAppContext) {
        let (viewport, cx) = cx.add_window_view(|_, cx| PdfViewport::new(cx));
        cx.simulate_resize(size(px(1280.0), px(900.0)));

        let doc = Arc::new(
            PdfDocument::open(&fixture_pdf_path("medium.pdf")).expect("fixture PDF should open"),
        );

        cx.update(|_, app| {
            viewport.update(app, |viewport, cx| {
                viewport.set_document(doc, cx);
                viewport.set_view_mode(ViewMode::Continuous, cx);
                viewport.fit_width(cx);
            });
        });
        run_render_cycles(cx, &viewport, 12);

        for _ in 0..4 {
            cx.update(|_, app| {
                viewport.update(app, |viewport, cx| {
                    viewport.last_scroll_activity =
                        Instant::now() - SCROLL_IDLE_DEBOUNCE - Duration::from_millis(10);
                    viewport.update_visible_pages();
                    viewport.dispatch_render_jobs(cx);
                });
            });
            run_render_cycles(cx, &viewport, 12);
        }

        let snapshot = cx.read_entity(&viewport, |viewport, _| viewport.perf_snapshot());
        assert!(
            snapshot.first_lq_ms.is_some(),
            "expected first low-quality paint milestone to be recorded"
        );
        assert!(
            snapshot.first_hq_ms.is_some(),
            "expected first high-quality paint milestone to be recorded"
        );
        assert!(
            snapshot.first_hq_ms.unwrap_or_default() >= snapshot.first_lq_ms.unwrap_or_default(),
            "HQ paint should not be recorded before LQ paint"
        );
        assert!(snapshot.lq_jobs_scheduled > 0);
        assert!(snapshot.hq_jobs_scheduled > 0);
    }

    #[gpui::test]
    fn scheduler_defers_hq_during_active_scroll(cx: &mut TestAppContext) {
        let (viewport, cx) = cx.add_window_view(|_, cx| PdfViewport::new(cx));
        cx.simulate_resize(size(px(1200.0), px(900.0)));

        let doc = Arc::new(
            PdfDocument::open(&fixture_pdf_path("medium.pdf")).expect("fixture PDF should open"),
        );

        cx.update(|_, app| {
            viewport.update(app, |viewport, cx| {
                viewport.set_document(doc, cx);
                viewport.set_view_mode(ViewMode::Continuous, cx);
                viewport.fit_width(cx);
                viewport.perf_reset();
            });
        });
        run_render_cycles(cx, &viewport, 12);

        let page_count = cx.read_entity(&viewport, |viewport, _| viewport.page_count());
        assert!(page_count > 1);

        for step in 0..8u16 {
            let page = step % page_count;
            cx.update(|_, app| {
                viewport.update(app, |viewport, cx| {
                    viewport.go_to_page(page, cx);
                    viewport.last_scroll_activity = Instant::now();
                    viewport.update_visible_pages();
                    viewport.dispatch_render_jobs(cx);
                });
            });
        }

        let during_churn = cx.read_entity(&viewport, |viewport, _| viewport.perf_snapshot());
        assert!(during_churn.lq_jobs_scheduled > 0);
        assert_eq!(
            during_churn.hq_jobs_scheduled, 0,
            "HQ jobs should stay deferred while scroll input remains active"
        );

        cx.update(|_, app| {
            viewport.update(app, |viewport, cx| {
                viewport.last_scroll_activity =
                    Instant::now() - SCROLL_IDLE_DEBOUNCE - Duration::from_millis(10);
                viewport.update_visible_pages();
                viewport.dispatch_render_jobs(cx);
            });
        });
        run_render_cycles(cx, &viewport, 12);

        let after_idle = cx.read_entity(&viewport, |viewport, _| viewport.perf_snapshot());
        assert!(after_idle.hq_jobs_scheduled > 0, "HQ work should resume once scrolling is idle");
    }

    #[gpui::test]
    fn critical_pressure_suppresses_hq_dispatch(cx: &mut TestAppContext) {
        let (viewport, cx) = cx.add_window_view(|_, cx| PdfViewport::new(cx));
        cx.simulate_resize(size(px(1200.0), px(900.0)));

        let doc = Arc::new(
            PdfDocument::open(&fixture_pdf_path("medium.pdf")).expect("fixture PDF should open"),
        );

        cx.update(|_, app| {
            viewport.update(app, |viewport, cx| {
                viewport.set_document(doc, cx);
                viewport.set_view_mode(ViewMode::Continuous, cx);
                viewport.fit_width(cx);
                viewport.memory_budget.total_budget_bytes = 1;
                viewport.memory_pressure_state = MemoryPressureState::Critical;
                viewport.perf_reset();
                viewport.last_scroll_activity =
                    Instant::now() - SCROLL_IDLE_DEBOUNCE - Duration::from_millis(10);
                viewport.update_visible_pages();
                viewport.dispatch_render_jobs(cx);
            });
        });
        run_render_cycles(cx, &viewport, 8);

        let (snapshot, display_pages) = cx.read_entity(&viewport, |viewport, _| {
            (viewport.perf_snapshot(), viewport.display_pages.clone())
        });
        assert!(
            display_pages
                .iter()
                .all(|page| !matches!(page.quality_state, PageQualityState::HqReady)),
            "critical mode should not promote visible pages to HQ"
        );
        assert!(
            display_pages.iter().any(|page| page.image.is_some()),
            "critical mode should still keep visible pages renderable"
        );
        assert_eq!(snapshot.hq_jobs_scheduled, 0);
        assert!(matches!(snapshot.memory_pressure_state, MemoryPressureState::Critical));
    }

    #[gpui::test]
    fn pressure_state_hysteresis_prevents_flapping(cx: &mut TestAppContext) {
        let (viewport, cx) = cx.add_window_view(|_, cx| PdfViewport::new(cx));

        let (warm_stays_warm, warm_to_normal, hot_stays_hot, hot_to_warm, critical_stays_critical) =
            cx.update(|_, app| {
                viewport.update(app, |viewport, _| {
                    viewport.memory_pressure_state = MemoryPressureState::Warm;
                    let warm_stays_warm = viewport.derive_memory_pressure_state(0.69, false);
                    let warm_to_normal = viewport.derive_memory_pressure_state(0.66, false);

                    viewport.memory_pressure_state = MemoryPressureState::Hot;
                    let hot_stays_hot = viewport.derive_memory_pressure_state(0.90, false);
                    let hot_to_warm = viewport.derive_memory_pressure_state(0.78, false);

                    viewport.memory_pressure_state = MemoryPressureState::Critical;
                    let critical_stays_critical =
                        viewport.derive_memory_pressure_state(0.90, false);

                    (
                        warm_stays_warm,
                        warm_to_normal,
                        hot_stays_hot,
                        hot_to_warm,
                        critical_stays_critical,
                    )
                })
            });

        assert!(matches!(warm_stays_warm, MemoryPressureState::Warm));
        assert!(matches!(warm_to_normal, MemoryPressureState::Normal));
        assert!(matches!(hot_stays_hot, MemoryPressureState::Hot));
        assert!(matches!(hot_to_warm, MemoryPressureState::Warm));
        assert!(matches!(critical_stays_critical, MemoryPressureState::Critical));
    }

    #[gpui::test]
    fn render_buffer_shrinks_as_pressure_rises(cx: &mut TestAppContext) {
        let (viewport, cx) = cx.add_window_view(|_, cx| PdfViewport::new(cx));

        let (normal, warm, hot, critical) = cx.update(|_, app| {
            viewport.update(app, |viewport, _| {
                viewport.memory_pressure_state = MemoryPressureState::Normal;
                let normal = viewport.current_render_buffer();
                viewport.memory_pressure_state = MemoryPressureState::Warm;
                let warm = viewport.current_render_buffer();
                viewport.memory_pressure_state = MemoryPressureState::Hot;
                let hot = viewport.current_render_buffer();
                viewport.memory_pressure_state = MemoryPressureState::Critical;
                let critical = viewport.current_render_buffer();

                (normal, warm, hot, critical)
            })
        });

        assert!(normal > warm);
        assert!(warm > hot);
        assert!(hot > critical);
    }

    #[gpui::test]
    fn memory_targets_follow_active_and_idle_formulas(cx: &mut TestAppContext) {
        let (viewport, cx) = cx.add_window_view(|_, cx| PdfViewport::new(cx));

        let (active_zero_floor, idle_zero_floor, active_raised, idle_raised) =
            cx.update(|_, app| {
                viewport.update(app, |viewport, _| {
                    viewport.memory_budget.total_budget_bytes = 1_000_000;
                    viewport.page_layouts.clear();
                    viewport.update_memory_targets();
                    let active_zero_floor = viewport.memory_targets.active_target_bytes;
                    let idle_zero_floor = viewport.memory_targets.idle_target_bytes;

                    viewport.memory_budget.total_budget_bytes = 6_000_000;
                    viewport.view_mode = ViewMode::Continuous;
                    viewport.scroll_y = 0.0;
                    viewport.viewport_height = 1_200.0;
                    viewport.scale_factor = 1.0;
                    viewport.page_layouts = vec![PageLayout {
                        page_index: 0,
                        width: 2000.0,
                        height: 2000.0,
                        y_offset: PAGE_GAP,
                    }];
                    viewport.update_memory_targets();
                    let active_raised = viewport.memory_targets.active_target_bytes;
                    let idle_raised = viewport.memory_targets.idle_target_bytes;

                    (active_zero_floor, idle_zero_floor, active_raised, idle_raised)
                })
            });

        assert_eq!(active_zero_floor, 1_000_000);
        assert_eq!(idle_zero_floor, 650_000);
        assert_eq!(active_raised, 6_000_000);
        assert!(idle_raised > ((active_raised as f64) * 0.65) as u64);
    }

    #[gpui::test]
    fn owned_ratio_can_raise_pressure_even_with_low_rss_ratio(cx: &mut TestAppContext) {
        let (viewport, cx) = cx.add_window_view(|_, cx| PdfViewport::new(cx));

        let next = cx.update(|_, app| {
            viewport.update(app, |viewport, _| {
                viewport.memory_pressure_state = MemoryPressureState::Normal;
                let owned_ratio = 0.80_f64;
                let rss_ratio = 0.10_f64;
                let effective_ratio = owned_ratio.max(rss_ratio);
                viewport.derive_memory_pressure_state(effective_ratio, false)
            })
        });

        assert!(matches!(next, MemoryPressureState::Warm | MemoryPressureState::Hot));
    }

    #[gpui::test]
    fn idle_trim_evicts_offscreen_hq_before_offscreen_lq(cx: &mut TestAppContext) {
        let (viewport, cx) = cx.add_window_view(|_, cx| PdfViewport::new(cx));

        cx.update(|_, app| {
            viewport.update(app, |viewport, _| {
                viewport.doc_fingerprint = 1;
                viewport.view_mode = ViewMode::Continuous;
                viewport.scroll_y = 0.0;
                viewport.viewport_height = 700.0;
                viewport.scale_factor = 1.0;
                viewport.page_layouts = vec![
                    PageLayout { page_index: 0, width: 400.0, height: 600.0, y_offset: PAGE_GAP },
                    PageLayout { page_index: 1, width: 400.0, height: 600.0, y_offset: 2_000.0 },
                ];
                viewport.cache = ByteLruCache::new(512 * 1024);

                let cached = CachedImage::from_image(dummy_image(64, 64), 64, 64, 64, 64);
                let visible_lq = RenderCacheKey::new(1, 0, 100, 0, RenderQuality::LqScroll, 100);
                let offscreen_hq = RenderCacheKey::new(1, 1, 100, 0, RenderQuality::HqFinal, 100);
                let offscreen_lq = RenderCacheKey::new(1, 1, 100, 0, RenderQuality::LqScroll, 100);

                assert!(viewport.cache.insert(visible_lq, cached.clone(), None));
                assert!(viewport.cache.insert(offscreen_hq, cached.clone(), None));
                assert!(viewport.cache.insert(offscreen_lq, cached, None));

                viewport.memory_targets.idle_target_bytes = 70_000;
                viewport.last_scroll_activity =
                    Instant::now() - IDLE_SETTLE_DEBOUNCE - Duration::from_millis(10);
                viewport.last_idle_trim_at = Instant::now() - IDLE_TRIM_COOLDOWN;
                viewport.maybe_trim_idle_cache();

                assert!(viewport.cache.contains(&visible_lq));
                assert!(!viewport.cache.contains(&offscreen_lq));
                assert!(
                    viewport.current_owned_bytes() <= viewport.memory_targets.idle_target_bytes
                );
            });
        });
    }

    #[gpui::test]
    fn hq_target_pages_include_visible_ring(cx: &mut TestAppContext) {
        let (viewport, cx) = cx.add_window_view(|_, cx| PdfViewport::new(cx));

        let targets = cx.update(|_, app| {
            viewport.update(app, |viewport, _| {
                viewport.page_layouts = (0..20)
                    .map(|index| PageLayout {
                        page_index: index,
                        width: 100.0,
                        height: 100.0,
                        y_offset: index as f32 * 120.0,
                    })
                    .collect();
                viewport.hq_target_page_indexes(&HashSet::from([10_u16]))
            })
        });

        for page in [8_u16, 9, 10, 11, 12] {
            assert!(targets.contains(&page));
        }
        assert!(!targets.contains(&7));
        assert!(!targets.contains(&13));
    }
}
