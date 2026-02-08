//! Local continuous-scroll benchmark runner.

use crate::app::{BenchmarkPerfSnapshot, PdfEditor};
use crate::process_memory;
use gpui::{Context, Window, WindowHandle};
use serde::Serialize;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

const WARMUP_SECONDS: u64 = 5;
const ACTIVE_SCROLL_SECONDS: u64 = 30;
const SCROLL_CYCLE_SECONDS: f64 = 4.0;
const SCROLL_SPEED_PX_PER_SEC: f32 = 2_800.0;
const TARGET_P95_MS: f32 = 8.3;
const TARGET_P99_MS: f32 = 12.0;
const OWNED_STABILITY_RATIO_LIMIT: f64 = 0.90;
const RSS_RATIO_LIMIT: f64 = 0.25;
const RSS_ABSOLUTE_LIMIT_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const RSS_STABILITY_RATIO_LIMIT: f64 = 0.93;
const FINAL_ACTIVE_OWNED_GROWTH_LIMIT: f64 = 0.03;
const MAX_VISIBLE_BLANK_RATIO: f64 = 0.0;
const MAX_THUMBNAIL_VISIBLE_BLANK_RATIO: f64 = 0.05;
const MAX_HQ_VISIBLE_LATENCY_P95_MS: f32 = 1000.0;
const JUMP_SEQUENCE: [f32; 5] = [0.25, 0.75, 0.40, 0.90, 0.10];

#[derive(Clone, Debug)]
pub struct BenchmarkConfig {
    pub file: PathBuf,
    pub seconds: u64,
    pub output: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
pub struct BenchmarkThresholds {
    pub p95_ui_frame_cpu_ms: f32,
    pub p99_ui_frame_cpu_ms: f32,
    pub peak_owned_bytes: u64,
    pub stability_end_owned_ratio: f64,
    pub final_active_owned_growth_ratio: f64,
    pub visible_blank_ratio: f64,
    pub thumbnail_visible_blank_ratio: f64,
    pub hq_visible_latency_p95_ms: f32,
    pub peak_rss_bytes: u64,
    pub stability_end_rss_ratio: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct BenchmarkResult {
    pub pdf_path: String,
    pub duration_seconds: u64,
    pub view_mode: String,
    pub zoom_mode: String,
    pub p95_ui_frame_cpu_ms: Option<f32>,
    pub p99_ui_frame_cpu_ms: Option<f32>,
    pub first_lq_ms: Option<u64>,
    pub first_hq_ms: Option<u64>,
    pub peak_decoded_bytes: u64,
    pub peak_texture_bytes: u64,
    pub peak_owned_bytes: u64,
    pub end_owned_bytes: u64,
    pub peak_thumbnail_decoded_bytes: u64,
    pub end_thumbnail_decoded_bytes: u64,
    pub visible_blank_ratio: f64,
    pub thumbnail_visible_blank_ratio: f64,
    pub hq_visible_latency_p95_ms: Option<f32>,
    pub peak_rss_bytes: u64,
    pub end_rss_bytes: u64,
    pub lq_jobs_scheduled: u64,
    pub hq_jobs_scheduled: u64,
    pub jobs_canceled: u64,
    pub hq_suppression_count: u64,
    pub pass: bool,
    pub fail_reasons: Vec<String>,
    pub warnings: Vec<String>,
    pub thresholds: BenchmarkThresholds,
}

#[derive(Debug)]
struct BenchmarkRuntime {
    started_at: Instant,
    last_tick: Instant,
    configured: bool,
    finished: bool,
    peak_rss_bytes: u64,
    peak_owned_bytes_active: u64,
    active_owned_samples: Vec<(f64, u64)>,
    warmup_baseline: Option<WarmupBaseline>,
    jump_cursor: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct WarmupBaseline {
    viewport_visible_blank_frames: u64,
    viewport_visible_frame_samples: u64,
    thumbnail_visible_blank_frames: u64,
    thumbnail_visible_frame_samples: u64,
}

impl BenchmarkRuntime {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            started_at: now,
            last_tick: now,
            configured: false,
            finished: false,
            peak_rss_bytes: 0,
            peak_owned_bytes_active: 0,
            active_owned_samples: Vec::new(),
            warmup_baseline: None,
            jump_cursor: 0,
        }
    }
}

pub fn start(
    window_handle: WindowHandle<PdfEditor>,
    config: BenchmarkConfig,
    cx: &mut gpui::App,
) -> Result<(), String> {
    let runtime = Rc::new(RefCell::new(BenchmarkRuntime::new()));
    window_handle
        .update(cx, |editor, window, cx| {
            editor.open_file(config.file.clone(), cx);
            schedule_next_frame(editor, window, cx, runtime, config);
        })
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn schedule_next_frame(
    _editor: &mut PdfEditor,
    window: &mut Window,
    cx: &mut Context<PdfEditor>,
    runtime: Rc<RefCell<BenchmarkRuntime>>,
    config: BenchmarkConfig,
) {
    cx.on_next_frame(window, move |editor, window, cx| {
        drive_benchmark(editor, window, cx, runtime.clone(), config.clone());
        if !runtime.borrow().finished {
            schedule_next_frame(editor, window, cx, runtime.clone(), config.clone());
        }
    });
}

fn fallback_snapshot() -> BenchmarkPerfSnapshot {
    BenchmarkPerfSnapshot {
        viewport: crate::viewport::PerfSnapshot {
            first_lq_ms: None,
            first_hq_ms: None,
            p95_ui_frame_cpu_ms: None,
            p99_ui_frame_cpu_ms: None,
            peak_decoded_bytes: 0,
            peak_texture_bytes: 0,
            viewport_cache_cap_bytes: 0,
            lq_jobs_scheduled: 0,
            hq_jobs_scheduled: 0,
            jobs_canceled: 0,
            peak_rss_bytes: 0,
            end_rss_bytes: 0,
            pressure_state_durations_ms: crate::viewport::PressureDurationsMs::default(),
            hq_suppression_count: 0,
            memory_pressure_state: crate::cache::MemoryPressureState::Normal,
            memory_budget_total_bytes: 0,
            current_viewport_decoded_bytes: 0,
            current_viewport_texture_bytes: 0,
            current_inflight_estimated_bytes: 0,
            current_owned_bytes: 0,
            peak_owned_bytes: 0,
            end_owned_bytes: 0,
            active_target_bytes: 0,
            idle_target_bytes: 0,
            visible_blank_frames: 0,
            visible_ultra_lq_frames: 0,
            visible_frame_samples: 0,
            hq_visible_latency_p95_ms: None,
        },
        thumbnail: crate::sidebar::ThumbnailPerfSnapshot::default(),
        total_owned_bytes: 0,
    }
}

fn final_active_growth(samples: &[u64]) -> (bool, f64) {
    let monotonic =
        samples.windows(2).all(|pair| pair.get(1).copied().unwrap_or_default() >= pair[0]);
    let ratio = if let (Some(first), Some(last)) = (samples.first(), samples.last()) {
        if *first == 0 {
            0.0
        } else {
            (*last as f64 / *first as f64) - 1.0
        }
    } else {
        0.0
    };
    (monotonic, ratio)
}

fn evaluate_benchmark(
    p95_ui_frame_cpu_ms: Option<f32>,
    p99_ui_frame_cpu_ms: Option<f32>,
    peak_owned_bytes: u64,
    end_owned_bytes: u64,
    visible_blank_ratio: f64,
    thumbnail_visible_blank_ratio: f64,
    hq_visible_latency_p95_ms: Option<f32>,
    peak_rss_bytes: u64,
    end_rss_bytes: u64,
    final_active_owned_samples: &[u64],
    thresholds: &BenchmarkThresholds,
) -> (Vec<String>, Vec<String>) {
    let mut fail_reasons = Vec::new();
    let mut warnings = Vec::new();

    if p95_ui_frame_cpu_ms.unwrap_or(f32::INFINITY) > thresholds.p95_ui_frame_cpu_ms {
        fail_reasons.push(format!(
            "p95_ui_frame_cpu_ms={} exceeds {}",
            p95_ui_frame_cpu_ms.unwrap_or(f32::NAN),
            thresholds.p95_ui_frame_cpu_ms
        ));
    }
    if p99_ui_frame_cpu_ms.unwrap_or(f32::INFINITY) > thresholds.p99_ui_frame_cpu_ms {
        fail_reasons.push(format!(
            "p99_ui_frame_cpu_ms={} exceeds {}",
            p99_ui_frame_cpu_ms.unwrap_or(f32::NAN),
            thresholds.p99_ui_frame_cpu_ms
        ));
    }
    if peak_owned_bytes > thresholds.peak_owned_bytes {
        fail_reasons.push(format!(
            "peak_owned_bytes={} exceeds {}",
            peak_owned_bytes, thresholds.peak_owned_bytes
        ));
    }
    if peak_owned_bytes > 0
        && (end_owned_bytes as f64 / peak_owned_bytes as f64) > thresholds.stability_end_owned_ratio
    {
        fail_reasons.push(format!(
            "end_owned/peak_owned={} exceeds {}",
            end_owned_bytes as f64 / peak_owned_bytes as f64,
            thresholds.stability_end_owned_ratio
        ));
    }

    let (final_active_monotonic, final_active_growth_ratio) =
        final_active_growth(final_active_owned_samples);
    if final_active_monotonic
        && final_active_growth_ratio > thresholds.final_active_owned_growth_ratio
    {
        fail_reasons.push(format!(
            "final active owned monotonic growth ratio={} exceeds {}",
            final_active_growth_ratio, thresholds.final_active_owned_growth_ratio
        ));
    }
    if visible_blank_ratio > thresholds.visible_blank_ratio {
        fail_reasons.push(format!(
            "visible_blank_ratio={} exceeds {}",
            visible_blank_ratio, thresholds.visible_blank_ratio
        ));
    }
    if thumbnail_visible_blank_ratio > thresholds.thumbnail_visible_blank_ratio {
        fail_reasons.push(format!(
            "thumbnail_visible_blank_ratio={} exceeds {}",
            thumbnail_visible_blank_ratio, thresholds.thumbnail_visible_blank_ratio
        ));
    }
    if hq_visible_latency_p95_ms.unwrap_or(f32::INFINITY) > thresholds.hq_visible_latency_p95_ms {
        fail_reasons.push(format!(
            "hq_visible_latency_p95_ms={} exceeds {}",
            hq_visible_latency_p95_ms.unwrap_or(f32::NAN),
            thresholds.hq_visible_latency_p95_ms
        ));
    }

    if peak_rss_bytes > thresholds.peak_rss_bytes {
        warnings.push(format!(
            "peak_rss_bytes={} exceeds {}",
            peak_rss_bytes, thresholds.peak_rss_bytes
        ));
    }
    if peak_rss_bytes > 0
        && (end_rss_bytes as f64 / peak_rss_bytes as f64) > thresholds.stability_end_rss_ratio
    {
        warnings.push(format!(
            "end_rss/peak_rss={} exceeds {}",
            end_rss_bytes as f64 / peak_rss_bytes as f64,
            thresholds.stability_end_rss_ratio
        ));
    }

    (fail_reasons, warnings)
}

fn drive_benchmark(
    editor: &mut PdfEditor,
    _window: &mut Window,
    cx: &mut Context<PdfEditor>,
    runtime: Rc<RefCell<BenchmarkRuntime>>,
    config: BenchmarkConfig,
) {
    let now = Instant::now();
    let mut state = runtime.borrow_mut();
    if state.finished {
        return;
    }

    if !state.configured {
        editor.configure_benchmark_continuous_fit_width(cx);
        state.configured = true;
    }

    let elapsed = now.saturating_duration_since(state.started_at);
    let elapsed_s = elapsed.as_secs_f64();
    let dt = now.saturating_duration_since(state.last_tick).as_secs_f32().max(1.0 / 240.0);
    state.last_tick = now;

    let warmup_s = WARMUP_SECONDS as f64;
    let active_end_s = (WARMUP_SECONDS + ACTIVE_SCROLL_SECONDS) as f64;
    let total_s = config.seconds as f64;

    if elapsed_s >= warmup_s && state.warmup_baseline.is_none() {
        if let Some(snapshot) = editor.benchmark_perf_snapshot(cx) {
            state.warmup_baseline = Some(WarmupBaseline {
                viewport_visible_blank_frames: snapshot.viewport.visible_blank_frames,
                viewport_visible_frame_samples: snapshot.viewport.visible_frame_samples,
                thumbnail_visible_blank_frames: snapshot.thumbnail.visible_blank_frames,
                thumbnail_visible_frame_samples: snapshot.thumbnail.visible_frame_samples,
            });
        }
    }

    if elapsed_s >= warmup_s && elapsed_s < active_end_s {
        let active_t = elapsed_s - warmup_s;
        let jump_interval = ACTIVE_SCROLL_SECONDS as f64 / JUMP_SEQUENCE.len() as f64;
        while state.jump_cursor < JUMP_SEQUENCE.len()
            && active_t >= jump_interval * (state.jump_cursor as f64 + 1.0)
        {
            let jump_target = JUMP_SEQUENCE[state.jump_cursor];
            editor.benchmark_jump_to_fraction(jump_target, cx);
            state.jump_cursor += 1;
        }

        let phase = active_t % SCROLL_CYCLE_SECONDS;
        let direction = if phase < (SCROLL_CYCLE_SECONDS / 2.0) { 1.0_f32 } else { -1.0_f32 };
        let delta = direction * SCROLL_SPEED_PX_PER_SEC * dt;
        editor.benchmark_scroll_by_pixels(delta, cx);

        if let Some(snapshot) = editor.benchmark_perf_snapshot(cx) {
            state.peak_owned_bytes_active =
                state.peak_owned_bytes_active.max(snapshot.total_owned_bytes);
            state.active_owned_samples.push((active_t, snapshot.total_owned_bytes));
        }
        if let Some(rss) = process_memory::current_rss_bytes() {
            state.peak_rss_bytes = state.peak_rss_bytes.max(rss);
        }
    } else {
        if let Some(rss) = process_memory::current_rss_bytes() {
            state.peak_rss_bytes = state.peak_rss_bytes.max(rss);
        }
    }

    if elapsed_s < total_s {
        return;
    }

    let snapshot = editor.benchmark_perf_snapshot(cx).unwrap_or_else(fallback_snapshot);
    let peak_rss_bytes = state.peak_rss_bytes.max(snapshot.viewport.peak_rss_bytes);
    let end_rss_bytes = snapshot.viewport.end_rss_bytes;
    let peak_owned_bytes = state
        .peak_owned_bytes_active
        .max(state.active_owned_samples.iter().map(|(_, owned)| *owned).max().unwrap_or(0))
        .max(snapshot.total_owned_bytes);
    let end_owned_bytes = snapshot.total_owned_bytes;
    let baseline = state.warmup_baseline.unwrap_or_default();
    let viewport_visible_blank_frames = snapshot
        .viewport
        .visible_blank_frames
        .saturating_sub(baseline.viewport_visible_blank_frames);
    let viewport_visible_frame_samples = snapshot
        .viewport
        .visible_frame_samples
        .saturating_sub(baseline.viewport_visible_frame_samples)
        .max(1);
    let thumbnail_visible_blank_frames = snapshot
        .thumbnail
        .visible_blank_frames
        .saturating_sub(baseline.thumbnail_visible_blank_frames);
    let thumbnail_visible_frame_samples = snapshot
        .thumbnail
        .visible_frame_samples
        .saturating_sub(baseline.thumbnail_visible_frame_samples)
        .max(1);
    let visible_blank_ratio =
        viewport_visible_blank_frames as f64 / viewport_visible_frame_samples as f64;
    let thumbnail_visible_blank_ratio =
        thumbnail_visible_blank_frames as f64 / thumbnail_visible_frame_samples as f64;

    let physical_ram = process_memory::physical_ram_bytes().unwrap_or(16 * 1024 * 1024 * 1024);
    let rss_threshold = ((physical_ram as f64) * RSS_RATIO_LIMIT) as u64;
    let rss_threshold = rss_threshold.min(RSS_ABSOLUTE_LIMIT_BYTES);
    let thresholds = BenchmarkThresholds {
        p95_ui_frame_cpu_ms: TARGET_P95_MS,
        p99_ui_frame_cpu_ms: TARGET_P99_MS,
        peak_owned_bytes: snapshot.viewport.memory_budget_total_bytes.max(1),
        stability_end_owned_ratio: OWNED_STABILITY_RATIO_LIMIT,
        final_active_owned_growth_ratio: FINAL_ACTIVE_OWNED_GROWTH_LIMIT,
        visible_blank_ratio: MAX_VISIBLE_BLANK_RATIO,
        thumbnail_visible_blank_ratio: MAX_THUMBNAIL_VISIBLE_BLANK_RATIO,
        hq_visible_latency_p95_ms: MAX_HQ_VISIBLE_LATENCY_P95_MS,
        peak_rss_bytes: rss_threshold,
        stability_end_rss_ratio: RSS_STABILITY_RATIO_LIMIT,
    };

    let final_window_start = (ACTIVE_SCROLL_SECONDS.saturating_sub(10)) as f64;
    let final_active_owned_samples: Vec<u64> = state
        .active_owned_samples
        .iter()
        .filter(|(t, _)| *t >= final_window_start)
        .map(|(_, owned)| *owned)
        .collect();

    let (fail_reasons, warnings) = evaluate_benchmark(
        snapshot.viewport.p95_ui_frame_cpu_ms,
        snapshot.viewport.p99_ui_frame_cpu_ms,
        peak_owned_bytes,
        end_owned_bytes,
        visible_blank_ratio,
        thumbnail_visible_blank_ratio,
        snapshot.viewport.hq_visible_latency_p95_ms,
        peak_rss_bytes,
        end_rss_bytes,
        &final_active_owned_samples,
        &thresholds,
    );

    let result = BenchmarkResult {
        pdf_path: config.file.to_string_lossy().to_string(),
        duration_seconds: config.seconds,
        view_mode: "Continuous".to_string(),
        zoom_mode: "FitWidth".to_string(),
        p95_ui_frame_cpu_ms: snapshot.viewport.p95_ui_frame_cpu_ms,
        p99_ui_frame_cpu_ms: snapshot.viewport.p99_ui_frame_cpu_ms,
        first_lq_ms: snapshot.viewport.first_lq_ms,
        first_hq_ms: snapshot.viewport.first_hq_ms,
        peak_decoded_bytes: snapshot.viewport.peak_decoded_bytes,
        peak_texture_bytes: snapshot.viewport.peak_texture_bytes,
        peak_owned_bytes,
        end_owned_bytes,
        peak_thumbnail_decoded_bytes: snapshot.thumbnail.peak_thumbnail_decoded_bytes,
        end_thumbnail_decoded_bytes: snapshot.thumbnail.current_thumbnail_decoded_bytes,
        visible_blank_ratio,
        thumbnail_visible_blank_ratio,
        hq_visible_latency_p95_ms: snapshot.viewport.hq_visible_latency_p95_ms,
        peak_rss_bytes,
        end_rss_bytes,
        lq_jobs_scheduled: snapshot.viewport.lq_jobs_scheduled,
        hq_jobs_scheduled: snapshot.viewport.hq_jobs_scheduled,
        jobs_canceled: snapshot.viewport.jobs_canceled,
        hq_suppression_count: snapshot.viewport.hq_suppression_count,
        pass: fail_reasons.is_empty(),
        fail_reasons,
        warnings,
        thresholds,
    };

    if let Some(parent) = config.output.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(&result) {
        Ok(json) => {
            if let Err(err) = std::fs::write(&config.output, json) {
                eprintln!("benchmark: failed to write output {}: {err}", config.output.display());
            }
        }
        Err(err) => {
            eprintln!("benchmark: failed to serialize results: {err}");
        }
    }

    eprintln!(
        "benchmark: {} (output: {})",
        if result.pass { "PASS" } else { "FAIL" },
        config.output.display()
    );
    state.finished = true;
    if result.pass {
        std::process::exit(0);
    } else {
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_thresholds() -> BenchmarkThresholds {
        BenchmarkThresholds {
            p95_ui_frame_cpu_ms: TARGET_P95_MS,
            p99_ui_frame_cpu_ms: TARGET_P99_MS,
            peak_owned_bytes: 500,
            stability_end_owned_ratio: OWNED_STABILITY_RATIO_LIMIT,
            final_active_owned_growth_ratio: FINAL_ACTIVE_OWNED_GROWTH_LIMIT,
            visible_blank_ratio: MAX_VISIBLE_BLANK_RATIO,
            thumbnail_visible_blank_ratio: MAX_THUMBNAIL_VISIBLE_BLANK_RATIO,
            hq_visible_latency_p95_ms: MAX_HQ_VISIBLE_LATENCY_P95_MS,
            peak_rss_bytes: 1000,
            stability_end_rss_ratio: RSS_STABILITY_RATIO_LIMIT,
        }
    }

    #[test]
    fn owned_stability_is_hard_fail() {
        let thresholds = base_thresholds();
        let (fails, warnings) = evaluate_benchmark(
            Some(1.0),
            Some(1.2),
            400,
            390,
            0.0,
            0.0,
            Some(10.0),
            100,
            90,
            &[300, 320],
            &thresholds,
        );
        assert!(warnings.is_empty());
        assert!(!fails.is_empty());
        assert!(fails.iter().any(|reason| reason.contains("end_owned/peak_owned")));
    }

    #[test]
    fn rss_thresholds_are_warnings_only() {
        let thresholds = base_thresholds();
        let (fails, warnings) = evaluate_benchmark(
            Some(1.0),
            Some(1.2),
            300,
            250,
            0.0,
            0.0,
            Some(10.0),
            1500,
            1400,
            &[200, 202, 204],
            &thresholds,
        );
        assert!(fails.is_empty());
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn final_active_monotonic_growth_hard_fails() {
        let thresholds = base_thresholds();
        let (fails, _warnings) = evaluate_benchmark(
            Some(1.0),
            Some(1.0),
            400,
            300,
            0.0,
            0.0,
            Some(10.0),
            100,
            90,
            &[100, 102, 110],
            &thresholds,
        );
        assert!(fails.iter().any(|reason| reason.contains("monotonic growth")));
    }
}
