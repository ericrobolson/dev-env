# Design: Non-interactive Checklist Generation

## Problem

Stage 4 (Checklist) in `bin/build-feature` runs interactively — it launches the agent in interactive mode and then calls `wait_for_user`. This is unnecessary because the checklist stage is purely generative (reads plan, outputs checklist). No human input is needed during generation.

This creates a pause in the pipeline between Stage 3 (Plan) and Stage 5 (Implementation) that adds no value.

## Current Behavior

1. Agent launches in interactive mode (no `--no-interactive` flag)
2. Agent generates checklist from plan file
3. Agent appends checklist to plan file
4. `wait_for_user` is called — but skipped for `claude` agent type, blocks for `cursor` agent type
5. User must confirm before pipeline continues

## Proposed Change

Pass `--no-interactive` to `run_agent` for the checklist stage, and remove the `wait_for_user` call.

### Scope

- **File**: `bin/build-feature`, lines 130–144
- **Change**: Add `--no-interactive` flag to `run_agent "checklist"` call
- **Change**: Remove `wait_for_user "$CHECKLIST_FILE"` call

### Before

```bash
echo "$FINAL_PROMPT" | run_agent "$STAGE" || exit 1
append_resume "$PROMPTS_FILE"
wait_for_user "$CHECKLIST_FILE"
```

### After

```bash
echo "$FINAL_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1
append_resume "$PROMPTS_FILE"
```

## User Flows

### Happy Path

1. Pipeline reaches Stage 4
2. Agent runs non-interactively (`--print` mode)
3. Checklist is generated and appended to plan file
4. Pipeline proceeds directly to Stage 5 (Implementation)

### Unhappy Paths

| Scenario | Current Behavior | New Behavior |
|---|---|---|
| Agent fails to generate checklist | Interactive session allows user to retry/fix | Agent exits with error, pipeline halts via `exit 1` |
| Checklist output is malformed | User can review before continuing | Malformed checklist passes to implementation stage |
| Plan file is missing/empty | Agent can ask user for guidance | Agent fails, pipeline halts |

### Risk: No review before implementation

The user loses the chance to review/edit the checklist before implementation begins. This is acceptable because:
- The checklist is derived deterministically from the plan (already reviewed)
- The debug stage (Stage 6) exists for corrections
- The `cursor` agent type previously had a pause here, which will be removed

## Unknowns / Questions

1. **Should `append_resume` still be called?** In `--print` mode (non-interactive), there's no persistent session to resume. The other non-interactive stage (Implementation, line 159) still calls `append_resume` — should we be consistent or skip it? Currently `append_resume` silently no-ops if no session is found, so it's harmless either way. **DECISION: yes**

2. **Should the checklist file still be created via `new_doc`?** The checklist prompt tells the agent to append to the plan file, not write to the checklist file. Is `CHECKLIST_FILE` actually used downstream? Yes — Stage 5 references `$CHECKLIST_FILE` (line 154). Need to confirm the agent actually writes to it in `--print` mode. **DECISION: yes**

3. **`--print` mode output**: In non-interactive mode, `run_agent` passes `--print` to `claude`/`cursor-agent`. Does the agent still write files in `--print` mode, or does it only print to stdout? If it only prints, the checklist won't be written to disk and Stage 5 will fail.  **DECISION: it still writes to files**

## Testing Plan

### Test Cases

| # | Case | Steps | Expected |
|---|---|---|---|
| 1 | Happy path — claude agent | Run full pipeline with `AGENT_TYPE=claude` | Checklist stage completes without pause, checklist file exists, implementation stage uses it |
| 2 | Happy path — cursor agent | Run full pipeline with `AGENT_TYPE=cursor` | Same as above — no `wait_for_user` prompt appears |
| 3 | Agent failure | Simulate agent failure (e.g., bad prompt) | Pipeline exits with error at checklist stage |
| 4 | Missing plan file | Delete plan file before checklist stage | Pipeline exits with error |
| 5 | Checklist file written | After stage completes, verify `04-checklist.md` exists and has content | File exists with valid checklist markdown |
| 6 | Plan file updated | After stage completes, verify plan file has checklist appended | Plan file contains checklist section at end |
| 7 | End-to-end timing | Compare pipeline duration before/after | Checklist stage should be faster (no interactive overhead) |

### Manual Verification

- Run `bin/build-feature` end-to-end and confirm no interactive prompt appears between plan and implementation stages
- Verify the generated checklist quality is equivalent to interactive mode
