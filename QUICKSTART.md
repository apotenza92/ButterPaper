# Quick Start - PDF Editor Refactoring

## TL;DR

Ralph Orchestrator is **running now** and will refactor your PDF Editor over ~12 hours.

### Watch Progress
```bash
tmux attach -t pdf-editor-refactor
```

### Check Status (any time)
```bash
cat .agent/scratchpad.md    # Current task checklist
git log --oneline           # Commits made so far
cat .agent/events.jsonl     # Event log
```

### After ~12 Hours
```bash
cargo build --release       # Verify it compiles
./target/release/pdf-editor test.pdf  # Test it works
open /tmp/*.png            # Review screenshots
git log --oneline | head   # See all work done
```

---

## What Ralph is Building

✨ **Tab System** - Open multiple PDFs as tabs  
✨ **Drag-to-Merge** - Browser-style window management  
✨ **Persistence** - Layout saved across restarts  
✨ **Preferences** - User controls for tab/window behavior  
✨ **Components** - Reusable UI library (30% less code)  
✨ **Screenshots** - 6 visual verification images  

---

## The 5 Phases

| Phase | File | Time | What |
|-------|------|------|------|
| 1 | `specs/01-cleanup.md` | 1h | Remove build artifacts |
| 2 | `specs/02-components.md` | 3h | Component library |
| 3 | `specs/03-workspace.md` | 4h | Tab system |
| 4 | `specs/04-integration.md` | 2.5h | Drag & persistence |
| 5 | `specs/05-testing.md` | 1.5h | Test & finalize |

---

## Documentation

**To understand:**
- `REFACTOR_SUMMARY.md` - Full overview
- `REFACTOR_INDEX.md` - Complete reference
- `AGENTS.md` - Zed patterns (development guidelines)

**For Ralph:**
- `PROMPT.md` - Ralph's instructions
- `specs/` - 5 phase specifications

---

## Success Criteria

✅ Zero compilation errors  
✅ Zero clippy warnings  
✅ Tab system works  
✅ All tests pass  
✅ 6 visual verification screenshots  
✅ 30% cleaner code  
✅ Ready to ship  

---

## Just Let It Run

Ralph is autonomous. You don't need to do anything. Ralph will:

1. Read specifications
2. Write code
3. Build & test
4. Take screenshots
5. Commit work
6. Report LOOP_COMPLETE when done

**Estimated time:** ~12 hours  
**Next check:** In ~1 hour (Phase 1 progress)  
**Final check:** In ~12 hours (all phases complete)

---

## If Something Goes Wrong

Check these in order:

1. `cat .agent/scratchpad.md` - What is Ralph working on?
2. `git log --oneline -5` - What did Ralph just do?
3. `git diff HEAD~1` - What changed?
4. `tmux attach -t pdf-editor-refactor` - Any error messages?

Usually Ralph knows what to do. But if you need to intervene, read the spec file for that phase and help guide the next iteration.

---

🚀 **Ralph is working. You're done. Check back in 12 hours!**
