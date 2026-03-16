# Research: Step 99 — Pipeline Overview Summary

## 1. Current Pipeline Architecture

### Stage Flow (build-feature lines 49-178)

```
Stage 1 (design) → Stage 2 (research) → Stage 3 (plan) → Stage 4 (checklist) → Stage 5 (implementation) → Stage 6 (debug)
```

**Per-stage pattern:**
1. `new_doc()` — generate filepath
2. Build prompt string with role + instructions
3. `build_prompt()` — append standard suffix (terseness, file output, IDE open, audio)
4. `append_prompt()` — log to `00-prompts.md`
5. `run_agent()` — execute via claude or cursor-agent
6. `append_resume()` — log session resume command
7. `wait_for_user()` — gate (skipped for claude agent)

**Exceptions to pattern:**
- Stage 5 (implementation): uses `--no-interactive`, no `build_prompt()`, no `wait_for_user()`, no output file — modifies codebase directly
- Stage 6 (debug): no `wait_for_user()` (last stage), creates file via `echo "" >> "$DEBUG_FILE"`

### Key Observation for Step 99

**Step 99 is strictly non-interactive. It runs autonomously and exits. No user input, no prompts, no gates.**

It most closely matches **Stage 5 (implementation)** pattern:
- Non-interactive (`--no-interactive` flag → `--print` on agent command)
- No `wait_for_user()` — stage runs to completion and exits
- Runs automatically after debug stage completes — no user confirmation to trigger it
- Agent executes, writes output, and terminates — no conversational loop
- But unlike Stage 5, it produces an output file (`99-overview.md`)

---

## 2. Implementation Challenges

### 2.1 Git Diff Baseline (Snapshot Ref)

**Decision:** Use snapshot ref captured at pipeline start.

**Challenge:** `build-feature` currently has no git integration. Must add:
1. Capture `git rev-parse HEAD` (or `git stash create`) at pipeline start (line ~40)
2. Store in variable (e.g., `GIT_BASELINE_REF`)
3. At step 99: `git diff $GIT_BASELINE_REF` for tracked changes
4. `git ls-files --others --exclude-standard` for new untracked files

**Edge cases:**
- Repo has no commits yet → `git rev-parse HEAD` fails. Use `git diff --cached` or `4b825dc642cb6eb9a060e54bf899d15363d7b169` (empty tree SHA).
- User has uncommitted changes before pipeline start → snapshot ref captures them correctly, but diff will include pre-existing changes. Acceptable per design decision.
- Implementation stage commits changes → diff against baseline still works (shows cumulative changes).
- No `.git` directory → skip code changes section per design.

### 2.2 Diff Size Management

**Decision:** Summarize at a higher level for large diffs.

**Implementation:** The prompt instructs the agent to summarize, not dump raw diff. The agent receives the diff output and produces a human-readable summary. No truncation logic needed in the shell script — the agent handles summarization.

**Risk:** Very large diffs may exceed agent context window. Mitigation: use `git diff --stat` for file-level summary first, then `git diff` for content. Or pipe through `head -n 2000` as a safety limit.

### 2.3 Markdown File Discovery

**Requirement:** Read all `*.md` files in `$DOC_DIRECTORY`, filename-sorted, excluding `00-prompts.md`.

**Implementation:**
```bash
ls "$DOC_DIRECTORY"/*.md | sort
```

**Edge cases:**
- Empty directory → `ls *.md` returns error. Use `ls "$DOC_DIRECTORY"/*.md 2>/dev/null`.
- Extra non-pipeline `.md` files → include per design decision.
- Binary/corrupted files → agent handles gracefully.

### 2.4 Non-Interactive Execution

**Step 99 is a non-interactive stage that runs and exits. There is no user interaction.**

**Existing pattern (Stage 5, line 146):**
```bash
echo "$IMPLEMENTATION_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1
```

Step 99 follows this exactly. The `--no-interactive` flag adds `--print` to the agent command. This means:
1. Agent receives prompt via stdin
2. Agent processes the prompt (reads files, runs git diff, writes `99-overview.md`)
3. Agent output goes to stdout
4. Agent process exits
5. Pipeline continues to completion (no further stages)

There is no conversational loop, no `wait_for_user()`, and no interactive session. The agent runs once and terminates.

### 2.5 Prompt Recursion

**Decision:** Exclude `00-prompts.md` from summarization.

**Implementation:** Instruct the agent in the prompt to skip `00-prompts.md`. The agent reads files by instruction, so explicit exclusion in the prompt text is sufficient.

---

## 3. Prior Art in the Repo

### 3.1 Non-Interactive Stage Pattern

**Source:** `bin/build-feature:135-147` (Stage 5 — Implementation)

The only existing non-interactive stage. Step 99 replicates this pattern but adds an output file.

### 3.2 build_prompt() Standard Suffix

**Source:** `bin/helpers.sh:120-136`

Appends terseness, file output path, IDE open, and audio notification instructions. Step 99 should use this for `99-overview.md`.

### 3.3 append_prompt() and append_resume()

**Source:** `bin/helpers.sh:138-201`

Both functions are non-critical (return 0 on failure). Step 99 must call both per pipeline convention.

### 3.4 Debug Stage (Last Current Stage)

**Source:** `bin/build-feature:154-178`

Step 99 inserts after this. The debug stage ends with `append_resume` and an echo. Step 99 code goes after line 178.

### 3.5 clean-room Pipeline

**Source:** `bin/clean-room`

Three-stage pipeline with similar patterns. Uses `append_prompt` and `append_resume`. Non-interactive stages use `--no-interactive`. No `wait_for_user` calls. Relevant as a reference for non-interactive-only pipelines.

### 3.6 Test Suite

**Source:** `tests/test-append-prompt.sh`

Tests `append_prompt` and `append_resume` functions. Step 99 should add tests for:
- `99-overview.md` creation
- Git baseline ref capture
- Prompt logged to `00-prompts.md`
- Resume command logged

---

## 4. File Numbering Gap Analysis

Current files: `00`, `01`, `02`, `03`, `04`, `05`. Step 99 leaves gap `06-98` for future stages.

The numbering scheme uses `ls | sort` for ordering. `99-overview.md` sorts last among two-digit prefixes. If three-digit prefixes are ever added (e.g., `100-*`), `99` would sort before them lexicographically. Not an immediate concern.

---

## 5. Relevant Files

| File | Path | Relevance |
|------|------|-----------|
| Main pipeline | `bin/build-feature` | Insert step 99 after line 178 |
| Shared helpers | `bin/helpers.sh` | `run_agent`, `build_prompt`, `append_prompt`, `append_resume`, `new_doc` |
| Test suite | `tests/test-append-prompt.sh` | Pattern for new tests; extend for step 99 |
| Design doc | `docs/2603160958-BuildFeatureOverview/01-design.md` | Requirements and decisions |
| README | `README.md` | Update pipeline diagram and output directory listing |
| TODOs | `TODOS.md` | Line 17: `[ ] Add a 'overview' section` — mark complete after implementation |
| gen-doc | `bin/gen-doc` | Reference for single-stage prompt patterns |
| clean-room | `bin/clean-room` | Reference for non-interactive multi-stage patterns |

---

## 6. Implementation Sketch

### Variables to Add (around line 40 of build-feature)

```bash
# Capture git baseline for step 99 diff
GIT_BASELINE_REF=""
if git rev-parse HEAD >/dev/null 2>&1; then
    GIT_BASELINE_REF=$(git rev-parse HEAD)
fi
```

### Step 99 Block (after line 178)

```bash
#
# Stage 99: Overview
#
STAGE="overview"
OVERVIEW_FILE=$(new_doc "$DOC_DIRECTORY" "99-overview.md")
OVERVIEW_PROMPT="You are an expert technical writer.
You are tasked to write a pipeline overview for $FEATURE_NAME.

Read all markdown files in '$DOC_DIRECTORY' (excluding '00-prompts.md') in filename-sorted order.
For each file, write a concise summary.

Then summarize all code changes made during this pipeline run.
$(if [[ -n "$GIT_BASELINE_REF" ]]; then
    echo "Run: git diff $GIT_BASELINE_REF"
    echo "Run: git diff $GIT_BASELINE_REF --stat"
    echo "Run: git ls-files --others --exclude-standard"
else
    echo "Git baseline not available. Run: git diff HEAD"
    echo "Run: git status"
fi)

If the diff is large, summarize at a high level (files changed, nature of changes).
Do not include the contents of '00-prompts.md'.

Write a high-level feature summary at the top (what was built and why)."

echo "==> Generating pipeline overview..."
FINAL_PROMPT=$(build_prompt "$OVERVIEW_FILE" "$OVERVIEW_PROMPT")
append_prompt "$PROMPTS_FILE" "Overview" "$FINAL_PROMPT"
echo "$FINAL_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1
append_resume "$PROMPTS_FILE"
echo "==> Pipeline overview generated: $OVERVIEW_FILE"
```

### Key Decisions Reflected

- **Non-interactive: runs and exits** — `--no-interactive` flag ensures agent runs once, writes output, and terminates. No user interaction at any point. No conversational loop. Pipeline exits after this stage completes.
- Uses `build_prompt()`: gets standard suffix (IDE open, audio, terseness)
- Git baseline: captured at start, diffed at end
- Exclusion: prompt explicitly excludes `00-prompts.md`
- Error handling: `|| exit 1` consistent with other stages
- Prompt logging: `append_prompt` + `append_resume` per convention

---

## 7. Testing Considerations

### Unit Tests to Add

1. **Git baseline capture** — verify `GIT_BASELINE_REF` is set when in a repo, empty when not
2. **Overview file creation** — verify `99-overview.md` path from `new_doc()`
3. **Prompt excludes 00-prompts.md** — grep prompt text for exclusion instruction

### Integration Tests

1. **Full pipeline** — run all stages, verify `99-overview.md` exists and references prior docs
2. **No git repo** — run in non-git directory, verify graceful handling
3. **No code changes** — verify overview notes "no code changes"
4. **Large diff** — verify agent produces high-level summary, not raw dump

### Existing Test Infrastructure

- `tests/test-append-prompt.sh` uses `mktemp`, `trap` cleanup, `assert_contains`, `assert_file_exists`
- Same patterns apply for step 99 tests
- Consider a new file: `tests/test-overview-stage.sh`

---

## 8. Files to Modify

| File | Change |
|------|--------|
| `bin/build-feature` | Add git baseline capture (~line 40), add step 99 block (after line 178) |
| `README.md` | Update pipeline diagram, stage table, output directory listing |
| `TODOS.md` | Check off overview item (line 17) |
| `tests/test-append-prompt.sh` | Or create new `tests/test-overview-stage.sh` |

---

## 9. Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Agent context overflow from large diff | Step 99 fails or produces truncated summary | Use `--stat` first; instruct agent to summarize at high level |
| `git rev-parse HEAD` fails in fresh repo | No baseline ref | Fallback to `git diff HEAD` or empty tree SHA |
| Step 99 failure blocks pipeline exit | User loses debug session work (already saved) | Non-critical — consider `|| true` instead of `|| exit 1` |
| Extra files in DOC_DIRECTORY confuse agent | Unexpected summaries | Acceptable per design decision; agent summarizes everything |
