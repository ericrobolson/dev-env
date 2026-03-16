# Audio Notification Refactor — Overview

## Feature Summary

Extracted audio notification logic from inline prompt text into a standalone `bin/play-sound` script with a `play_sound()` helper in `helpers.sh`. Previously, prompts contained vague "if possible, play an audio notification" instructions that the AI agent might or might not execute. Now, the absolute path to `bin/play-sound` is injected into prompts so the agent runs it explicitly. This makes audio notifications reliable and deterministic.

---

## Pipeline Document Summaries

### 01-design.md
Defines the problem (unreliable audio via prompt text), proposes `bin/play-sound` as a standalone script, scopes the work to three existing call sites (`helpers.sh`, `build-feature`, `clean-room`), and documents design decisions: non-fatal execution, background playback, `SILENT` env var support. Includes a testing plan with 9 test cases.

### 02-research.md
Audits all three existing audio instruction locations, maps which scripts use `build_prompt()`, identifies where `play-sound` calls should be inserted (only after non-interactive stages), documents `afplay` behavior on macOS, and catalogs exact strings to remove from each file.

### 03-plan.md
Step-by-step implementation plan: create `bin/play-sound`, remove audio text from `helpers.sh`/`build-feature`/`clean-room`, add `play-sound` calls after `run_agent` in each pipeline script, create test file, update `TODOS.md`. Includes full code for the script and tests.

### 04-checklist.md
Tracks implementation progress. All items checked except manual verification. 10 checklist sections covering script creation, prompt cleanup, call-site wiring, tests (11/11 passing), and TODO update.

### 05-debug.md
Documents 5 post-implementation issues and fixes:
1. Made `SOUND` variable configurable via env var.
2. Moved logic from standalone script to `helpers.sh` function.
3. Fixed background playback killing sound on script exit — switched to foreground.
4. Reversed approach: `play_sound()` now returns the path to `bin/play-sound`, which is injected into prompts for agent execution.
5. Extracted `get_bin_dir()` as a reusable helper.

---

## Code Changes

### Files Changed (15 files, +1067/-10 lines)

| File | Change |
|---|---|
| `bin/play-sound` | **New.** Standalone script: plays `Glass.aiff` via `afplay`, guards for `SILENT`, missing `afplay`, missing sound file. Always exits 0. |
| `bin/helpers.sh` | Removed audio instruction from `build_prompt()`. Added `get_bin_dir()` and `play_sound()` (returns path to `bin/play-sound`). |
| `bin/build-feature` | Removed inline audio text from implementation prompt. Injects `play_sound` path into implementation and overview prompts. |
| `bin/clean-room` | Removed audio text from echo block. Injects `play_sound` path into implementation prompt. |
| `bin/gen-doc` | Injects `play_sound` path into the generated prompt. |
| `tests/test-play-sound.sh` | **New.** 11 tests covering `bin/play-sound` behavior, `SILENT` mode, missing `afplay`, and integration checks (no audio text in prompts, `play_sound` wired in all scripts). |
| `TODOS.md` | Marked audio refactor complete. Added new TODO items. |
| `docs/2603161214-AudioNotificationRefactor/*` | **New.** Pipeline docs (design, research, plan, checklist, debug). |
| `test/2603161242-Sonata/*` | **New.** Unrelated test doc created during pipeline run. |

### Untracked Files

- `test/2603161242-Sonata/DOCUMENT.md`
