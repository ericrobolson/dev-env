# Overview: RemoveChecklistCod

## Feature Summary

Removed the unnecessary IDE-open instruction from the checklist stage (Stage 4) in `bin/build-feature`. The checklist is auto-generated and non-interactive, so opening it in the IDE was visual noise. Also added sound notification and IDE launch after the implementation stage completes.

---

## Pipeline Document Summaries

### 01-design.md
Defines the problem: Stage 4 uses `build_prompt` which injects an IDE-open instruction, but the stage is non-interactive and never waits for user review. Proposes inlining the prompt without the IDE-open line. Scoped to Stage 4 only. Includes happy/unhappy paths and test cases.

### 02-research.md
Deep analysis of the `build_prompt` function, its callers, and the checklist stage code path. Maps all `build_prompt` usage across the codebase. Identifies prior art (Stages 5, 6, and `bin/clean-room` all inline prompts). Confirms no downstream impact. Notes the Overview stage (99) has the same issue but is out of scope.

### 03-plan.md
Single-file change plan: replace `build_prompt` call on line 141 with an inlined prompt that includes `$TERSENESS` and file-output instruction but omits IDE-open. Links to design and research docs. Lists verification steps.

### 04-checklist.md
Task checklist for the implementation: edit line 141, replace with inlined prompt, verify surrounding lines untouched. Verification: end-to-end run confirming checklist generation, no IDE open, other stages unaffected.

### 05-debug.md
Post-implementation fix: added `$PLAY_SOUND` and `$IDE .` after Stage 5 (Implementation) completes, so the user gets an audio alert and Cursor opens for code review before the debug session.

---

## Code Changes

### Files Changed (7 files, +503 -3)

| File | Changes |
|------|---------|
| `bin/build-feature` | +10 -3 |
| `docs/2603161858-RemoveChecklistCod/00-prompts.md` | +170 (new) |
| `docs/2603161858-RemoveChecklistCod/01-design.md` | +54 (new) |
| `docs/2603161858-RemoveChecklistCod/02-research.md` | +167 (new) |
| `docs/2603161858-RemoveChecklistCod/03-plan.md` | +60 (new) |
| `docs/2603161858-RemoveChecklistCod/04-checklist.md` | +22 (new) |
| `docs/2603161858-RemoveChecklistCod/05-debug.md` | +23 (new) |

### Code Changes in `bin/build-feature`

1. **Stage 4 (Checklist)**: Replaced `build_prompt` call with inlined prompt that omits the IDE-open instruction. Keeps file-output and terseness directives.

2. **Stage 5 (Implementation)**: Added `$PLAY_SOUND` and `$IDE .` after implementation completes to notify the user and open the project in Cursor.

3. **Stage 6 (Debug)**: Removed `$PLAY_SOUND` from the debug prompt (moved to post-implementation).

### No Untracked Files

No untracked files outside the docs directory.
