# PDF Editor Refactoring - Complete Index

## 📚 Documentation Map

### For Ralph Orchestrator (Execution)
- **PROMPT.md** - Ralph's complete instruction prompt with context and patterns
- **specs/01-cleanup.md** - Phase 1: Remove build artifacts (1 hour)
- **specs/02-components.md** - Phase 2: Component library (3 hours)
- **specs/03-workspace.md** - Phase 3: Tab system (4 hours)
- **specs/04-integration.md** - Phase 4: Polish & integration (2.5 hours)
- **specs/05-testing.md** - Phase 5: Test & finalize (1.5 hours)

### For You (Monitoring)
- **REFACTOR_SUMMARY.md** - Overview of all specs and timeline
- **REFACTOR_INDEX.md** - This file (quick reference)
- **AGENTS.md** - Development guidelines and Zed patterns

## 🎯 Quick Facts

| Item | Details |
|------|---------|
| **Total Specs** | 5 phases (716 lines) |
| **Estimated Time** | ~12 hours |
| **Status** | ✅ Ralph running now |
| **Session** | `pdf-editor-refactor` (tmux) |
| **Backend** | Claude API via ralph.yml |

## 📋 Phase Breakdown

### Phase 1: Cleanup (1 hour)
**File:** `specs/01-cleanup.md`
- Remove tauri/egui build artifacts
- Verify clean `cargo build --release`
- Confirm no legacy dependencies

### Phase 2: Components (3 hours)
**File:** `specs/02-components.md`
- Create component library (button, dropdown, toggle, etc.)
- Build unified sizing system (`ui/sizes.rs`)
- Refactor settings UI to use components
- Update main window layout

### Phase 3: Workspace (4 hours)
**File:** `specs/03-workspace.md`
- Create workspace data model (Tab, TabBar, EditorWindow, Workspace)
- Implement tab bar UI component with interactions
- Implement window management (open, close, merge)
- Add preferences persistence
- Update main window with tab bar
- Add "Behavior" settings section

### Phase 4: Integration (2.5 hours)
**File:** `specs/04-integration.md`
- Implement tab drag-to-new-window with preview
- Window merging on drop
- Keyboard navigation (Cmd+Alt+arrows, Cmd+W)
- Multi-file CLI opening
- State persistence (layout save/restore)
- Element registry updates

### Phase 5: Testing (1.5 hours)
**File:** `specs/05-testing.md`
- Verify clean build (clippy, fmt)
- 8+ functional test cases
- Visual verification via xcap screenshots
- Code cleanup and documentation

## 🚀 How to Monitor

### Watch Ralph Work
```bash
tmux attach -t pdf-editor-refactor
```

### Check Progress
```bash
cat .agent/scratchpad.md        # Task checklist
cat .agent/events.jsonl         # Event log
git log --oneline               # Commits
```

### Review Results
```bash
# After Phase 5 completes:
open /tmp/basic-window.png /tmp/multi-tabs.png /tmp/behavior-settings.png \
     /tmp/drag-before.png /tmp/drag-after.png /tmp/final-state.png
```

## ✨ Key Features

### Tab System
- Multiple PDFs open as tabs
- Click to switch, close button on each
- Dirty indicator (•) for unsaved changes

### Browser-Style Windowing
- Drag tabs to create new windows
- Drag tabs between windows to merge
- Automatic cleanup of empty windows
- Layout persists across restarts

### User Preferences
- "Open PDFs in tabs" toggle
- "Show tab bar" toggle
- "Allow window merging" toggle

### Keyboard Navigation
- `Cmd+Alt+→` - Next tab
- `Cmd+Alt+←` - Previous tab
- `Cmd+W` - Close tab/window

### Code Quality
- Component library (reusable blocks)
- Unified sizing system (no hardcoded pixels)
- 30% less boilerplate code
- Pure GPUI (no tauri/egui)

## 📸 Visual Verification

Ralph will capture these screenshots:

| Screenshot | Shows |
|-----------|-------|
| basic-window.png | Single tab with title, tab bar, viewport, status bar |
| multi-tabs.png | Multiple tabs in tab bar |
| behavior-settings.png | New Behavior settings section |
| drag-before.png | Tab before drag operation |
| drag-after.png | Windows after successful merge |
| final-state.png | App after restart (persistence test) |

## 🔧 Commands Reference

```bash
# Monitor Ralph
tmux attach -t pdf-editor-refactor

# Build & test
cargo build --release
cargo clippy --all-targets
cargo fmt --check

# Run app
./target/release/pdf-editor test.pdf
./target/release/pdf-editor file1.pdf file2.pdf  # Multi-file
./target/release/pdf-editor --settings            # Settings window

# Take screenshots
./target/release/pdf-editor test.pdf --screenshot /tmp/test.png

# Check progress
git log --oneline | head -10
cat .agent/scratchpad.md
```

## ✅ Success Criteria

When Ralph finishes:
- ✅ Zero compilation errors
- ✅ Zero clippy warnings
- ✅ Tab system fully functional
- ✅ All 8+ test cases pass
- ✅ Window layout persists
- ✅ No tauri/egui references
- ✅ 30% cleaner code
- ✅ 6 visual verification screenshots

## 📂 Files Created

```
/Users/alex/code/pdf-editor/
├── PROMPT.md                    (Ralph's instructions)
├── REFACTOR_SUMMARY.md          (Overview)
├── REFACTOR_INDEX.md            (This file)
├── AGENTS.md                    (Zed patterns)
├── ralph.yml                    (Ralph config)
├── specs/
│   ├── 01-cleanup.md           (Phase 1)
│   ├── 02-components.md        (Phase 2)
│   ├── 03-workspace.md         (Phase 3)
│   ├── 04-integration.md       (Phase 4)
│   └── 05-testing.md           (Phase 5)
└── crates/gpui-app/src/
    ├── components/             (To be created)
    ├── workspace/              (To be created)
    └── ui/sizes.rs             (To be created)
```

## 🎬 Getting Started

1. **Ralph is already running:**
   ```bash
   tmux attach -t pdf-editor-refactor
   ```

2. **Let Ralph work** (no input needed)

3. **Monitor progress** periodically:
   ```bash
   cat .agent/scratchpad.md
   ```

4. **After ~12 hours**, check results:
   ```bash
   git log --oneline | head -5
   ./target/release/pdf-editor test.pdf
   ```

5. **Review screenshots:**
   ```bash
   open /tmp/*.png
   ```

## 🤖 Ralph's Behavior

Ralph will:
- Read each spec file sequentially
- Plan tasks using `.agent/scratchpad.md`
- Implement code changes
- Build and test (`cargo build --release`)
- Verify acceptance criteria
- Take xcap screenshots
- Commit with meaningful messages
- Emit events to mark completion
- Move to next spec

Ralph **stops after each event emission** and waits for next iteration with fresh context.

## 📞 Need to Help Ralph?

If Ralph gets stuck, you can:
1. Attach to tmux: `tmux attach -t pdf-editor-refactor`
2. Read latest entry in `.agent/scratchpad.md`
3. Check git diff for current work: `git diff`
4. Provide guidance via manual edits or next prompt

But Ralph is designed to work autonomously—trust the process! 🚀

---

**Setup completed:** Jan 23, 2026 12:23 UTC  
**Ralph status:** 🔄 RUNNING  
**Next milestone:** Phase 1 complete (check in ~1 hour)
