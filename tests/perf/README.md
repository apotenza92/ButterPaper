# Performance Smoke Tests

Current smoke checks live in `crates/gpui-app/src/viewport.rs` as `#[gpui::test]` cases:
- `perf_snapshot_records_lq_then_hq_milestones`
- `scheduler_defers_hq_during_active_scroll`
- `memory_targets_follow_active_and_idle_formulas`
- `idle_trim_evicts_offscreen_hq_before_offscreen_lq`

Memory-cap churn guardrail lives in `crates/gpui-app/src/cache.rs`:
- `byte_budget_is_enforced_under_insert_churn`

Run them with:

```bash
cargo test -p butterpaper perf_snapshot_records_lq_then_hq_milestones
cargo test -p butterpaper scheduler_defers_hq_during_active_scroll
cargo test -p butterpaper memory_targets_follow_active_and_idle_formulas
cargo test -p butterpaper idle_trim_evicts_offscreen_hq_before_offscreen_lq
cargo test -p butterpaper byte_budget_is_enforced_under_insert_churn

# local hard gate benchmark (continuous mode)
scripts/bench_all_slides.sh
```

These are deterministic guardrails, not full microbenchmarks:
- first-LQ and first-HQ paint milestones are captured
- HQ work is deferred while scroll activity is hot, then resumed on idle
- cache decoded/texture bytes are capped under repeated insert churn
- benchmark gate uses owned-bytes as hard pass/fail criteria
- RSS thresholds are emitted as warnings in benchmark JSON
- benchmark emits JSON and exits non-zero when hard thresholds fail
