# Debug Stage - Research

## 1. Current Pipeline Architecture

`build-feature` runs 5 stages sequentially. Each stage pipes a prompt to `run_agent()` and gates on `|| exit 1`.

| Stage | File | Interactive | `wait_for_user` | Output File |
|-------|------|-------------|-----------------|-------------|
| 1 Design | `build-feature:44-61` | Yes | Yes | `01-design.md` |
| 2 Research | `build-feature:63-81` | Yes | Yes | `02-research.md` |
| 3 Plan | `build-feature:83-103` | Yes | Yes | `03-plan.md` |
| 4 Checklist | `build-feature:105-116` | Yes | Yes | `04-checklist.md` |
| 5 Implementation | `build-feature:118-134` | No (`--no-interactive`) | No | None (modifies codebase) |
| **6 Debug (proposed)** | — | **Yes** | **No** | `05-debug.md` |

## 2. Key Functions Analysis

### `run_agent()` — `helpers.sh:42-79`

Accepts prompt via stdin, dispatches to `claude` or `cursor-agent`.

- `--no-interactive` adds `--print` flag (non-interactive output mode)
- Without `--no-interactive`, agent runs interactively (user can converse)
- Returns non-zero on failure; caller uses `|| exit 1`

**Challenge for debug stage:** The design calls for `echo "$DEBUG_PROMPT" | run_agent "$STAGE"` (no `--no-interactive`). This is the same pattern as stages 1-4. Works as-is — no changes to `run_agent()` needed.

### `build_prompt()` — `helpers.sh:122-136`

Appends standard suffix: terseness directive, output file path, IDE open, audio notification.

**Challenge for debug stage:** The design does NOT use `build_prompt()`. Stage 5 (implementation) also bypasses it — uses raw `echo "$PROMPT" | run_agent`. Debug follows the same pattern since:
- Debug prompt needs custom structure (no standard suffix wanted)
- No output file instruction needed in the standard suffix form (agent writes conversationally)
- No IDE open or audio (per design decisions)

### `new_doc()` — `helpers.sh:83-87`

Returns `"$dir/$filename"` string. Does not create the file. Used by debug to get `05-debug.md` path.

### `wait_for_user()` — `helpers.sh:91-118`

Prompts for y/n to continue. Skipped when `AGENT_TYPE=claude` or non-TTY.

**Not used by debug stage** — it's the last stage, no gating needed.

## 3. Design Element Deep-Dive

### 3.1 Interactive Agent Session

**How it works:** When `run_agent()` is called without `--no-interactive`:
- Claude: `claude --dangerously-skip-permissions` (interactive CLI session)
- Cursor: `cursor-agent --model "$CURSOR_MODEL"` (interactive)

The prompt is piped via stdin. The agent reads it, then enters interactive mode where the developer can type follow-up messages.

**Intricacy:** The prompt is sent via `echo "$prompt" | claude`. Claude CLI accepts initial prompt via stdin and then continues interactively if stdin is a pipe that closes. This works because `claude` detects TTY on stdout and enters interactive mode. Verified by stages 1-4 which use this exact pattern.

### 3.2 File Naming: `05-debug.md`

Numbering rationale from design: implementation (stage 5) produces no output file, so debug takes slot `05`.

Existing file sequence:
```
01-design.md      (stage 1)
02-research.md    (stage 2)
03-plan.md        (stage 3)
04-checklist.md   (stage 4)
                  (stage 5 - no file)
05-debug.md       (stage 6)
```

No conflict. `new_doc()` returns path only; file creation is handled by the agent writing to it.

### 3.3 Prompt Context

Design DECISION: Reference all prior docs, not just checklist.

Current implementation prompt (stage 5, `build-feature:122-128`) references only `$CHECKLIST_FILE`. The debug prompt in the design references only `$CHECKLIST_FILE`.

**Gap:** Design decision says "reference the other files for full context" but the code snippet only references `$CHECKLIST_FILE`. Implementation should add `$DESIGN_FILE`, `$RESEARCH_FILE`, and `$PLAN_FILE` to the debug prompt.

### 3.4 Non-Interactive Mode Handling

Design RECOMMENDATION: Skip debug entirely in non-interactive mode.

**Challenge:** There is no global `--no-interactive` flag for `build-feature` itself. The `--no-interactive` flag exists only as a parameter to `run_agent()`. Currently, only stage 5 passes it.

**Options:**
1. Check `AGENT_TYPE` — but this isn't a reliable proxy for non-interactive mode
2. Add a script-level flag (e.g., `--skip-debug` or `--no-debug`)
3. Since debug is inherently interactive, simply run it. If the user is running non-interactively (piped stdin, no TTY), the agent will exit immediately

**Simplest approach:** No special handling needed. If stdin is not a TTY, `claude` in interactive mode will read the prompt and exit after processing. The debug stage effectively becomes a no-op in non-interactive contexts.

### 3.5 Error Handling

Pattern from existing stages: `run_agent "$STAGE" || exit 1`

Debug follows the same pattern. If the agent crashes:
- `run_agent` returns non-zero
- `|| exit 1` propagates failure
- Partial `05-debug.md` content preserved (written by agent before crash)
- Script prints error via `run_agent`'s stderr message

### 3.6 Audio Notification

Design DECISION: No audio for debug stage.

Stage 5 includes audio in its prompt directly (`build-feature:128`). Debug stage omits this. Since debug doesn't use `build_prompt()`, no audio suffix is appended.

## 4. Prior Art in Repo

### Stage Pattern (stages 1-4)
Each follows: set `STAGE` var, call `new_doc()`, define prompt, pipe through `build_prompt()` to `run_agent()`, call `wait_for_user()`.

### Implementation Stage Pattern (stage 5)
Bypasses `build_prompt()` and `wait_for_user()`. Uses `echo "$PROMPT" | run_agent "$STAGE" --no-interactive`. Debug follows this bypass pattern but without `--no-interactive`.

### TODOS.md Entry
Line 14: `[ ] Add a 'debug' stage after implementation so user can continue to provide updates in the context.. Then write out all findings to a DEBUG.md file`

## 5. Challenges & Risks

| Challenge | Detail | Mitigation |
|-----------|--------|------------|
| Prompt via pipe + interactive | `echo "prompt" \| claude` must still allow interactive follow-up | Stages 1-4 prove this works |
| Agent writes to file | Agent must create `05-debug.md` and append conversation | Include explicit instruction in prompt |
| Partial file on crash | Agent may crash mid-write | Acceptable — documented in design as expected behavior |
| Variable scope | `$CHECKLIST_FILE` set in stage 4, used in stage 6 | Works — bash variables persist in same script execution |
| No `build_prompt()` | Debug prompt is manually constructed | Consistent with stage 5 pattern |
| TTY detection | Non-interactive environments may cause agent to hang or exit unexpectedly | Test with both TTY and non-TTY stdin |

## 6. Relevant Files

| File | Path | Relevance |
|------|------|-----------|
| build-feature | `bin/build-feature` | Main pipeline script. Debug stage appended at line 135. |
| helpers.sh | `bin/helpers.sh` | Shared functions: `run_agent`, `new_doc`, `build_prompt`. No changes needed. |
| TODOS.md | `TODOS.md:14` | Tracks debug stage as pending item. Mark complete after implementation. |
| Design doc | `docs/2603111257-DebugStage/01-design.md` | Full design specification for this feature. |
| Example stage 5 | `bin/build-feature:118-134` | Pattern to follow: bypasses `build_prompt()`, no `wait_for_user()`. |
| Example stages 1-4 | `bin/build-feature:44-116` | Interactive stage pattern with `build_prompt()` and `wait_for_user()`. |
| gen-doc | `bin/gen-doc` | Alternative script using same helpers. Not affected by debug stage. |
| Makefile | `Makefile` | Test targets. May want to add debug-specific test target. |

## 7. Implementation Summary

The debug stage requires **one change**: append ~15 lines to `bin/build-feature` after line 134. No changes to `helpers.sh` or other files. The pattern is proven by 5 existing stages. Key decision: include all doc references in the prompt (not just checklist) per the design decision in section "Unknowns #3".
