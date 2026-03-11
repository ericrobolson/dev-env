# Research: Add Non-Interactive Mode to `run_agent`

Reference: [01-design.md](./01-design.md)

---

## 1. Current `run_agent` Implementation

**File:** `bin/helpers.sh:42-65`

```bash
run_agent() {
    local stage="$1"
    local prompt
    prompt=$(cat)
    # ... validates prompt, runs claude or cursor-agent, returns status
}
```

**Key facts:**
- Single positional arg: `stage`
- Reads prompt from stdin via `cat`
- Claude: `claude --dangerously-skip-permissions`
- Cursor: `cursor-agent --model "$CURSOR_MODEL"`
- Returns 1 on failure with error message including stage name
- Prints `✓ Stage '<stage>' complete` on success

---

## 2. CLI `--print` Flag Behavior

### Claude (`-p`/`--print`)

- Prints response to stdout and exits (non-interactive)
- Skips workspace trust dialog automatically
- Enables additional flags only available in print mode:
  - `--max-budget-usd <amount>` — caps API spend
  - `--output-format <format>` — `text`, `json`, `stream-json`
  - `--fallback-model <model>` — auto-fallback on overload
  - `--no-session-persistence` — don't save session to disk
  - `--include-partial-messages` — stream partial chunks
- `--dangerously-skip-permissions` still valid alongside `--print`
- No built-in retry or timeout mechanism

### Cursor Agent (`-p`/`--print`)

- Prints responses to console for scripts/non-interactive use
- "Has access to all tools, including write and shell" (per `--help`)
- Supports `--output-format` (`text`, `json`, `stream-json`)
- No `--max-budget-usd` equivalent
- No `--fallback-model` equivalent
- `--force`/`--yolo` controls permission bypass (separate from `--print`)

### Differences

| Aspect | Claude | Cursor |
|--------|--------|--------|
| Budget cap | `--max-budget-usd` | Not available |
| Fallback model | `--fallback-model` | Not available |
| Permission bypass | `--dangerously-skip-permissions` | `--force`/`--yolo` |
| Output format | Same options | Same options |
| Session persistence | `--no-session-persistence` | Not documented |

**Challenge:** Budget protection is only available for Claude. Cursor agent in `--print` mode has unrestricted tool access including shell and file writes with no cost cap.

---

## 3. Call Sites Analysis

### `bin/build-feature`

| Line | Stage | Current Call | Proposed Change |
|------|-------|-------------|-----------------|
| 60 | design | `build_prompt ... \| run_agent "$STAGE"` | No change (interactive) |
| 80 | research | `build_prompt ... \| run_agent "$STAGE"` | No change (interactive) |
| 102 | plan | `build_prompt ... \| run_agent "$STAGE"` | No change (interactive) |
| 115 | checklist | `build_prompt ... \| run_agent "$STAGE"` | No change (interactive) |
| 131 | implementation | `echo "$IMPLEMENTATION_PROMPT" \| run_agent "$STAGE"` | Add `--no-interactive` |

**Note on line 129:** There's a stray `"` on line 129 creating a syntax artifact — `"` after the closing heredoc-like string. This is a latent bug (extra empty string concatenated). Should be cleaned up.

### `bin/gen-doc`

| Line | Stage | Current Call | Proposed Change |
|------|-------|-------------|-----------------|
| 42 | generate | `build_prompt ... \| run_agent "generate"` | No change |

---

## 4. Implementation Challenges

### 4.1 Bash Flag Parsing in `run_agent`

Current signature: `run_agent <stage_name>`
Proposed: `run_agent <stage_name> [--no-interactive]`

Options for parsing:
1. **Simple `$2` check** — `if [[ "$2" == "--no-interactive" ]]` — minimal, fragile if more flags added later
2. **`getopts` loop** — standard bash, doesn't support long options natively
3. **Manual loop with `shift`** — most flexible for long options in bash

Recommendation: Simple `$2` check. Only one flag is proposed. YAGNI.

### 4.2 Implementation Stage Doesn't Use `build_prompt`

Stage 5 in `build-feature` (line 131) pipes directly with `echo`, not through `build_prompt`. This means:
- No `$TERSENESS` suffix
- No "open in IDE" instruction
- No "play audio notification" instruction (it has its own inline version)
- The `--print` mode won't produce IDE-opening behavior anyway (agent runs headless)

This is actually correct for non-interactive — you don't want the agent trying to open files in the IDE during headless execution.

### 4.3 `--print` Mode + `--dangerously-skip-permissions`

Per Claude help: `--print` already skips workspace trust dialog. `--dangerously-skip-permissions` bypasses all permission checks. Both can be used together. For unattended execution, using both is appropriate since there's no user to approve permissions.

### 4.4 Output Handling

In interactive mode, agent output goes to the terminal interactively. In `--print` mode, output goes to stdout. The current `run_agent` doesn't capture or redirect output — it flows to the caller's stdout/stderr. This is fine; `build-feature` doesn't process agent output.

### 4.5 Error Propagation

Current error handling: `if ! echo "$prompt" | claude ...; then return 1; fi`

This works identically for `--print` mode. Claude/cursor-agent return non-zero on failure in both modes.

---

## 5. Prior Art in the Repo

### `wait_for_user` in `helpers.sh:76-104`

Already has non-interactive detection: `[[ ! -t 0 ]]` checks if stdin is a terminal. Also skips for `AGENT_TYPE=claude`. This shows precedent for mode-aware behavior.

### `build_prompt` in `helpers.sh:108-122`

Appends IDE-open and audio-notification instructions. Not used by the implementation stage — consistent with non-interactive intent.

### Commit `47fa1b8` — "Added one shot mode"

Early precedent for non-interactive/scripted execution in the project.

### Commit `f014ad6` — "Moved functionality to shared library"

Established the `helpers.sh` pattern. All agent invocation goes through `run_agent`.

---

## 6. Relevant Files

| File | Path | Relevance |
|------|------|-----------|
| Shared helpers | `bin/helpers.sh` | Contains `run_agent` — the function to modify |
| Build feature script | `bin/build-feature` | Primary call site; stage 5 gets `--no-interactive` |
| Gen doc script | `bin/gen-doc` | Secondary call site; no changes needed |
| Makefile | `Makefile` | Test targets for both scripts |
| README | `README.md` | Documents CLI usage and env vars |
| Design doc | `docs/2603111115-AddNonInteractiveImplementation/01-design.md` | Feature specification |

---

## 7. Answers to Design Unknowns

| # | Question | Finding |
|---|----------|---------|
| 1 | Flag vs positional? | Flag (`--no-interactive`) is cleaner. Simple `$2` check is sufficient. |
| 2 | `--max-budget-usd`? | Only works with Claude `--print`. Should be added as optional env var (e.g., `MAX_BUDGET_USD`). Cursor has no equivalent. |
| 3 | Auto-retry? | Neither CLI has built-in retry. Don't add now — complexity not justified. |
| 4 | `cursor-agent --print` vs `claude --print`? | Both output to stdout as text. Same `--output-format` options. Behavioral parity sufficient for this use case. |
| 5 | `gen-doc` non-interactive? | Not needed now. Easy to add later — just pass `--no-interactive` to `run_agent`. |
| 6 | `--dangerously-skip-permissions` in `--print`? | Keep it. `--print` only skips trust dialog, not all permissions. Both flags together = fully unattended. |

---

## 8. Latent Issues Found

1. **`build-feature` line 129:** Stray `"` after the implementation prompt closing quote creates an empty string concatenation. Harmless but should be cleaned. DONE
2. **No timeout handling:** Neither `--print` mode has timeout. Long-running implementation could hang indefinitely. Consider `timeout` command wrapper as future enhancement. DEFERRED
3. **No output capture:** Non-interactive output goes to stdout but isn't logged to a file. Consider `tee` to a log file for post-mortem analysis. Update TODOS.md with this task for future implementation.
