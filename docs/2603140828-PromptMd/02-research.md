# Research: Prompt Logging (`00-prompts.md`)

## 1. `append_prompt` Function (helpers.sh)

### Implementation Details

**Location:** `bin/helpers.sh` — currently 136 lines, 6 functions. `append_prompt` will be the 7th.

**Signature:** `append_prompt <filepath> <stage_name> <prompt_text>`

### Challenges

**Shell quoting/expansion:** Prompts contain `$VARIABLE` references (e.g., `$DESIGN_FILE`, `$FEATURE_NAME`) that are already expanded by the time they reach `append_prompt`. The function receives the final interpolated string — no expansion risk inside the function itself. However, the function must write content verbatim without shell interpretation of special chars (`$`, `` ` ``, `\`, `!`).

**Writing verbatim content:** `echo "$prompt_text"` can misbehave with `-n`, `-e`, `--` at start of content. Safer to use `printf '%s\n'` or `cat <<'EOF'`. Since prompts may contain any character, `printf '%s\n' "$prompt_text"` is the safest approach.

**4-backtick fencing:** Prompts from `build_prompt` contain lines like:
```
Output everything to the file '...'.
```
These don't contain triple backticks. But user-provided prompts (`$PROMPT`) could. The design specifies 4-backtick fences (``````````). An even safer approach: scan the prompt for the longest backtick run and use N+1, but the design says 4 is sufficient. Stick with 4.

**Non-critical failure:** The function must not `exit 1` or `return 1` in a way that triggers `|| exit 1` in the caller. Options:
1. Always `return 0` regardless of write success
2. Print warning to stderr on failure, return 0
3. Callers don't chain with `|| exit 1` for this call

Option 2 aligns with design decision #3. The callers should invoke `append_prompt` as a standalone statement (no `|| exit 1`).

**File creation vs append:** First call creates file with `# Prompts\n\n` header, subsequent calls append. Use `[[ -f "$filepath" ]]` check. Race condition is irrelevant (single-process pipeline).

### Existing Patterns in helpers.sh

| Pattern | Example | Relevance |
|---|---|---|
| Stderr warnings | `echo "Error: ..." >&2` (lines 6, 18, 35, 54, 64, 73) | Use same pattern for write failure warning |
| Variable naming | `local var="$1"` | Follow same local variable convention |
| Return vs exit | Functions use `return`, never `exit` | `append_prompt` must use `return 0` |
| Cat from stdin | `prompt=$(cat)` in `run_agent` (line 51) | Shows pattern for reading piped input |
| Printf not used | All output via `echo` | Could introduce `printf` for safety |

### Prompt Content After `build_prompt`

`build_prompt` (line 122-136) appends this suffix to every prompt:
```
<user instructions>

Be concise and to the point. Stick to facts. Be succinct and terse. Don't be verbose.

Output everything to the file '<filepath>'.

Then open the file '<filepath>' in the IDE '<IDE>' so I can review it.

If possible, play an audio notification to alert me that the file is ready to review.
```

The **full final prompt** (post-`build_prompt`) is what must be logged. This means `append_prompt` must be called after `build_prompt` but before `run_agent`.

### Integration Pattern

Current flow:
```bash
build_prompt "$FILE" "$PROMPT" | run_agent "$STAGE"
```

New flow requires capturing the built prompt to both log it and pipe it:
```bash
FINAL_PROMPT=$(build_prompt "$FILE" "$PROMPT")
append_prompt "$PROMPTS_FILE" "$STAGE" "$FINAL_PROMPT"
echo "$FINAL_PROMPT" | run_agent "$STAGE"
```

This is a structural change — the pipe pattern must be broken into capture-then-pipe. Affects all 6 stages in `build-feature`, 1 in `gen-doc`, and 2 in `clean-room`.

**Exception — implementation stage in `build-feature`:** Line 131 doesn't use `build_prompt`:
```bash
echo "$IMPLEMENTATION_PROMPT" | run_agent "$STAGE" --no-interactive
```
The raw `$IMPLEMENTATION_PROMPT` is what the agent receives. Log that directly.

**Exception — debug stage in `build-feature`:** Line 160 also doesn't use `build_prompt`:
```bash
echo "$DEBUG_PROMPT" | run_agent "$STAGE"
```
Same: log the raw `$DEBUG_PROMPT`.

**Exception — clean-room stages:** Lines 80, 112 don't use `build_prompt`. Analysis and compliance prompts include `$TERSENESS` directly. Stage 3 writes to file, never runs `run_agent`.

---

## 2. build-feature Integration

**File:** `bin/build-feature` — 162 lines, 6 stages.

### Stage-by-Stage Analysis

| Stage | Line | File | Uses `build_prompt` | Uses `run_agent` | Interactive |
|---|---|---|---|---|---|
| design | 60 | `01-design.md` | Yes | Yes | Yes (default) |
| research | 80 | `02-research.md` | Yes | Yes | Yes |
| plan | 102 | `03-plan.md` | Yes | Yes | Yes |
| checklist | 115 | `04-checklist.md` | Yes | Yes | Yes |
| implementation | 131 | N/A (codebase) | **No** | Yes | No (`--no-interactive`) |
| debug | 160 | `05-debug.md` | **No** | Yes | Yes |

### Key Observations

- **Stages 1-4** use `build_prompt | run_agent` pattern — need refactor to capture prompt
- **Stage 5 (implementation)** uses `echo "$PROMPT" | run_agent` — simpler, just add `append_prompt` before
- **Stage 6 (debug)** uses `echo "$PROMPT" | run_agent` — same as stage 5
- **Output dir:** `$DOC_DIRECTORY` is `<user-dir>/<YYMMDDHHMM>-<feature-name>/` (line 37)
- **Prompts file:** `$DOC_DIRECTORY/00-prompts.md`
- **Error handling:** All stages chain with `|| exit 1` — `append_prompt` must NOT be in that chain

### Prompt Variables Per Stage

| Stage | Variable | Contains `$` refs | Contains backticks |
|---|---|---|---|
| design | `$DESIGN_PROMPT` | `$PROMPT` | No |
| research | `$RESEARCH_PROMPT` | `$FEATURE_NAME`, `$DESIGN_FILE` | No |
| plan | `$PLAN_PROMPT` | `$FEATURE_NAME`, `$DESIGN_FILE`, `$RESEARCH_FILE` | No |
| checklist | `$CHECKLIST_PROMPT` | `$PLAN_FILE` | No |
| implementation | `$IMPLEMENTATION_PROMPT` | `$FEATURE_NAME`, `$CHECKLIST_FILE` | No |
| debug | `$DEBUG_PROMPT` | `$FEATURE_NAME`, `$DESIGN_FILE`, `$RESEARCH_FILE`, `$PLAN_FILE`, `$CHECKLIST_FILE`, `$DEBUG_FILE` | No |

All `$` variables are expanded before reaching `append_prompt` — content is safe literal text by that point.

---

## 3. gen-doc Integration

**File:** `bin/gen-doc` — 49 lines, 1 stage.

### Details

- **Stage:** `generate` (line 49)
- **Uses `build_prompt`:** Yes (line 49: `build_prompt "$FILE_PATH" "$FULL_PROMPT" | run_agent "generate"`)
- **Output dir:** `$DOC_DIRECTORY/$TIME_STAMP-$DOC_NAME/` (line 32)
- **Prompts file:** `$DOC_DIRECTORY/00-prompts.md`
- **Single refactor needed:** Break pipe into capture + log + pipe

### Note

Line 49 has leading whitespace (` build_prompt`). Cosmetic only but worth noting.

---

## 4. clean-room Integration

**File:** `bin/clean-room` — 163 lines, 3 stages (2 agent-run, 1 file-write-only).

### Stage-by-Stage Analysis

| Stage | Line | Uses `build_prompt` | Uses `run_agent` | Log? |
|---|---|---|---|---|
| analysis | 80 | **No** | Yes | Yes |
| compliance | 112 | **No** | Yes | Yes |
| implementation | 158 | **No** | **No** (writes to file) | Yes (already written to file) |

### Key Differences from build-feature

- **No `build_prompt` usage** — prompts embed `$TERSENESS` directly (lines 78, 110)
- **Stage 3 never runs `run_agent`** — writes prompt to `IMPLEMENTATION.md` (line 158). Could still log to `00-prompts.md` for consistency.
- **Output dir:** `$SPEC_DIR/$TIME_STAMP-$FEATURE_NAME/` (line 31)
- **Prompts file:** `$SPEC_DIR/00-prompts.md`

### Decision

Design says "DECISION: do this" for clean-room (question #2). All 3 stages should log. For stage 3, log the `$IMPLEMENTATION_PROMPT` even though it's also in `IMPLEMENTATION.md`.

---

## 5. Debug Stage Details

**Location:** `bin/build-feature` lines 139-161.

### How It Works

1. Creates empty `05-debug.md` (line 141: `echo "" >> "$DEBUG_FILE"`)
2. Sends initial prompt describing debugger role + file references
3. Runs interactively (no `--no-interactive`)
4. User types bug descriptions, agent fixes them
5. Agent writes conversation to `$DEBUG_FILE`
6. Exits when user terminates (`Ctrl+C` or `/exit`)

### Prompt Logging Scope

Design decision #4: log initial prompt only. The debug agent manages its own conversation record in `05-debug.md`. Multiple debug iterations would each need the agent to call `append_prompt`, which isn't feasible without modifying the agent loop. Initial prompt is sufficient for replay.

---

## 6. Fencing Strategy

### The Problem

Prompts may contain triple backticks (```) if the user's `$PROMPT` argument includes markdown. After `build_prompt`, the appended suffix doesn't contain backticks, but user input is unpredictable.

### Design Decision

Use 4-backtick fences (````````). This handles 99.9% of cases. A prompt would need to contain exactly ```````` on its own line to break it.

### Alternative (not chosen)

Scan prompt for longest consecutive backtick run, use N+1. More robust but overengineered for this use case.

### Output Format

```markdown
## Design

````
<full prompt text, may contain ``` inside>
````
```

---

## 7. Existing Patterns & Prior Art

### File Writing in Codebase

| Location | Pattern | Notes |
|---|---|---|
| `build-feature:141` | `echo "" >> "$DEBUG_FILE"` | Append to create file |
| `clean-room:158` | `echo "$IMPLEMENTATION_PROMPT" > "$IMPL_PROMPT_FILE"` | Overwrite with content |
| `helpers.sh:127` | `echo "$instructions..."` | Multi-line echo output |

### No Existing Prompt Logging

No `00-prompts.md`, no `append_prompt`, no prompt recording anywhere in the codebase. This is entirely new functionality.

### Markdown Generation

All markdown in the project is generated by AI agents. The scripts themselves don't generate structured markdown — `append_prompt` will be the first function to do so.

---

## 8. Test Infrastructure

**File:** `Makefile` — 3 manual test targets.

```makefile
test-build-feature:
	./bin/build-feature test-feature docs/ "This is a test prompt"
test-gen-doc:
	./bin/gen-doc SonataOverview Give me an overview of a sonata musical form.
test-clean-room:
	./bin/clean-room test-feature ./bin ./specs ./impl "Reimplement the helper utilities"
```

### Testing Approach

- No unit test framework (no bats, shunit2, shpec)
- Manual integration tests via Makefile
- No mocking or stubbing capability
- Tests run real agent calls (expensive, slow)

### Implications for Testing `append_prompt`

The design specifies 13 test cases (6 unit, 5 integration, 2 edge). Options:
1. **Bash script tests** — simple `assert` functions, run `append_prompt` directly
2. **Add bats** — proper bash testing framework
3. **Manual verification** — extend Makefile targets

Given the project's lightweight approach, a simple bash test script sourcing `helpers.sh` and calling `append_prompt` with assertions is most consistent.

---

## 9. Relevant Files

| File | Path | Relevance |
|---|---|---|
| Shared helpers | `bin/helpers.sh` | Where `append_prompt` will be added |
| Feature builder | `bin/build-feature` | 6 integration points for prompt logging |
| Doc generator | `bin/gen-doc` | 1 integration point |
| Clean room tool | `bin/clean-room` | 2-3 integration points |
| TODOs | `TODOS.md` | Line 14: the TODO driving this feature |
| Makefile | `Makefile` | Test targets to extend |
| Design doc | `docs/2603140828-PromptMd/01-design.md` | Feature specification |
| Debug stage design | `docs/2603111257-DebugStage/01-design.md` | Context for debug stage behavior |

---

## 10. Summary of Implementation Complexity

| Component | Changes | Complexity |
|---|---|---|
| `append_prompt` in `helpers.sh` | New function (~15 lines) | Low — file create/append with markdown formatting |
| `build-feature` stages 1-4 | Break pipe pattern into capture+log+pipe | Medium — structural change to 4 call sites |
| `build-feature` stages 5-6 | Add `append_prompt` call before `echo \| run_agent` | Low — single line addition each |
| `gen-doc` | Break pipe pattern into capture+log+pipe | Low — single call site |
| `clean-room` stages 1-2 | Add `append_prompt` call before `echo \| run_agent` | Low — single line addition each |
| `clean-room` stage 3 | Add `append_prompt` call before file write | Low — single line addition |
| Tests | New test script or Makefile targets | Medium — 13 test cases per design |
