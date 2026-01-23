# Ralph Scratchpad - Modular Refactor

## Current Task
Spec 02: Modular Architecture Refactoring

## Progress

### Spec 02 Tasks
- [x] Task 2.1: Create shared Tooltip component
- [ ] Task 2.6: Create Icon component
- [ ] Task 2.2: Extract CLI module
- [ ] Task 2.3: Extract Window module
- [ ] Task 2.4: Extract Editor module
- [ ] Task 2.5: Slim main.rs to ~100 lines
- [ ] Task 2.7: Clean up unused TabBar

### Spec 03 Tasks
- [ ] Task 3.1: Extend UI Sizes Module
- [ ] Task 3.2: Create Interactive Element Base
- [ ] Task 3.3: Standardize Button Variants
- [ ] Task 3.4: Create IconButton Component
- [ ] Task 3.5: Create Card Component
- [ ] Task 3.6: Create Input Component
- [ ] Task 3.7: Document Component API

## Notes

- main.rs currently 1638 lines
- TooltipView duplicated in main.rs and components/tab_bar.rs
- components/tab_bar.rs TabBar struct is not actually used

## Build Commands
```bash
cargo build --manifest-path crates/gpui-app/Cargo.toml
cargo clippy --manifest-path crates/gpui-app/Cargo.toml -- -D warnings
cargo run --manifest-path crates/gpui-app/Cargo.toml --bin pdf-editor
```
