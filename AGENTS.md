# ButterPaper Agent Rules

## Mission
Rebuild ButterPaper as a full-Rust native desktop PDF app.

## Locked Decisions
- UI/runtime stack: `GPUI`
- Platforms: macOS, Windows, Linux
- No webview, no JS runtime in production
- Keep desktop app and CLI in one Rust workspace with shared core crates
- Testing stack: `cargo test` + `#[gpui::test]` + visual screenshot checks

## UI Precedent
- Use [Zed](https://github.com/zed-industries/zed) as a precedent for UI work because it is open source.
- Treat Zed as a reference for interaction quality and desktop polish, while keeping ButterPaper architecture decisions independent.

## Priorities (in order)
1. Reader UX parity (open, tabs, thumbnails, zoom, page navigation)
2. Rendering performance (smooth scroll/zoom, DPR-aware output)
3. Annotation/editing foundation
4. Cross-platform packaging and CI gates

## Engineering Guardrails
- Keep UI shell, viewer core, PDF engine, and domain models separate.
- Introduce traits/contracts before backend-specific implementation.
- Prefer deterministic behavior and testable state transitions.
- Avoid global mutable state unless there is a strong runtime reason.
- Document architecture changes in `planning/` as they are made.
