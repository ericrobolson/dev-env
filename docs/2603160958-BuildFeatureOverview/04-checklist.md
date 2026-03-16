# Implementation Checklist: Pipeline Overview Summary

**Reference:** [Implementation Plan](03-plan.md)

---

## 1. Git Baseline Capture (`bin/build-feature`)

- [x] Add `GIT_BASELINE_REF=""` variable after `PROMPTS_FILE` assignment (after line 42)
- [x] Add `git rev-parse HEAD` conditional to populate `GIT_BASELINE_REF`
- [x] Verify baseline ref is empty string when not in a git repo

## 2. Stage 99 Block (`bin/build-feature`)

- [x] Add `STAGE="overview"` block after Stage 6 (debug) — after line 178
- [x] Set `OVERVIEW_FILE` via `new_doc "$DOC_DIRECTORY" "99-overview.md"`
- [x] Construct `OVERVIEW_PROMPT` with:
  - [x] Instruction to read all markdown files in `$DOC_DIRECTORY` (sorted, excluding `00-prompts.md`)
  - [x] Conditional git diff commands using `$GIT_BASELINE_REF`
  - [x] Fallback to `git diff HEAD` / `git status` when baseline unavailable
  - [x] High-level feature summary instruction
- [x] Call `build_prompt` to construct final prompt
- [x] Call `append_prompt` to log prompt to `$PROMPTS_FILE`
- [x] Call `run_agent "$STAGE" --no-interactive` with `|| exit 1`
- [x] Call `append_resume` after agent completes

## 3. Documentation Updates

- [x] Update `README.md` pipeline stage diagram to include `Stage 99 (overview)`
- [x] Update `README.md` output directory listing to include `99-overview.md`
- [x] Mark `[ ] Add a 'overview' section` as `[x]` in `TODOS.md` (line 17)

## 4. Tests (`tests/test-overview-stage.sh`)

- [x] Create `tests/test-overview-stage.sh` following `tests/test-append-prompt.sh` pattern
- [x] Implement `test_git_baseline_capture`: init temp repo, commit, verify `rev-parse HEAD` returns non-empty ref
- [x] Implement `test_overview_file_created`: verify `new_doc` returns correct path `$doc_dir/99-overview.md`
- [x] Implement `test_prompt_excludes_prompts_file`: verify prompt text contains exclusion clause
- [x] Implement `test_no_git_repo_fallback`: verify `GIT_BASELINE_REF` is empty outside a git repo

## 5. Validation

- [x] Run `tests/test-overview-stage.sh` — all tests pass
- [ ] Run full pipeline end-to-end on a test feature
- [ ] Verify `99-overview.md` is generated in the doc directory
- [ ] Verify `99-overview.md` contains document summaries and code change summary
- [ ] Verify `00-prompts.md` content is not included in the overview
