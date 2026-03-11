# Research: Consolidate bin/ Script Patterns

## Design Reference
- [01-design.md](./01-design.md)

---

## 1. Source Scripts Analysis

### bin/build-feature (234 lines)

**Purpose:** Multi-stage feature development pipeline (design → research → plan → checklist → implement).

**Input Validation (lines 19-22):**
```bash
if [[ -z "$1" || -z "$2" || -z "$3" ]]; then
  echo "Usage: build-feature <feature-name> <doc-directory> <prompt>"
  exit 1
fi
```
- Requires 3 arguments
- Exits with code 1 on failure
- Usage message to stderr (implicit via echo)

**Global Variables (lines 27-42):**
| Variable | Value | Purpose |
|----------|-------|---------|
| `FEATURE_NAME` | `"$1"` | Feature identifier |
| `TIME_STAMP` | `$(date +%y%m%d%H%M)` | YYMMDDHHMM format |
| `DOC_DIRECTORY` | `"$2/$TIME_STAMP-$FEATURE_NAME"` | Output directory |
| `PROMPT` | `"${@:3}"` | All remaining args |
| `TERSENESS` | Hardcoded string | Verbosity directive for LLM |
| `CURSOR_MODEL` | `kimi-k2.5` (line 39 overrides 38) | Model selection |
| `IDE` | `"cursor"` | Target IDE |
| `AGENT_TYPE` | `"cursor"` | Agent selection |

**Directory Creation (line 44):**
- `mkdir -p $DOC_DIRECTORY` creates nested directory structure
- No error handling if mkdir fails

**Agent Functions:**

`run_cursor_agent()` (lines 50-62):
- Reads prompt from stdin via `local STDIN_PROMPT=$(cat)`
- Echoes prompt back (debugging?)
- Executes `cursor-agent --model $CURSOR_MODEL`
- Exits with code 1 on agent failure
- Logs stage completion

`run_claude_agent()` (lines 64-82):
- **Bug:** Contains logic for both claude and cursor paths
- When `AGENT_TYPE="claude"`: uses `claude --dangerously-skip-permissions`
- Otherwise: falls back to `cursor-agent --model $CURSOR_MODEL`
- Duplicates cursor-agent logic from run_cursor_agent()

**Helper Functions:**

`new_doc()` (lines 90-94):
- Input: filename
- Returns: `$DOC_DIRECTORY/$FILE_NAME`
- Simple path constructor

`wait_for_user()` (lines 98-113):
- Early return if `AGENT_TYPE="claude"`
- Prompts user with "Continue? (y/n)" style loop
- Accepts: y, Y, yes, Yes, YES
- Loops until valid yes response
- No exit on negative response, just logs

`build_prompt()` (lines 115-132):
- Input: filepath, instructions (varargs)
- Appends standard suffix:
  - Terseness directive
  - Output file instruction
  - IDE open instruction
  - Audio notification request
- Returns formatted prompt via echo

---

### bin/gen-doc (76 lines)

**Purpose:** Single-prompt AI task runner for document generation.

**Input Validation (lines 13-16):**
```bash
if [[ -z "$1" || -z "$2" ]]; then
  echo "Usage: gen-doc <doc-name> <prompt>"
  exit 1
fi
```
- Requires 2 arguments
- Same pattern as build-feature

**Global Variables (lines 21-29):**
| Variable | Value | Purpose |
|----------|-------|---------|
| `DOC_NAME` | `"$1"` | Document identifier |
| `PROMPT` | `"${@:2}"` | All remaining args |
| `TIME_STAMP` | `$(date +%y%m%d%H%M)` | YYMMDDHHMM format |
| `FILE_PATH` | `"$TIME_STAMP-$DOC_NAME.md"` | Output filename |
| `IDE` | `"cursor"` | Target IDE |
| `AGENT_TYPE` | `"cursor"` | Agent selection |

**Agent Functions:**

`run_cursor_agent()` (lines 33-44):
- Same stdin pattern as build-feature
- **Difference:** Uses bare `cursor-agent` without `--model` flag
- Same error handling

`run_claude_agent()` (lines 46-57):
- **Difference:** Only supports claude, no cursor fallback
- Same stdin pattern
- Same error handling

**Prompt Construction (lines 59-69):**
- Inline construction (not a function)
- Similar structure to build_prompt() output:
  - Preference directive
  - Task reference with `$DOC_NAME`
  - User prompt insertion
  - Output file instruction
  - IDE open instruction
  - Audio notification request

**Execution Flow (lines 71-75):**
- Conditional on `AGENT_TYPE`
- Pipes prompt to selected agent

---

## 2. Pattern Comparison Matrix

| Pattern | build-feature | gen-doc | Notes |
|---------|---------------|---------|-------|
| Input validation | Lines 19-22 | Lines 13-16 | Same pattern, different arg count |
| Arg count check | 3 args | 2 args | Could be parameterized |
| Usage message | Embedded | Embedded | Could be passed as param |
| Timestamp format | `%y%m%d%H%M` | `%y%m%d%H%M` | Identical |
| TIME_STAMP var | Yes | Yes | Same purpose |
| IDE var | Yes | Yes | Same purpose |
| AGENT_TYPE var | Yes | Yes | Same purpose |
| TERSENESS var | Yes | No | gen-doc uses inline preference |
| CURSOR_MODEL var | Yes (with override bug) | No | gen-doc uses bare cursor-agent |
| run_cursor_agent() | Yes (with --model) | Yes (bare) | Inconsistent |
| run_claude_agent() | Yes (with cursor fallback) | Yes (claude only) | Inconsistent |
| Agent error handling | `exit 1` | `exit 1` | Identical |
| new_doc() helper | Yes | No | gen-doc doesn't need it |
| wait_for_user() | Yes | No | Design says keep this way |
| build_prompt() | Yes | No | gen-doc uses inline |
| Directory output | Yes | No | Different use cases |
| Trailing slash handling | Yes (line 32) | No | gen-doc has no directory |

---

## 3. Intricacies & Challenges

### 3.1 CURSOR_MODEL Override Bug

**Location:** build-feature lines 38-39
```bash
CURSOR_MODEL=opus-4.5-thinking
CURSOR_MODEL=kimi-k2.5  # This overwrites the previous line
```

The first assignment has no effect. This appears unintentional.

### 3.2 run_claude_agent() Logic Duplication

In build-feature, `run_claude_agent()` handles both claude AND cursor:
```bash
if [[ "$AGENT_TYPE" == "claude" ]]; then
  # run claude
else
  # run cursor-agent  <-- confusing!
fi
```

This makes the function name misleading. The `run_cursor_agent()` function is never called when `AGENT_TYPE=cursor` in build-feature.

### 3.3 Agent Abstraction Inconsistency

**gen-doc:** `run_claude_agent()` only runs claude
**build-feature:** `run_claude_agent()` can run claude OR cursor

This creates a trap: sourcing helpers into both files requires reconciling this behavior.

### 3.4 Model Flag Inconsistency

**build-feature:** Always uses `--model $CURSOR_MODEL`
**gen-doc:** Never uses `--model` flag

Per design decision, gen-doc should adopt the `--model` flag for consistency.

### 3.5 AGENT_TYPE Conditional Logic

Both scripts check `AGENT_TYPE` at the top level to decide which function to call. This could be abstracted into `run_agent()` that internally checks `AGENT_TYPE`.

### 3.6 Stdin Reading Pattern

Both use `local STDIN_PROMPT=$(cat)` then `echo "$STDIN_PROMPT"`. The echo appears to be for debugging/logging but may be unnecessary.

### 3.7 Trailing Slash Normalization

build-feature has: `DOC_DIRECTORY="${DOC_DIRECTORY%/}"`
- Removes trailing slash if present
- Prevents double slashes in paths
- Not needed in gen-doc (no directory construction)

### 3.8 Prompt Construction Differences

**build_prompt()** appends suffix after main prompt.
**gen-doc** interleaves preference directive before user prompt.

Order may affect LLM behavior. Need to standardize.

### 3.9 Function Export/Scope

Bash functions are not exported to subshells by default. Sourcing helpers makes functions available, but variables need consideration:
- Variables set in helpers will be in global scope
- Need to ensure no naming collisions

### 3.10 Error Propagation

Both scripts use `exit 1` on failures. When sourced, `exit` in a function exits the entire shell, not just the function. Must use `return` in helper functions.

---

## 4. Prior Art & Existing Solutions

### 4.1 Other bin/ Scripts

| Script | Purpose | Reusable Patterns |
|--------|---------|-------------------|
| `bin/init-git-lfs.sh` | Git LFS setup | No shared patterns |
| `bin/make_executable.sh` | chmod wrapper | Single command, no relevance |

### 4.2 Existing Feature Documents

All feature development in this repo follows the build-feature pattern:

| Feature | Path | Structure |
|---------|------|-----------|
| ConvertOneShot | `docs/2603110912-ConvertOneShot/` | 01-design.md, 02-research.md, 03-plan.md, 04-checklist.md |
| UpdateReadme | `docs/2603110850-UpdateReadme/` | Same structure |
| BehaviorConsolidation | `docs/2603110930-BehaviorConsolidation/` | Same structure (in progress) |

### 4.3 No Existing Shared Libraries

- No `lib/` directory
- No `common.sh` or `utils.sh`
- No prior attempt at script consolidation
- First effort to DRY bin/ scripts

---

## 5. Relevant Files Summary

| File | Path | Purpose | Research Relevance |
|------|------|---------|-------------------|
| Main source | `bin/build-feature` | Multi-stage pipeline | Source of shared patterns |
| Secondary source | `bin/gen-doc` | Single-stage runner | Target for consolidation |
| Helpers target | `bin/helpers.sh` (to create) | Shared library | Output of this feature |
| Design doc | `docs/2603110930-BehaviorConsolidation/01-design.md` | Requirements | Input reference |
| This research | `docs/2603110930-BehaviorConsolidation/02-research.md` | Deep analysis | Output document |
| TODOs | `TODOS.md` | Task list | Line 12 explicitly requests this work |
| README | `README.md` | Documentation | May need updates post-implementation |

---

## 6. Unknowns / Open Questions

1. **Should `run_agent()` be a single function?**
   - Option A: Single function with internal AGENT_TYPE check
   - Option B: Keep separate functions, add thin wrapper
   - Decision needed for API design **DECISION**: Option A

2. **How to handle the gen-doc model flag?**
   - gen-doc currently uses bare `cursor-agent`
   - Should it use `--model $CURSOR_MODEL` per design decision
   - Need to verify cursor-agent accepts --model flag in all versions
   - **DECISION**: Yes, use `--model $CURSOR_MODEL`

3. **Should helpers.sh be executable?**
   - Library files typically are not executable
   - But bin/ convention suggests 755 permissions
   - Decision: not executable (sourced, not run)

4. **What if user sets AGENT_TYPE to unsupported value?**
   - Current scripts: undefined behavior
   - Should validate in `init_globals()` or `run_agent()`
   - **DECISION**: Validate in `init_globals()`

5. **Should CURSOR_MODEL be configurable via env var?**
   - Currently hardcoded then overwritten
   - **DECISION**: Yes, support via env var

---

## 7. Testing Considerations

### 7.1 Unit Testable Functions

| Function | Test Cases |
|----------|------------|
| `validate_args_min()` | Sufficient args, insufficient args, exact count |
| `init_globals()` | All variables set correctly, timestamp format |
| `new_doc()` | Path format, trailing slash handling |
| `build_prompt()` | Suffix appended correctly, varargs handling |

### 7.2 Integration Test Scenarios

| Scenario | Test |
|----------|------|
| AGENT_TYPE=cursor | Uses cursor-agent --model |
| AGENT_TYPE=claude | Uses claude CLI |
| Missing helpers | Source failure propagation |
| Agent failure | Error logged, exit 1 |

### 7.3 Edge Cases Identified

1. **Empty prompt string** - `${@:N}` with no additional args
2. **Directory with trailing slash** - Already handled in build-feature
3. **Non-interactive mode** - `wait_for_user()` needs stdin check
4. **Special characters in filenames** - Not currently escaped
5. **Spaces in DOC_NAME** - Would break FILE_PATH construction

---

## 8. Implementation Risks

| Risk | Mitigation |
|------|------------|
| Breaking existing scripts | Test both thoroughly after refactor |
| Variable scope pollution | Use local in functions, prefix globals |
| Exit vs return confusion | Use return in all helper functions |
| Regression in wait_for_user() | Test interactive loop behavior |
| claude fallback removal | Ensure build-feature still works with AGENT_TYPE=cursor |
