# PDF Editor Modular Refactor - Ralph Orchestrator Prompt

You are Ralph, tasked with refactoring the PDF Editor codebase from a monolithic structure to a clean, modular architecture with standardized UI components.

## Context

**Project:** PDF Editor (GPUI-based native macOS app)
**Goal:** Break up 1600+ line main.rs into clean modules, standardize UI components
**Specs Location:** `./specs/` folder

## Current Specs

Read specs in order:

1. **`01-tabbed-interface.md`** - COMPLETE - Tab system implementation
2. **`02-modular-architecture.md`** - IN_PROGRESS - Break up main.rs into modules
3. **`03-ui-component-standards.md`** - PENDING - Standardize UI components

## Your Mission

Execute Spec 02 (Modular Architecture), then Spec 03 (UI Standards).

### Spec 02: Modular Architecture (Priority: HIGH)

Break the monolithic main.rs into:

```
app/           - PdfEditor, DocumentTab, menus
cli/           - CliArgs, automation
window/        - Window management, screenshots
components/    - Shared tooltip, icons
```

**Tasks in order:**
1. Task 2.1: Create shared Tooltip component
2. Task 2.6: Create Icon component
3. Task 2.2: Extract CLI module
4. Task 2.3: Extract Window module
5. Task 2.4: Extract Editor module
6. Task 2.5: Slim main.rs to ~100 lines
7. Task 2.7: Clean up unused TabBar component

### Spec 03: UI Component Standards (Priority: MEDIUM)

After Spec 02 completes:
- Extend ui.rs with spacing tokens
- Create Button variants
- Create IconButton component
- Create Card component
- Document all components

## Execution Rules

1. **One task at a time** - Complete each task fully before moving on
2. **Build after each task:**
   ```bash
   cargo build --manifest-path crates/gpui-app/Cargo.toml
   cargo clippy --manifest-path crates/gpui-app/Cargo.toml -- -D warnings
   ```
3. **Test the app works:**
   ```bash
   cargo run --manifest-path crates/gpui-app/Cargo.toml --bin pdf-editor
   ```
4. **Update scratchpad** with progress in `.ralph/scratchpad.md`
5. **Commit after each major task:**
   ```bash
   git add -A && git commit -m "refactor: [task description]"
   ```

## Key Files

**Current structure:**
```
crates/gpui-app/src/
├── main.rs           # 1638 lines - NEEDS SPLITTING
├── components/
│   ├── tab_bar.rs    # Has duplicate TooltipView
│   └── ...
├── settings.rs       # 700+ lines
├── sidebar.rs
├── viewport.rs
├── theme.rs
└── workspace/
```

**Target structure:**
```
crates/gpui-app/src/
├── main.rs           # ~100 lines - just entry point
├── app/
│   ├── editor.rs     # PdfEditor struct
│   ├── document.rs   # DocumentTab
│   └── menus.rs      # Menu setup
├── cli/
│   ├── args.rs       # CliArgs parsing
│   └── automation.rs # Mouse sim, element click
├── window/
│   ├── manager.rs    # list_windows, focus_window
│   └── screenshot.rs # capture_window
├── components/
│   ├── tooltip.rs    # Shared TooltipView
│   ├── icon.rs       # Icon enum
│   └── ...
└── ...
```

## Success Criteria

**Spec 02 Complete when:**
- [ ] main.rs under 150 lines
- [ ] No duplicate TooltipView definitions
- [ ] cli/, window/, app/ modules exist
- [ ] All builds pass
- [ ] App works identically to before

**Spec 03 Complete when:**
- [ ] All spacing uses size constants
- [ ] Button/IconButton components standardized
- [ ] COMPONENTS.md documents everything

## Commands

```bash
# Build
cargo build --manifest-path crates/gpui-app/Cargo.toml

# Lint
cargo clippy --manifest-path crates/gpui-app/Cargo.toml -- -D warnings

# Run
cargo run --manifest-path crates/gpui-app/Cargo.toml --bin pdf-editor

# Test with PDF
cargo run --manifest-path crates/gpui-app/Cargo.toml --bin pdf-editor -- test.pdf
```

## Completion

When both Spec 02 and Spec 03 are complete:
1. Verify all acceptance criteria
2. Run final build and clippy
3. Test app functionality
4. Output: **LOOP_COMPLETE**

---

Start with Spec 02, Task 2.1: Create Shared Tooltip Component

Read `specs/02-modular-architecture.md` for full details.
