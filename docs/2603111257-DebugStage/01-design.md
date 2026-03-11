# Debug Stage - Design Document

## Overview

Add a **debug** stage (Stage 6) to `build-feature` that runs after implementation. It starts an interactive agent session where the developer can instruct the agent to fix bugs found during manual testing. All conversation output is written to `05-debug.md` in the same feature document directory.

> Note: This is the 6th stage in execution order (stages 1-4 are docs, stage 5 is implementation, stage 6 is debug). The output file is numbered `05` since implementation (stage 5) produces no output file.

## Motivation

After implementation completes, bugs are common. Currently there's no structured way to capture the debugging/fix cycle. The debug stage provides:
- A guided interactive session for post-implementation fixes
- A persistent record of what was changed and why during debugging

## User Flow

### Happy Path

1. Implementation stage completes successfully
2. Script prints: `"==> Starting debug session. Describe bugs to fix. Output: 05-debug.md"`
3. An interactive agent session starts with instructions to:
   - Write all conversation/changes to `<DOC_DIRECTORY>/05-debug.md`
   - Reference the checklist and implementation context
4. Developer describes a bug, agent fixes it, output is appended to `05-debug.md`
5. Developer can describe multiple bugs in the same session
6. Session ends when the developer exits the agent (Ctrl+C or `/exit`)
7. Script prints: `"==> Debug session completed"`

### Unhappy Paths

| Scenario | Behavior |
|---|---|
| Implementation stage failed (exit 1) | Debug stage never runs. Existing `exit 1` propagation handles this. |
| Agent crashes mid-debug | Script catches non-zero exit, prints error, exits 1. `05-debug.md` contains partial output up to crash point. |
| Developer exits immediately (no bugs) | Empty or minimal `05-debug.md` is created. This is fine — not all features have bugs. |
| `--no-interactive` flag passed to build-feature | **Open question** — see Unknowns below. |
| Debug file path conflicts | Not possible — timestamp in directory name ensures uniqueness. |
| Agent cannot reproduce/fix the bug | Developer describes the issue; agent writes attempted fixes to `05-debug.md`. Developer can exit and debug manually. |

## Technical Design

### Stage Definition (in `build-feature`)

```bash
#
# Stage 6: Debug
#
STAGE="debug"
DEBUG_FILE=$(new_doc "$DOC_DIRECTORY" "05-debug.md")
DEBUG_PROMPT="You are an expert at debugging and fixing software issues.
You are tasked to work on $FEATURE_NAME.
The implementation plan is in '$CHECKLIST_FILE'.

The developer will describe bugs or issues found after implementation.
Fix each issue as described.

Write all conversation output, including what was changed and why, to '$DEBUG_FILE'.

Each interaction should be recorded in '$DEBUG_FILE'."

echo "==> Starting debug session. Describe bugs to fix. Output: $DEBUG_FILE"
echo "$DEBUG_PROMPT" | run_agent "$STAGE" || exit 1
echo "==> Debug session completed"
```

### Key Behaviors

- **Interactive by default**: Unlike implementation (which uses `--no-interactive`), debug runs interactively so the developer can describe bugs conversationally.
- **No `wait_for_user`**: Since debug is the last stage, there's no next stage to gate.
- **File naming**: `05-debug.md` — follows the sequential numbering after `04-checklist.md` (implementation has no output file).
- **Prompt context**: References `$CHECKLIST_FILE` so the agent understands what was implemented.

## Unknowns / Open Questions

1. **Non-interactive mode**: Should debug run when `--no-interactive` is passed? Options:
   - (a) Skip debug entirely in non-interactive mode
   - (b) Run debug non-interactively with a generic "check for bugs" prompt
   - (c) Always run interactively regardless of flag
   - **Recommendation**: (a) Skip — debug is inherently interactive.

2. **Multiple debug sessions**: If the developer runs `build-feature` again, a new directory is created. But what if they want to resume debugging an existing feature? This is out of scope for now but worth noting.

3. **Scope of agent instructions**: Should the debug prompt also reference `01-design.md` and `02-research.md` for fuller context, or is `04-checklist.md` sufficient? **DECISION** reference the other files for full context.

4. **Exit mechanism**: The agent session ends when the developer exits. Should we add explicit instructions (e.g., "type 'done' to exit")? Or rely on standard agent exit (Ctrl+C, `/exit`)? **DECISION** rely on standard agent exit (Ctrl+C, `/exit`)

5. **Audio notification**: Implementation currently plays audio on completion. Should debug also play audio when the session starts (to alert the developer that implementation is done and debug is ready)? **DECISION** no, don't play audio.

## Testing Plan

### Test Cases

| # | Case | Input | Expected Outcome |
|---|---|---|---|
| 1 | Happy path — debug runs after implementation | Normal `build-feature` run | Debug stage starts interactively after implementation completes. `05-debug.md` created. |
| 2 | Implementation fails | Implementation returns exit 1 | Debug stage does not run. Script exits 1. |
| 3 | Debug file created in correct directory | Normal run | `05-debug.md` lives in `<DOC_DIRECTORY>` alongside `01-04` files. |
| 4 | Debug prompt includes checklist reference | Inspect prompt passed to agent | Prompt contains path to `04-checklist.md`. |
| 5 | Agent exit with zero code | Developer exits cleanly | Script prints completion message, exits 0. |
| 6 | Agent exit with non-zero code | Agent crashes or errors | Script prints error, exits 1. |
| 7 | Non-interactive mode (if skip chosen) | `AGENT_TYPE=claude` or non-interactive | Debug stage is skipped. |
| 8 | Both agent types work | `AGENT_TYPE=claude` and `AGENT_TYPE=cursor` | Debug runs correctly with both agent backends. |
| 9 | Empty debug session | Developer exits immediately without input | `05-debug.md` may be empty or minimal. No error. |
| 10 | Multiple bugs in one session | Developer describes 2+ bugs | All fixes and conversation captured in `05-debug.md`. |

### Manual Verification

- Run full `build-feature` pipeline end-to-end
- Verify file structure: `01-design.md`, `02-research.md`, `03-plan.md`, `04-checklist.md`, `05-debug.md`
- Verify debug prompt content via `echo` before piping to agent
- Test with both `AGENT_TYPE=claude` and `AGENT_TYPE=cursor`
