# PDF Editor Refactoring - Ralph Orchestrator Setup

## ✅ Complete Setup Status

### Files Created

**Specification Files (in `./specs/` - Ralph-compatible):**
1. ✅ `specs/01-cleanup.md` (60 lines) - Phase 1: Build artifact cleanup
2. ✅ `specs/02-components.md` (106 lines) - Phase 2: Component library & sizing
3. ✅ `specs/03-workspace.md` (175 lines) - Phase 3: Tab system & data model
4. ✅ `specs/04-integration.md` (166 lines) - Phase 4: Drag-to-merge, persistence
5. ✅ `specs/05-testing.md` (209 lines) - Phase 5: Testing & cleanup

**Supporting Files:**
- ✅ `PROMPT.md` (265 lines) - Ralph's instruction prompt with full context
- ✅ `AGENTS.md` (existing) - Development guidelines with Zed patterns

### Tmux Session

**Session:** `pdf-editor-refactor` (250x50)
**Status:** ✅ Running `ralph run -v`
**Command to attach:** `tmux attach -t pdf-editor-refactor`

---

## 📋 Specifications Overview

### Phase 1: Cleanup & Foundation (1 hour)
**File:** `specs/01-cleanup.md`

Clean build artifacts and verify GPUI-only build:
- Remove tauri build artifacts
- Verify no egui/tauri in source
- Ensure clean `cargo build --release`
- **Result:** Production-ready foundation

### Phase 2: UI Component Standardization (3 hours)
**File:** `specs/02-components.md`

Build reusable component library following Zed patterns:
- Create component modules (button, dropdown, toggle, etc.)
- Unified sizing system (`ui/sizes.rs` with constants)
- Refactor settings UI to use components
- Update main window to use sizing system
- **Result:** 30% less boilerplate, reusable components

### Phase 3: Window & Tab System (4 hours)
**File:** `specs/03-workspace.md`

Implement core workspace and tab system:
- Data model: `TabId`, `Tab`, `TabBar`, `EditorWindow`, `Workspace`
- Tab bar UI component with close buttons and overflow handling
- Window management (open, close, merge logic)
- Preferences persistence (`prefer_tabs`, `allow_merge`, `show_tab_bar`)
- Add "Behavior" settings section
- **Result:** Full tab system foundation ready

### Phase 4: Integration & Polish (2.5 hours)
**File:** `specs/04-integration.md`

Connect all pieces and add advanced features:
- Drag-to-new-window with semi-transparent preview
- Window merging when dropping on tab bar
- Keyboard navigation (Cmd+Alt+arrows, Cmd+W)
- Multi-file CLI opening (`pdf-editor file1.pdf file2.pdf`)
- State persistence (save/restore layout on restart)
- Element registry updates for automation
- **Result:** Full-featured tab system with smooth UX

### Phase 5: Testing & Cleanup (1.5 hours)
**File:** `specs/05-testing.md`

Comprehensive testing and finalization:
- Clean build verification (cargo clippy, cargo fmt)
- 8+ functional test cases with visual verification
- xcap screenshot verification for visual regressions
- Code cleanup and documentation
- **Result:** Production-ready release

---

## 🎯 Key Features Being Implemented

### User-Facing Features
✨ **Tab System**
- Multiple PDFs open as tabs in single window
- Click to switch tabs
- Close button on each tab
- Shows dirty indicator (•) for unsaved changes

✨ **Browser-Style Windowing**
- Drag tabs to create new windows
- Drag tabs between windows to merge them
- Automatic window cleanup when empty
- Window positions and layout persist across restarts

✨ **User Preferences**
- "Open PDFs in tabs" toggle (can open in new window instead)
- "Show tab bar" toggle (hide when single tab)
- "Allow window merging" toggle
- All preferences save to disk

✨ **Keyboard Navigation**
- `Cmd+Alt+→` - Next tab
- `Cmd+Alt+←` - Previous tab  
- `Cmd+W` - Close current tab (or window if last tab)

✨ **CLI Enhancements**
- `pdf-editor file1.pdf file2.pdf file3.pdf` opens all as tabs
- Support for opening files with preferences

✨ **State Persistence**
- Window positions saved
- Tab list saved  
- Active window/tab restored
- Preferences persisted

### Developer-Facing Improvements
🔧 **Component Library**
- Reusable button, dropdown, toggle, settings_item components
- Unified sizing system (constants for all pixel values)
- 30% cleaner codebase

🔧 **Workspace System**
- Clean separation: `workspace/` module handles all multi-window logic
- Type-safe IDs (`TabId`, `WindowId`, `TabBarId`)
- Serde support for persistence

🔧 **Automation Support**
- All tab UI elements have trackable IDs
- Can click/interact via CLI in dev mode
- Works with testing infrastructure

---

## 📊 Implementation Timeline

| Phase | Spec File | Duration | Status |
|-------|-----------|----------|--------|
| 1 | 01-cleanup.md | 1 hour | 🔄 Ralph working |
| 2 | 02-components.md | 3 hours | ⏳ Next |
| 3 | 03-workspace.md | 4 hours | ⏳ Next |
| 4 | 04-integration.md | 2.5 hours | ⏳ Next |
| 5 | 05-testing.md | 1.5 hours | ⏳ Next |
| | **TOTAL** | **~12 hours** | |

---

## 🚀 How Ralph Works

Ralph Orchestrator uses **spec-driven workflow**:

1. **Reads specs** from `./specs/` folder
2. **Plans execution** (updates `.agent/scratchpad.md`)
3. **Executes tasks** one by one
4. **Verifies** each task with build/test commands
5. **Screenshots** key visual states via xcap
6. **Commits** work after each spec completes
7. **Reports progress** via events (`.agent/events.jsonl`)

Ralph runs multiple **iterations**, with each iteration:
- Fresh context
- Focused on single spec or task
- Builds → Tests → Commits
- Emits completion events

---

## 📸 Visual Verification Strategy

Ralph will capture xcap screenshots at key points:

| Screenshot | Purpose | Location |
|-----------|---------|----------|
| basic-window.png | Single tab layout verification | `/tmp/` |
| multi-tabs.png | Multiple tabs rendering | `/tmp/` |
| behavior-settings.png | Behavior section in settings | `/tmp/` |
| drag-before.png | Before drag operation | `/tmp/` |
| drag-after.png | After window merge | `/tmp/` |
| final-state.png | After app restart (persistence) | `/tmp/` |

Can review all at once:
```bash
open /tmp/basic-window.png /tmp/multi-tabs.png /tmp/behavior-settings.png \
     /tmp/drag-before.png /tmp/drag-after.png /tmp/final-state.png
```

---

## 💾 Build & Test Commands

Ralph will execute these commands to verify each phase:

```bash
# Clean build
cargo clean
cargo build --release 2>&1 | tee build.log

# Code quality
cargo clippy --all-targets
cargo fmt --check

# Verify no tauri/egui
grep -r "tauri\|egui" Cargo.lock

# Run app
./target/release/pdf-editor [file.pdf]

# Screenshot
./target/release/pdf-editor --screenshot /tmp/test.png

# Multi-file
./target/release/pdf-editor file1.pdf file2.pdf --screenshot /tmp/tabs.png

# Settings
./target/release/pdf-editor --settings --screenshot /tmp/settings.png

# Dev mode automation
./target/release/pdf-editor --dev --list-elements
```

---

## 🎯 Success Criteria

When Ralph completes all 5 specs, you'll have:

✅ **Zero compilation errors**  
✅ **Zero clippy warnings**  
✅ **Tab system fully functional** - Open/close/switch/drag/merge  
✅ **All test cases pass** (8+ functional tests)  
✅ **Window layout persists** - Restarts remember positions & tabs  
✅ **No tauri/egui references** - Pure GPUI codebase  
✅ **30% cleaner code** - Component library & sizing system  
✅ **Visual verification complete** - 6 key screenshots reviewed  
✅ **Production-ready** - Ready for release  

---

## 📝 Key Implementation Files

**To Create/Modify:**
- `crates/gpui-app/src/components/` - New component library
- `crates/gpui-app/src/workspace/` - New workspace system
- `crates/gpui-app/src/ui/sizes.rs` - New sizing constants
- `crates/gpui-app/src/settings.rs` - Refactor to use components
- `crates/gpui-app/src/main.rs` - Add tab bar to layout
- `.agent/scratchpad.md` - Ralph's progress tracker

**No Changes Needed:**
- `crates/render/` - PDF rendering library
- `AGENTS.md` - Development guidelines
- `Cargo.toml` - No new dependencies

---

## 🔗 How to Monitor Progress

### Watch Ralph Work

```bash
# Attach to tmux session
tmux attach -t pdf-editor-refactor

# View Ralph's event log
cat .agent/events.jsonl | tail -20

# View Ralph's scratchpad
cat .agent/scratchpad.md
```

### Check Git Commits

```bash
# See what Ralph has implemented
git log --oneline | head -20

# View changes from last commit
git diff HEAD~1
```

### Manual Verification

After Ralph finishes each phase:
```bash
# Build to verify
cargo build --release

# Run app to verify
./target/release/pdf-editor test.pdf

# Check code quality
cargo clippy --all-targets
```

---

## ✨ Next Steps

1. **Monitor Ralph** - Watch the tmux session or tail events
2. **Review screenshots** - After Phase 5 completes, review `/tmp/` screenshots
3. **Test manually** - Try out the new tab system
4. **Deploy** - Build release binary and ship it!

---

## 📞 Ralph's Workflow

Ralph will:

**Phase 1 (Cleanup):**
- Remove build artifacts
- Verify clean build
- Check no tauri/egui references

**Phase 2 (Components):**
- Create component module structure
- Create sizing constants system
- Refactor settings UI
- Update main window layout

**Phase 3 (Workspace):**
- Create workspace data model
- Implement tab bar UI
- Implement window management
- Add preferences persistence
- Update main window for tabs
- Add Behavior settings section

**Phase 4 (Integration):**
- Implement tab drag-to-window
- Add keyboard navigation
- Support multi-file CLI opening
- Implement state persistence
- Update element registry

**Phase 5 (Testing):**
- Verify clean build (clippy, fmt)
- Execute 8+ functional tests
- Capture visual screenshots
- Final code cleanup
- Output LOOP_COMPLETE

---

**Happy refactoring! Ralph is on the job.** 🤖✨
