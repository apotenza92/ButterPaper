# LQ-First Rendering System Plan (Thumbnails + Main Canvas Scroll)

## Summary
Build a new rendering pipeline that is **LQ-first** for both thumbnail generation and main-canvas scrolling, then upgrades visible content to HQ after scroll idle. This directly targets the current memory and jank issues seen with `samples/All Slides + Cases.pdf` (~112MB) by replacing full-page eager rendering with bounded, adaptive caches and staged quality upgrades.

## Current-State Findings (Grounded in Repo)
1. `crates/gpui-app/src/viewport.rs` renders full page RGBA for visible pages via `render_page_rgba()` and caches up to 50 page textures by `(page, zoom)`.
2. `crates/gpui-app/src/sidebar.rs` eagerly renders thumbnails for **all pages** on document load.
3. `crates/render/src/tile.rs` and `crates/render/src/progressive.rs` exist but are not integrated into `gpui-app` viewport/sidebar flow.
4. `TileRenderer::render_tile()` currently renders full page then crops tile, which defeats tile-level memory efficiency.

## Decisions Locked For This Plan
1. Rollout: **Default on immediately** (no feature-flag-first rollout).
2. Main canvas HQ policy: **Upgrade on scroll idle**.
3. Memory policy: **Adaptive host-RAM tier caps** (no explicit % exposed to user).

## Architecture Changes

## 1) Rendering Profiles
Add explicit quality profiles in render core:
1. `RenderQuality::LqThumb`
2. `RenderQuality::LqScroll`
3. `RenderQuality::HqFinal`

Map to deterministic scale multipliers:
1. `LqThumb`: 0.25x target linear resolution (clamped min width 96 px).
2. `LqScroll`: 0.5x target linear resolution.
3. `HqFinal`: 1.0x logical target * DPR scale factor.

## 2) Main Canvas Pipeline (Viewport)
Replace page-level single-cache strategy with dual-stage page entries:
1. During active scroll/wheel/drag: request `LqScroll` only.
2. On idle debounce (120ms after last scroll event): enqueue `HqFinal` for currently visible pages first, then near-visible pages.
3. Keep displaying LQ until HQ arrives; atomically swap image per page.

Scheduler rules:
1. Visible-first priority.
2. Render budget per frame: `max 2` LQ jobs or `max 1` HQ job.
3. Cancel stale HQ jobs when zoom/page geometry changes.
4. Reuse in-flight LQ as placeholder if HQ is pending.

Execution model (required):
1. All PDF rasterization runs off the UI thread on a bounded worker pool.
2. UI thread responsibilities are scheduling, cache lookup, and atomic image swap only.
3. Every job carries a generation token `{doc, zoom, rotation, view_mode}`; results apply only when token matches current generation.
4. Cancellation checks occur both before rasterization and before UI upload/apply.

Single-page mode contract:
1. On page flip, request `LqScroll` immediately for the destination page.
2. Keep current page visible until destination LQ is ready (no blank flash).
3. Enqueue `HqFinal` only after idle debounce on the active destination page.
4. Cancel stale HQ work if another page flip/zoom/rotation occurs before apply.
5. Preserve page alignment on LQ->HQ swap (no visible jump).

## 3) Thumbnail Pipeline
Change sidebar from eager all-page render to windowed progressive:
1. Initial load: render only visible thumbnail rows + small overscan.
2. Offscreen thumbs render lazily as user scrolls thumbnail list.
3. Use `LqThumb` only for thumbnail strip (no HQ upgrade needed initially).
4. Keep selected-page thumbnail prewarmed.

## 4) Cache Model
Introduce quality-aware cache keys and byte-bounded eviction.

Public cache key shape:
1. `RenderCacheKey { doc_fingerprint, page_index, zoom_bucket, rotation, quality, dpr_bucket }`

Caches:
1. `page_surface_cache` for viewport LQ/HQ images.
2. `thumbnail_cache` for sidebar thumbnails.
3. `inflight_jobs` map keyed by `RenderCacheKey`.

Eviction:
1. Byte-accurate LRU (not entry-count-only).
2. Adaptive host-RAM tiers for combined decoded image cache cap:
   - RAM <= 8GB: 128MB
   - RAM > 8GB and <= 16GB: 256MB
   - RAM > 16GB and <= 32GB: 384MB
   - RAM > 32GB: 512MB
3. Reserve 70% budget for viewport cache, 30% for thumbnails.
4. Under pressure, evict HQ before LQ for currently visible pages only if replacement LQ exists.

## 5) Render Core API Additions
Add/adjust APIs in `crates/render`:
1. `PdfDocument::render_page_rgba_with_quality(page, target_w, target_h, RenderQuality) -> PdfResult<Vec<u8>>`
2. `PdfDocument::render_page_scaled_with_quality(page, max_w, max_h, RenderQuality) -> PdfResult<(Vec<u8>, u32, u32)>`
3. Optional follow-up for tiles: `render_tile_rgba_with_quality(TileRequest)` that does true subregion rendering (avoid full-page render+crop).

## 6) GPUI Integration Changes
1. `crates/gpui-app/src/viewport.rs`
   - Introduce `PageRenderState { lq: Option<Image>, hq: Option<Image>, status }`.
   - Add scroll-idle detector and staged queue processing.
   - Render uses `hq.or(lq)` image selection.
2. `crates/gpui-app/src/sidebar.rs`
   - Add thumbnail virtualization window + lazy render queue.
3. `crates/gpui-app/src/cache.rs`
   - Replace entry-count LRU with byte-bounded LRU and quality-aware keying.

## Public Interfaces / Types (Important Additions)
1. `crates/render/src/lib.rs`
   - `pub enum RenderQuality { LqThumb, LqScroll, HqFinal }`
2. `crates/gpui-app/src/cache.rs`
   - `pub struct RenderCacheKey { ... quality ... dpr_bucket ... }`
   - `pub struct CacheBudget { max_bytes, viewport_bytes, thumbnail_bytes }`
3. `crates/gpui-app/src/viewport.rs`
   - `enum PageQualityState { Empty, LqReady, HqReady, Upgrading }`

## Data Flow (End-to-End)
1. User scrolls.
2. Viewport computes visible page set.
3. For each visible page: try HQ cache, else LQ cache, else enqueue LQ job.
4. Renderer returns LQ images quickly; UI paints immediately.
5. Idle timer fires; enqueue HQ jobs for visible pages.
6. HQ results swap in-place, preserving layout and scroll anchor.
7. Cache manager evicts by bytes using adaptive budget tiers.

## Failure Modes and Handling
1. Render job timeout/error: keep existing image, then retry with exponential backoff (`250ms`, `500ms`, `1000ms`) up to `3` attempts.
2. Zoom changes mid-render: invalidate incompatible in-flight jobs by generation token.
3. Very large page dimensions: clamp requested decode dimensions to a safe max edge (8192 px) and downscale in UI.
4. Memory pressure: drop offscreen HQ entries first, then offscreen LQ.
5. After `3` consecutive HQ failures for the same cache key, suppress HQ for that key until generation changes.

## Additional Efficiency Guardrails
1. Clamp requested render dimensions before decode using both:
   - max edge: `8192 px`
   - max megapixels: `32 MP`
2. Add queue backpressure:
   - max queued LQ jobs: `24`
   - max queued HQ jobs: `12`
   - drop stale offscreen HQ first when queue is full
3. Add cancellation by generation token on:
   - zoom change
   - rotation change
   - document swap
4. Add hysteresis for quality swaps:
   - after HQ is shown, do not downgrade to LQ for micro-scrolls under `24 px` unless page leaves viewport
5. Split memory accounting into separate counters:
   - decoded RGBA/BGRA bytes
   - GPUI texture bytes
   - in-flight job intermediate bytes
6. Direction-aware prefetch:
   - prefetch 2 pages ahead in scroll direction
   - prefetch 1 page behind
7. Keep selected thumbnail always hot in cache even when offscreen in thumbnail list.

## Test Plan (Phase 1 Lean Scope)

## Core Automated Tests
### Unit Tests
1. Cache key uniqueness across quality + DPR buckets.
2. Generation-token cancellation correctness (stale results never apply).
3. Retry backoff state transitions and max-attempt behavior.

### Integration / gpui::test
1. Opening `samples/All Slides + Cases.pdf` does not trigger eager all-page thumbnail render.
2. Continuous scroll shows immediate LQ pages while moving; after idle, visible pages transition to HQ without scroll jump.
3. Zoom change or document swap invalidates stale in-flight renders and repaints only current generation.

## Perf Checks (Informational, Non-Gating)
1. Run scripted open + rapid-scroll scenarios and emit JSON metrics.
2. Record: `first_lq_ms`, `first_hq_ms`, `p95_frame_time_ms`, `peak_decoded_bytes`, `jobs_canceled`.
3. Start on macOS CI as artifact-only until baselines stabilize.

## Deferred for Post-Phase-1
1. Full cross-platform hard perf gates and regression thresholds.
2. Expanded scenario matrix and long-run soak cases.
3. Additional testing frameworks (property/concurrency/microbench suites).

## Rollout Steps
1. Implement render-quality APIs and byte-bounded cache primitives.
2. Integrate staged LQ/HQ pipeline in viewport.
3. Virtualize thumbnail rendering.
4. Add telemetry counters/logging for cache bytes, queue depth, HQ upgrade latency.
5. Run test gates and add visual checks for LQ->HQ transition correctness.

## Assumptions and Defaults
1. Existing GPUI stack remains unchanged.
2. Existing full-page rendering path remains as fallback during migration.
3. Default behavior is immediate use of new pipeline (no feature-flag gate requested).
4. Idle debounce default is 120ms and can be tuned after profiling.

## 2026-02 No-Blank + Jump Preemption Addendum (Implemented)
1. Shared ultra-LQ preview cache now exists as a per-tab shared cache used by both viewport and sidebar.
2. No-blank contract is enforced by fallback order:
   - viewport: `HQ -> LQScroll -> SharedUltraLQ -> Skeleton`
   - sidebar: `LocalThumb -> SharedUltraLQ -> Skeleton`
3. Sidebar queue now supports deep-jump preemption:
   - detects large visible-window center deltas
   - increments generation token
   - hard-clears stale queued work
   - rebuilds queue from `selected ±1`, then strict-visible window, then nearest-distance ring
   - applies a `300ms` immediate-dispatch bypass window post-jump
4. Viewport HQ policy now maintains a stable ring:
   - HQ target set = strict-visible pages + `±2` neighbors
   - active scroll keeps LQ/ultra-LQ first and only schedules visible HQ when LQ debt is low
   - idle promotes visible then neighborhood HQ
   - pressure policy trims HQ to strict-visible in `Hot`, and pauses HQ in `Critical`
5. Benchmark output now includes:
   - `visible_blank_ratio`
   - `thumbnail_visible_blank_ratio`
   - `hq_visible_latency_p95_ms`
   and uses deterministic active-phase jump sequence `25% -> 75% -> 40% -> 90% -> 10%`.
