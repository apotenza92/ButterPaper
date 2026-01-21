# High-Performance CAD PDF Editor (Desktop)

Desktop-only, cross-platform, high-performance CAD-style PDF editor for construction and engineering workflows. Written in Rust with GPU-accelerated rendering. Free and open source.

## Core Principles

- Near-instant startup and page switching
- No blocking work on the UI thread
- GPU-accelerated, tile-based rendering
- Deterministic, verifiable measurements
- Offline-only operation
- No telemetry, no cloud dependencies

## Target Platforms

- **macOS (Metal)** - primary development target
- Windows (DirectX) - future
- Linux (Vulkan) - future

## Architecture Overview

**Language:** Rust (no browser runtime, no JavaScript)

**Major Subsystems:**
1. UI/viewport compositor (GPU-rendered retained scene graph)
2. Document core and state model
3. PDF render pipeline (tile-based, preview + crisp profiles)
4. Tile cache (RAM → VRAM → disk, LRU eviction)
5. Job scheduler (priority-based, cancellable workers)
6. OCR subsystem (local, progressive, non-blocking)
7. Annotation engine (vector-based GPU primitives)
8. CAD measurement engine (scale-aware, page coordinates)
9. Persistence/export pipeline

## Out of Scope

- Web application
- Collaboration features
- Cloud sync
- AI-based inference
- Accounts/authentication

---

## Tasks

### Phase 1: Foundation
- [x] Initialize Rust workspace with Cargo.toml and workspace members (core, ui, render, cache, scheduler)
- [x] Set up GPU abstraction layer with Metal backend for macOS
- [x] Create basic application window with GPU-rendered UI shell using metal-rs
- [x] Implement retained scene graph for UI rendering
- [x] Build frame loop (game-style, updates every frame)

### Phase 2: PDF Rendering Pipeline
- [x] Integrate PDF parsing library (pdfium or mupdf bindings)
- [x] Implement tile-based page rendering with fixed-size tiles
- [x] Create tile identity system (content hash, page, zoom, coords, profile, rotation)
- [x] Build preview render profile (fast, lower fidelity)
- [x] Build crisp render profile (high fidelity)
- [x] Implement progressive tile loading (preview first, then crisp)

### Phase 3: Caching System
- [x] Build RAM tile cache with LRU eviction
- [x] Build GPU texture cache (VRAM) with separate budget
- [x] Build persistent disk cache (content-addressed)
- [x] Implement non-blocking cache reads
- [x] Add user-configurable cache size and location

### Phase 4: Job Scheduler
- [x] Create job scheduler with priority queue
- [x] Implement cancellation tokens for jobs
- [x] Build render worker pool (separate from UI thread)
- [x] Build IO thread for file operations
- [x] Implement job priority ordering (visible tiles > margin > adjacent > thumbnails > OCR)
- [x] Add aggressive cancellation for off-screen content

### Phase 5: Document Loading
- [x] Implement fast file open (metadata only initially)
- [x] Build first-page immediate preview rendering
- [x] Defer OCR, indexing, thumbnails on file open
- [x] Implement page switch fast path (<100ms cached, <250ms preview)
- [x] Add prefetching for adjacent pages and margin tiles

### Phase 6: Viewport and Navigation
- [x] Build viewport compositor (tiles + annotations + labels + guides)
- [ ] Implement smooth pan and zoom
- [ ] Add discrete zoom levels for tile rendering
- [ ] Build thumbnail strip/page navigator
- [ ] Implement page rotation support

### Phase 7: Annotation Engine
- [ ] Design annotation data model (immutable geometry + editable metadata)
- [ ] Implement stable annotation IDs
- [ ] Build page-local coordinate system for annotations
- [ ] Render annotations as GPU vector primitives
- [ ] Implement vector-based hit testing
- [ ] Add annotation selection and manipulation handles

### Phase 8: CAD Measurement Engine
- [ ] Build measurement geometry storage (page coordinates)
- [ ] Implement scale system (manual ratio, two-point calibration, per-page)
- [ ] Add measurement labels with real-time value derivation
- [ ] Implement snapping guides for precision
- [ ] Store scale in document metadata
- [ ] Build scale detection from OCR (suggestion only)

### Phase 9: OCR Subsystem
- [ ] Integrate local OCR engine (Tesseract or similar)
- [ ] Implement automatic detection of pages without selectable text
- [ ] Build progressive OCR (current page → nearby → remaining when idle)
- [ ] Create invisible text layer aligned to page coordinates
- [ ] Make OCR output searchable and selectable

### Phase 10: Text Editing
- [ ] Implement text editing on PDF content streams
- [ ] Preserve embedded fonts where possible
- [ ] Apply minimal layout adjustments only
- [ ] Ensure edits remain selectable (no rasterization fallback)
- [ ] Make text edits non-blocking (no sync re-render)

### Phase 11: Persistence and Export
- [ ] Implement working state in memory with batched atomic writes
- [ ] Add crash-safe checkpoints
- [ ] Build PDF save with standard annotations and appearance streams
- [ ] Add flattened PDF export option
- [ ] Implement CSV export for markups and measurements

### Phase 12: Polish and Performance
- [ ] Profile and optimize startup time
- [ ] Ensure large PDFs open without UI stalls
- [ ] Verify page flipping feels instantaneous
- [ ] Test reopening cached documents (must feel instant)
- [ ] Test on macOS with Metal rendering
