# Research: Convert bin/one-shot to bin/gen-doc

## Design Reference
- [01-design.md](./01-design.md)

---

## 1. Current Script Analysis

### bin/one-shot (78 lines)

**Purpose:** Single-prompt AI task runner for document generation.

**Key Components:**
- Input validation (lines 15-18): Requires 2 args, exits 1 on failure
- Global variables (lines 23-30):
  - `FEATURE_NAME="$1"` — to be renamed to `DOC_NAME`
  - `PROMPT="${@:2}"` — captures all remaining args
  - `TIME_STAMP=$(date +%y%m%d%H%M)` — YYMMDDHHMM format
  - `FILE_PATH="$TIME_STAMP-$FEATURE_NAME.md"` — output filename
  - `IDE="cursor"` — hardcoded IDE
  - `AGENT_TYPE="cursor"` — hardcoded agent selection

**Agent Functions:**
- `run_cursor_agent()` (lines 35-46): Uses `cursor-agent` CLI
- `run_claude_agent()` (lines 48-59): Uses `claude --dangerously-skip-permissions`

**LLM Prompt Construction (lines 61-71):**
- References `$FEATURE_NAME` in task description
- References `$FILE_PATH` for output
- References `$IDE` for file opening
- Requests audio notification

**Execution Flow (lines 73-77):**
- Checks `AGENT_TYPE`, pipes prompt to selected agent

---

## 2. Target Script Comparison

### bin/gen-doc (proposed) vs bin/one-shot (current)

| Aspect | Current | Proposed |
|--------|---------|----------|
| Filename | `bin/one-shot` | `bin/gen-doc` |
| Variable | `FEATURE_NAME` | `DOC_NAME` |
| Usage syntax | `<feature-name>` | `<doc-name>` |
| Output pattern | `YYMMDDHHMM-<feature-name>.md` | `YYMMDDHHMM-<doc-name>.md` |

---

## 3. Prior Art: Similar Renaming Patterns

**None found in repo.** No existing symlinks or alias patterns for backwards compatibility.

---

## 4. Related Files Analysis

### bin/build-feature (234 lines)

**Comparison with one-shot:**

| Feature | one-shot | build-feature |
|---------|----------|---------------|
| Args | 2 (name, prompt) | 3 (name, dir, prompt) |
| Output | Single file in `docs/` | Directory with 4 files |
| Stages | 1 (single prompt) | 5 (design→research→plan→checklist→implement) |
| `AGENT_TYPE` | Hardcoded, functional | Hardcoded, functional |
| `wait_for_user()` | Not present | Present (lines 98-113) |
| `build_prompt()` | Inline | Function (lines 115-132) |

**Shared patterns:**
- Both use `cursor-agent` or `claude` based on `AGENT_TYPE`
- Both use timestamped naming
- Both open files in Cursor IDE
- Both request audio notification

---

## 5. Documentation References

### README.md (198 lines)

**one-shot references found:**
- Line 9: Core Tools table entry
- Line 51-53: Shell function definition
- Line 63: Usage section header
- Line 70: Syntax `<feature-name>`
- Line 76: Example command
- Line 81: Output description
- Line 158: Configuration section
- Line 182: Troubleshooting section

### TODOS.md (8 lines)

**Related items:**
- Line 1: "Convert 'one-shot' to 'gen-doc'" — this task
- Line 7: "Update `one-shot` to follow patterns in `build-feature`"
- Line 8: "Maybe even combine functionality into a shared file"

---

## 6. Intricacies & Challenges

### Variable Renaming Scope

**Must change in bin/gen-doc:**
1. `FEATURE_NAME` → `DOC_NAME` (line 23)
2. `$FEATURE_NAME` in LLM prompt (line 62)

**Must NOT change:**
- `FILE_PATH` pattern — already uses variable, output format unchanged
- `AGENT_TYPE`, `IDE` — unrelated to rename

### Documentation Updates Required

| File | Lines | Changes |
|------|-------|---------|
| README.md | 9 | Table entry |
| README.md | 51-53 | Shell function |
| README.md | 63 | Section header |
| README.md | 70 | Syntax line |
| README.md | 76 | Example |
| README.md | 81 | Output description |
| README.md | 158 | Configuration |
| README.md | 182 | Troubleshooting |
| TODOS.md | 1 | Mark complete |
| TODOS.md | 7-8 | Update references |

---

## 7. Backwards Compatibility

**Decision:** Hard break (no symlink/alias)

**Impact:**
- Users with `one-shot()` shell function in `.zshrc`/`.bashrc` must update
- No deprecation period
- Clean cut reduces maintenance

---

## 8. Testing Considerations

**From design document:**
- GD-1: `bin/gen-doc` exists and is executable
- GD-2: `bin/one-shot` does not exist
- GD-3: No-args shows usage with `<doc-name>`, exits 1
- GD-4: Successful run creates correct file

**Verification methods:**
```bash
test -x bin/gen-doc                    # GD-1
test ! -f bin/one-shot                 # GD-2
bin/gen-doc 2>&1 | grep -q "<doc-name>" # GD-3
grep -r "one-shot" README.md TODOS.md   # Should be empty
```

---

## 9. Unknowns

1. **Git history preservation** — Decision: delete/add, not `git mv`
2. **Shell function migration** — Users must manually update `.zshrc`/`.bashrc`
3. **Documentation timing** — Update README before or after script rename?

---

## 10. Relevant Files Summary

| File | Path | Purpose |
|------|------|---------|
| Source script | `bin/one-shot` | To be renamed to `gen-doc` |
| Comparison script | `bin/build-feature` | Pattern reference |
| Main docs | `README.md` | Update all `one-shot` references |
| Task tracker | `TODOS.md` | Mark conversion complete |
| This research | `docs/2603110912-ConvertOneShot/02-research.md` | This document |
