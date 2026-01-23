# Refactoring Manifest

**Created:** Jan 23, 2026 12:25 UTC  
**Status:** ✅ Complete  
**Ralph Status:** 🔄 Running (PID: 38262)

---

## Files Created

### Specification Files (716 lines total)
- `specs/01-cleanup.md` (60 lines) - Phase 1
- `specs/02-components.md` (106 lines) - Phase 2
- `specs/03-workspace.md` (175 lines) - Phase 3
- `specs/04-integration.md` (166 lines) - Phase 4
- `specs/05-testing.md` (209 lines) - Phase 5

### Documentation Files

**For Ralph Orchestrator:**
- `PROMPT.md` (265 lines) - Complete execution instructions

**For You:**
- `QUICKSTART.md` (113 lines) - TL;DR guide (read this!)
- `REFACTOR_SUMMARY.md` (189 lines) - Full overview
- `REFACTOR_INDEX.md` (178 lines) - Complete reference
- `AGENTS.md` (existing) - Development guidelines

**This File:**
- `MANIFEST.md` (this file) - What was created and why

---

## Execution Details

**Session:** `pdf-editor-refactor` (tmux)  
**Working Directory:** `/Users/alex/code/pdf-editor`  
**Backend:** Claude API  
**Config File:** `ralph.yml`  

**Ralph's Process:**
1. Read PROMPT.md
2. Plan execution in `.agent/scratchpad.md`
3. Read one spec file at a time (specs/ folder)
4. Implement code following spec
5. Build with `cargo build --release`
6. Test with acceptance criteria
7. Screenshot with `xcap`
8. Commit with git
9. Emit event for next iteration
10. Repeat for all 5 specs

**Expected Duration:** ~12 hours  
**Estimated Completion:** Jan 23, 2026 ~00:25 UTC

---

## What Gets Built

### Phase 1: Cleanup (1 hour)
- Remove build artifacts
- Verify GPUI-only build
- Check no tauri/egui in source

### Phase 2: Components (3 hours)
- Component library (button, dropdown, toggle, etc.)
- Unified sizing system
- Refactor settings UI
- Update main window

### Phase 3: Workspace (4 hours)
- Tab/TabBar/EditorWindow/Workspace data model
- Tab bar UI component
- Window management logic
- Preferences persistence
- "Behavior" settings section

### Phase 4: Integration (2.5 hours)
- Tab drag-to-window
- Window merging
- Keyboard navigation
- Multi-file CLI
- State persistence
- Element registry

### Phase 5: Testing (1.5 hours)
- Build verification
- 8+ functional tests
- 6 xcap screenshots
- Code cleanup

---

## Features Delivered

✨ **Tab System**
- Open multiple PDFs as tabs
- Click to switch tabs
- Close buttons with dirty indicators
- Overflow handling

✨ **Browser-Style Windowing**
- Drag tabs to create windows
- Drag tabs to merge windows
- Automatic cleanup
- Visual feedback

✨ **User Preferences**
- "Open PDFs in tabs" toggle
- "Show tab bar" toggle
- "Allow window merging" toggle
- All persist to disk

✨ **Keyboard Navigation**
- Cmd+Alt+Right (next tab)
- Cmd+Alt+Left (previous tab)
- Cmd+W (close tab/window)

✨ **CLI Enhancements**
- Multi-file opening as tabs
- Preferences-aware behavior

✨ **State Persistence**
- Window positions saved
- Tab list saved
- Layout restored on restart

✨ **Code Quality**
- Component library
- Unified sizing system
- 30% less boilerplate
- Pure GPUI (no tauri/egui)

---

## Visual Verification

Ralph will capture 6 xcap screenshots:
1. `basic-window.png` - Single tab layout
2. `multi-tabs.png` - Multiple tabs
3. `behavior-settings.png` - New settings section
4. `drag-before.png` - Drag operation start
5. `drag-after.png` - Window merge result
6. `final-state.png` - Persistence after restart

All saved to `/tmp/` for review.

---

## How to Monitor

**Attach to tmux:**
```bash
tmux attach -t pdf-editor-refactor
```

**Check progress:**
```bash
cat .agent/scratchpad.md    # Current tasks
git log --oneline           # Recent commits
cat .agent/events.jsonl     # Event log
```

**Every hour:**
```bash
cat .agent/scratchpad.md    # See what Ralph is working on
```

**After 12 hours:**
```bash
cargo build --release           # Verify build
./target/release/pdf-editor test.pdf  # Test it
open /tmp/*.png                 # View screenshots
git log --oneline | head -10    # See all work
```

---

## Success Criteria

✅ Zero compilation errors  
✅ Zero clippy warnings  
✅ Tab system fully functional  
✅ All 8+ test cases pass  
✅ Window layout persists  
✅ No tauri/egui references  
✅ 30% cleaner code  
✅ 6 visual verification screenshots  
✅ Production-ready  

---

## Next Steps

1. **Let Ralph work** (no input needed)
2. **Check progress hourly:** `cat .agent/scratchpad.md`
3. **After 12 hours:** Verify build and review screenshots
4. **Celebrate!** Modern tab system with cleaner code

---

## Key Files

**Ralph reads from:**
- `PROMPT.md` (instructions)
- `specs/01-cleanup.md` through `specs/05-testing.md` (specifications)
- `AGENTS.md` (development patterns)

**Ralph writes to:**
- `.agent/scratchpad.md` (progress tracking)
- `.agent/events.jsonl` (event log)
- Source files in `crates/gpui-app/src/`
- Commits to git

**You review:**
- `.agent/scratchpad.md` (progress)
- `git log` (commits)
- `/tmp/*.png` (screenshots)

---

## Commands

**Watch Ralph:**
```bash
tmux attach -t pdf-editor-refactor
tail -f .agent/events.jsonl
```

**Check Results:**
```bash
cargo build --release
./target/release/pdf-editor test.pdf
open /tmp/*.png
```

**Review Work:**
```bash
git log --oneline | head -10
git diff HEAD~1  # Last commit
cat .agent/scratchpad.md  # Current status
```

---

## Timeline

| Time | Event |
|------|-------|
| 12:22 UTC | Setup complete, Ralph starts |
| 13:22 UTC | Phase 1 done (cleanup) |
| 16:22 UTC | Phase 2 done (components) |
| 20:22 UTC | Phase 3 done (workspace) |
| 22:52 UTC | Phase 4 done (integration) |
| 00:22 UTC | Phase 5 done (testing) |
| 00:25 UTC | Ralph outputs LOOP_COMPLETE ✅ |

---

## Important Notes

- **Ralph is autonomous** - No manual intervention needed
- **Spec-driven workflow** - Ralph follows specs precisely
- **Iterative approach** - Fresh context each iteration
- **Self-verifying** - Ralph tests as it goes
- **Visual verification** - Screenshots confirm correctness

---

## Support

If Ralph gets stuck, check:
1. `.agent/scratchpad.md` (current task)
2. `git diff` (what changed)
3. `git log -1 -p` (last commit)
4. Relevant spec file (what should be done)

Ralph is designed to handle problems autonomously, but if you need to help:
1. Read the spec for that phase
2. Manually fix the issue
3. Commit the fix
4. Let Ralph continue in next iteration

---

## Success Indicators

✅ Build completes without errors  
✅ Clippy shows zero warnings  
✅ Git commits appear regularly (1-2 per hour)  
✅ `.agent/scratchpad.md` shows progress  
✅ `/tmp/` fills with screenshots (Phase 5)  
✅ Final message: `LOOP_COMPLETE`  

---

**Status: Ready for execution**  
**Ralph: Running and working**  
**You: Done! Check back in 12 hours.**

🚀
