# Step 99: Pipeline Overview Summary

## Overview

Add a final stage (step 99) to `build-feature` that generates a summary of all markdown files in the feature directory and all code changes made during the pipeline run. Numbered 99 to allow inserting future stages between implementation/debug (stages 5-6) and this final summary.

## User Flow

### Happy Path

1. User runs `build-feature` as normal
2. Stages 1-6 execute (design → research → plan → checklist → implementation → debug)
3. Debug session completes
4. **Step 99 automatically runs (non-interactive)**
5. Agent reads all markdown files in `$DOC_DIRECTORY` (01-design.md through 05-debug.md, etc.)
6. Agent runs `git diff` to capture code changes made during the pipeline
7. Agent writes `99-overview.md` containing:
   - High-level feature summary (what was built and why)
   - Per-file summary of each markdown document
   - Summary of code changes (files modified/added/deleted, nature of changes)
8. File opens in IDE
9. Audio notification plays
10. Pipeline exits

### Unhappy Paths

| Scenario | Behavior |
|---|---|
| No markdown files exist in directory | Agent writes overview noting no documents found; pipeline continues (non-fatal) |
| No code changes detected (`git diff` is empty) | Agent notes "no code changes" in overview; pipeline continues |
| Agent fails (non-zero exit) | Pipeline exits with error (consistent with other stages via `|| exit 1`) |
| Some markdown files are empty/malformed | Agent summarizes what it can; skips or notes empty files |
| Git is not initialized / not a repo | Skip code changes section; note git unavailable |
| Feature directory has extra non-pipeline files | Include them in summary — overview should capture everything in the directory |

## Requirements

1. **Always runs last** — numbered 99 to leave room for future stages (7-98)
2. **Non-interactive** — no user confirmation needed; runs automatically after debug
3. **Output file** — `99-overview.md` in `$DOC_DIRECTORY`
4. **Markdown summaries** — read and summarize each `*.md` file in the feature directory
5. **Code change summaries** — capture `git diff` from pipeline start to end (or use current working tree diff)
6. **Prompt logging** — append prompt to `00-prompts.md` like other stages
7. **Resume logging** — append claude resume command like other stages

## Unknowns & Questions

1. **Diff baseline**: Should we capture a git ref at pipeline start and diff against it at step 99? Or just use `git diff HEAD` (unstaged/staged changes)? A snapshot ref would be more accurate if user had pre-existing uncommitted changes. **DECISION: use snapshot ref**
2. **Scope of code changes**: Should this include only tracked files, or also new untracked files (`git status`)? **DECISION: all files, new and updated**
3. **00-prompts.md**: Should the overview include/summarize the prompts file, or exclude it to avoid recursion? **DECISION: exclude it to avoid recursion**
4. **Debug stage interaction**: The debug stage is interactive and open-ended. Should step 99 run immediately after debug exits, or should there be a `wait_for_user` before it? **DECISION: run immediately after debug exits**
5. **File ordering**: Should markdown files be summarized in filename-sorted order (which matches pipeline order)? **DECISION: yes, filename-sorted order**
6. **Diff size**: If the diff is very large, should we truncate or summarize at a higher level? Or pass the full diff to the agent and let it summarize? **DECISION: summarize at a higher level**

## Implementation Notes

- Add stage after stage 6 (debug) in `bin/build-feature`
- Use `--no-interactive` mode (like implementation stage)
- Prompt should instruct agent to read all `*.md` files in `$DOC_DIRECTORY` and run `git diff`
- Reference pattern: follows same structure as stages 1-6 (prompt → run_agent → append_resume)

## Testing Plan

### Unit Tests

| # | Test Case | Expected Result |
|---|---|---|
| 1 | Step 99 runs after all other stages complete | `99-overview.md` exists in `$DOC_DIRECTORY` |
| 2 | `99-overview.md` contains summaries of all markdown files | Each pipeline doc (01 through 05) is referenced and summarized |
| 3 | `99-overview.md` contains code change summary | Git diff output is summarized in the file |
| 4 | Prompt is logged to `00-prompts.md` | Overview stage prompt appears in prompts file |
| 5 | Resume command is logged | Claude resume ID appended after overview stage |
| 6 | No code changes scenario | Overview file notes "no code changes"; pipeline exits 0 |
| 7 | Empty feature directory (no markdown files) | Overview file notes no documents; pipeline exits 0 |
| 8 | Pipeline failure in step 99 | Script exits non-zero; error is visible to user |
| 9 | Non-interactive execution | Stage runs without user prompts (uses `--no-interactive`) |

### Integration Tests

| # | Test Case | Expected Result |
|---|---|---|
| 1 | Full pipeline run end-to-end | All stages 1-6 + 99 produce expected output files |
| 2 | Step 99 output references correct file paths | All file paths in overview match actual `$DOC_DIRECTORY` contents |
| 3 | Adding a hypothetical stage 7 between debug and overview | Step 99 still runs last and includes stage 7 output in summary |
