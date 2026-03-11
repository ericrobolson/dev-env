# Design: Add Non-Interactive Mode to `run_agent`

## Overview

Add an `interactive` argument to `run_agent()` in `helpers.sh` that controls whether the agent runs in interactive or non-interactive (`--print`) mode. Update all call sites in `build-feature` and `gen-doc`. The implementation stage in `build-feature` should run in non-interactive mode.

## Current State

- `run_agent()` accepts one argument: `stage` name
- Claude is invoked with: `claude --dangerously-skip-permissions`
- Cursor is invoked with: `cursor-agent --model "$CURSOR_MODEL"`
- Neither call uses `--print` (non-interactive) mode
- Both CLIs support `-p`/`--print` for non-interactive output

## Proposed Change

### New Signature

```bash
run_agent <stage_name> [--no-interactive]
```

- Default: interactive mode (current behavior)
- `--no-interactive`: adds `--print` flag to the underlying CLI call

### Agent Commands

| Mode | Claude | Cursor |
|------|--------|--------|
| Interactive (default) | `claude --dangerously-skip-permissions` | `cursor-agent --model "$CURSOR_MODEL"` |
| Non-interactive | `claude --dangerously-skip-permissions --print` | `cursor-agent --model "$CURSOR_MODEL" --print` |

### Call Site Changes

#### `bin/build-feature`

| Stage | Mode | Rationale |
|-------|------|-----------|
| Design | Interactive | User reviews output |
| Research | Interactive | User reviews output |
| Plan | Interactive | User reviews output |
| Checklist | Interactive | User reviews output |
| **Implementation** | **Non-interactive** | Runs autonomously, no user interaction needed |

```bash
# Implementation stage becomes:
echo "$IMPLEMENTATION_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1
```

#### `bin/gen-doc`

No change — remains interactive (default).

## User Flow

### Happy Path

1. User runs `build-feature <name> <dir> <prompt>`
2. Stages 1-4 run interactively — agent can ask clarifying questions, user reviews each output
3. Stage 5 (implementation) runs non-interactively — agent executes the plan without prompting the user, outputs result to stdout
4. Script completes successfully

### Unhappy Paths

1. **Non-interactive agent fails** — `run_agent` returns non-zero, script exits with error message: `"Error: <agent> agent failed at stage 'implementation'"`
2. **Empty prompt in non-interactive mode** — Same existing validation: returns error before invoking agent
3. **Invalid flag passed to `run_agent`** — Unrecognized argument is ignored (or optionally: error out with usage message)
4. **Non-interactive mode produces incomplete output** — No built-in retry. User must re-run. Consider: should `--print` mode failures be retried automatically? (See Unknowns)
5. **Agent times out in non-interactive mode** — Depends on CLI timeout behavior. No special handling added.

## Unknowns / Questions

1. **Should `--no-interactive` be a flag or positional arg?** — Proposed as flag for clarity, but positional (`run_agent "stage" "no-interactive"`) would be simpler in bash.
2. **Should non-interactive mode set `--max-budget-usd` for Claude?** — Prevents runaway costs on unattended runs.
3. **Should non-interactive failures auto-retry?** — Current behavior: fail and exit. May want configurable retry count.
4. **Does cursor-agent `--print` behave the same as claude `--print`?** — Both support it, but output format differences may exist.
5. **Should `gen-doc` also support non-interactive?** — Not requested, but may be useful for CI/automation.
6. **Should `--dangerously-skip-permissions` still be used in non-interactive mode?** — Claude `--print` already skips the workspace trust dialog, but permissions may still apply.

## Testing Plan

### Unit Tests

| # | Test Case | Input | Expected |
|---|-----------|-------|----------|
| 1 | Interactive mode (default) | `echo "prompt" \| run_agent "test"` | Agent runs without `--print` flag |
| 2 | Non-interactive mode | `echo "prompt" \| run_agent "test" --no-interactive` | Agent runs with `--print` flag |
| 3 | Empty prompt + non-interactive | `echo "" \| run_agent "test" --no-interactive` | Returns error, does not invoke agent |
| 4 | Claude agent + non-interactive | `AGENT_TYPE=claude`, `--no-interactive` | Runs `claude --dangerously-skip-permissions --print` |
| 5 | Cursor agent + non-interactive | `AGENT_TYPE=cursor`, `--no-interactive` | Runs `cursor-agent --model X --print` |

### Integration Tests

| # | Test Case | Expected |
|---|-----------|----------|
| 6 | `build-feature` stages 1-4 | Run interactively (no `--print` in process list) |
| 7 | `build-feature` stage 5 | Runs with `--print` flag |
| 8 | `gen-doc` | Runs interactively (unchanged) |

### Manual Verification

| # | Test Case | How to Verify |
|---|-----------|---------------|
| 9 | Non-interactive implementation produces working code | Run full `build-feature`, check output compiles/runs |
| 10 | Interactive stages still allow user Q&A | Run stages 1-4, confirm agent prompts work |
| 11 | Error propagation in non-interactive | Kill agent mid-run, verify script exits with error |
