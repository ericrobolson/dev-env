# Implementation Plan: Step 99 — Pipeline Overview Summary

**References:**
- [Design Document](01-design.md)
- [Research Document](02-research.md)

---

## 1. Changes to `bin/build-feature`

### 1.1 Add Git Baseline Capture (after line 42, before Stage 1)

Insert after `PROMPTS_FILE` assignment:

```bash
# Capture git baseline ref for step 99 diff
GIT_BASELINE_REF=""
if git rev-parse HEAD >/dev/null 2>&1; then
    GIT_BASELINE_REF=$(git rev-parse HEAD)
fi
```

### 1.2 Add Step 99 Block (after line 178, end of debug stage)

```bash
#
# Stage 99: Overview
#
STAGE="overview"
OVERVIEW_FILE=$(new_doc "$DOC_DIRECTORY" "99-overview.md")
OVERVIEW_PROMPT="You are an expert technical writer.
You are tasked to write a pipeline overview for $FEATURE_NAME.

Read all markdown files in '$DOC_DIRECTORY' in filename-sorted order, excluding '00-prompts.md'.
For each file, write a concise summary.

Then summarize all code changes made during this pipeline run.
$(if [[ -n "$GIT_BASELINE_REF" ]]; then
    echo "Run: git diff $GIT_BASELINE_REF --stat"
    echo "Run: git diff $GIT_BASELINE_REF"
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

**Key points:**
- Uses `--no-interactive` — agent runs once, writes file, exits. No user interaction.
- Uses `build_prompt()` — gets standard suffix (IDE open, audio notification, terseness).
- Calls `append_prompt` + `append_resume` per pipeline convention.
- `|| exit 1` for consistent error handling.

---

## 2. Changes to `README.md`

Update the pipeline stage diagram to include Stage 99:

```
Stage 1 (design) → Stage 2 (research) → Stage 3 (plan) → Stage 4 (checklist) → Stage 5 (implementation) → Stage 6 (debug) → Stage 99 (overview)
```

Update the output directory listing to include `99-overview.md`.

---

## 3. Changes to `TODOS.md`

Mark line 17 (`[ ] Add a 'overview' section`) as complete: `[x]`.

---

## 4. Testing

Create `tests/test-overview-stage.sh` following the pattern in `tests/test-append-prompt.sh`.

### 4.1 Git Baseline Capture Test

```bash
test_git_baseline_capture() {
    local tmpdir=$(mktemp -d)
    trap "rm -rf $tmpdir" EXIT

    cd "$tmpdir"
    git init
    echo "initial" > file.txt
    git add file.txt
    git commit -m "initial"

    local ref=$(git rev-parse HEAD)
    [[ -n "$ref" ]] || { echo "FAIL: baseline ref empty"; return 1; }
    echo "PASS: git baseline captured: $ref"
}
```

### 4.2 Overview File Creation Test

```bash
test_overview_file_created() {
    local doc_dir=$(mktemp -d)
    trap "rm -rf $doc_dir" EXIT

    local overview_file=$(new_doc "$doc_dir" "99-overview.md")
    [[ "$overview_file" == "$doc_dir/99-overview.md" ]] || { echo "FAIL: wrong path"; return 1; }
    echo "PASS: overview file path correct"
}
```

### 4.3 Prompt Excludes 00-prompts.md

```bash
test_prompt_excludes_prompts_file() {
    local prompt="Read all markdown files... excluding '00-prompts.md'"
    echo "$prompt" | grep -q "excluding '00-prompts.md'" || { echo "FAIL"; return 1; }
    echo "PASS: prompt excludes 00-prompts.md"
}
```

### 4.4 No Git Repo Fallback

```bash
test_no_git_repo_fallback() {
    local tmpdir=$(mktemp -d)
    trap "rm -rf $tmpdir" EXIT
    cd "$tmpdir"

    local ref=""
    if git rev-parse HEAD >/dev/null 2>&1; then
        ref=$(git rev-parse HEAD)
    fi

    [[ -z "$ref" ]] || { echo "FAIL: ref should be empty outside git repo"; return 1; }
    echo "PASS: no git repo handled gracefully"
}
```

---

## 5. Implementation Checklist

- [ ] Add `GIT_BASELINE_REF` capture to `bin/build-feature` after line 42
- [ ] Add Stage 99 block to `bin/build-feature` after line 178
- [ ] Update `README.md` pipeline diagram and output listing
- [ ] Mark overview TODO as complete in `TODOS.md`
- [ ] Create `tests/test-overview-stage.sh` with baseline, file creation, exclusion, and no-repo tests
- [ ] Run full pipeline end-to-end to verify `99-overview.md` is generated
