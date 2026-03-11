# Design: Configurable Agent Environment Variables

## Overview
Enable environment variable configuration for `AGENT_TYPE`, `IDE`, and `CURSOR_MODEL` with sensible defaults. Change default `AGENT_TYPE` from "cursor" to "claude".

## Current State

The `bin/helpers.sh` `init_globals()` function already supports environment variables:

```bash
IDE="${IDE:-cursor}"
AGENT_TYPE="${AGENT_TYPE:-cursor}"  # To change: default "claude"
CURSOR_MODEL="${CURSOR_MODEL:-kimi-k2.5}"
```

README.md has a TODO placeholder and outdated "Configuration" section claiming values are hardcoded.

## Unknowns & Questions

1. **Claude CLI availability**: Is `claude` command always available when users select `AGENT_TYPE=claude`? <- yes, it is always available when users select `AGENT_TYPE=claude`
2. **Model validation**: Should `CURSOR_MODEL` validate against allowed values or pass through blindly? <- pass through blindly
3. **IDE values**: Are values other than "cursor" valid? What happens with `IDE=vscode`? <- yes, pass through blindly
4. **Migration impact**: Will changing default `AGENT_TYPE` to "claude" break existing users who depend on cursor behavior? <- yes, break existing users who depend on cursor behavior
5. **Cross-script consistency**: Do `bin/build-feature` and `bin/gen-doc` need similar updates? <- yes, need similar updates. Prefer updating the shared helpers.sh file over individual scripts.

## User Flows

### Happy Flow: User Sets Custom Values

```bash
export AGENT_TYPE=claude
export IDE=cursor
export CURSOR_MODEL=kimi-k2.5
build-feature MyFeature docs "Build auth system"
```

1. User sets environment variables in shell or `.zshrc`
2. User runs `build-feature` or `gen-doc`
3. `init_globals()` reads env vars, applies defaults where unset
4. Script executes with selected agent/IDE/model
5. File opens in specified IDE

### Happy Flow: User Uses Defaults

```bash
build-feature MyFeature docs "Build auth system"
```

1. No env vars set
2. Defaults applied: `AGENT_TYPE=claude`, `IDE=cursor`, `CURSOR_MODEL=kimi-k2.5`
3. Script executes with claude agent

### Unhappy Flow: Invalid AGENT_TYPE

```bash
export AGENT_TYPE=invalid
build-feature MyFeature docs "Build auth system"
```

1. `init_globals()` validates `AGENT_TYPE`
2. Validation fails (not "cursor" or "claude")
3. Error printed: `Error: AGENT_TYPE must be 'cursor' or 'claude', got: invalid`
4. Script exits with code 1

### Unhappy Flow: Missing CLI for Selected Agent

```bash
export AGENT_TYPE=claude
# claude CLI not installed
build-feature MyFeature docs "Build auth system"
```

1. `AGENT_TYPE=claude` passes validation
2. `run_agent()` attempts to execute `claude --dangerously-skip-permissions`
3. Command not found error
4. Error printed: `Error: claude agent failed at stage 'design'`
5. Script exits with code 1

### Unhappy Flow: Invalid CURSOR_MODEL

```bash
export AGENT_TYPE=cursor
export CURSOR_MODEL=nonexistent-model
build-feature MyFeature docs "Build auth system"
```

1. `cursor-agent` receives invalid model
2. Agent may fail silently or with error
3. User sees generic failure message

## Implementation

### Changes to `bin/helpers.sh`

Line 28: Change default from "cursor" to "claude":

```bash
AGENT_TYPE="${AGENT_TYPE:-claude}"
```

### Changes to `README.md`

1. Update "Environment Variables" table (lines 24-28)
2. Update "Configuration" section (lines 161-179) to document env var usage
3. Remove "hardcoded" claims

## Testing Plan

### Test Cases

| ID | Test | Command | Expected |
|----|------|---------|----------|
| T1 | Default AGENT_TYPE | `build-feature Test docs "test"` | Uses claude agent |
| T2 | Explicit AGENT_TYPE=claude | `AGENT_TYPE=claude build-feature Test docs "test"` | Uses claude agent |
| T3 | Explicit AGENT_TYPE=cursor | `AGENT_TYPE=cursor build-feature Test docs "test"` | Uses cursor-agent |
| T4 | Invalid AGENT_TYPE | `AGENT_TYPE=invalid build-feature Test docs "test"` | Error, exit 1 |
| T5 | Default IDE | `build-feature Test docs "test"` | Opens in Cursor |
| T6 | Custom IDE | `IDE=vscode build-feature Test docs "test"` | Opens in VSCode |
| T7 | Default CURSOR_MODEL | `AGENT_TYPE=cursor build-feature Test docs "test"` | Uses kimi-k2.5 |
| T8 | Custom CURSOR_MODEL | `CURSOR_MODEL=opus-4.5-thinking build-feature Test docs "test"` | Uses opus model |
| T9 | Missing claude CLI | `AGENT_TYPE=claude build-feature Test docs "test"` (without claude installed) | Error, exit 1 |
| T10 | Documentation accuracy | Review README.md | All env vars documented with defaults |

### Regression Tests

- [ ] `gen-doc` works with default settings
- [ ] `build-feature` completes all 5 stages with default settings
- [ ] Interactive `wait_for_user` still works when `AGENT_TYPE=cursor`
- [ ] Non-interactive mode still skips prompts when `AGENT_TYPE=claude`

## Acceptance Criteria

- [ ] `AGENT_TYPE` defaults to "claude" when unset
- [ ] `IDE` defaults to "cursor" when unset
- [ ] `CURSOR_MODEL` defaults to "kimi-k2.5" when unset
- [ ] All three variables documented in README.md with descriptions
- [ ] Validation rejects invalid `AGENT_TYPE` values
- [ ] No regression in existing functionality
