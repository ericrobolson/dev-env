# Debug Stage - Implementation Plan

## References

- [Design Document](01-design.md)
- [Research Document](02-research.md)

## Summary

Append ~15 lines to `bin/build-feature` after line 134. No changes to `helpers.sh` or other files. One TODO item to mark complete.

## Changes

### 1. Add Stage 6 to `bin/build-feature` (after line 134)

Insert after `echo "==> Implementation completed"`:

```bash
#
# Stage 6: Debug
#
STAGE="debug"
DEBUG_FILE=$(new_doc "$DOC_DIRECTORY" "05-debug.md")
DEBUG_PROMPT="You are an expert at debugging and fixing software issues.
You are tasked to work on $FEATURE_NAME.

Reference these documents for full context:
- Design: '$DESIGN_FILE'
- Research: '$RESEARCH_FILE'
- Plan: '$PLAN_FILE'
- Checklist: '$CHECKLIST_FILE'

The developer will describe bugs or issues found after implementation.
Fix each issue as described.

Write all conversation output, including what was changed and why, to '$DEBUG_FILE'.
Each interaction should be recorded in '$DEBUG_FILE'."

echo "==> Starting debug session. Describe bugs to fix. Output: $DEBUG_FILE"
echo "$DEBUG_PROMPT" | run_agent "$STAGE" || exit 1
echo "==> Debug session completed"
```

### 2. Update `TODOS.md` line 14

Change:
```
[ ] Add a 'debug' stage after implementation...
```
To:
```
[x] Add a 'debug' stage after implementation...
```

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Interactive mode | Yes (no `--no-interactive`) | Debug is conversational by nature |
| `build_prompt()` | Not used | Matches stage 5 pattern; no standard suffix needed |
| `wait_for_user()` | Not used | Last stage, nothing to gate |
| Prompt context | All 4 prior docs | Per design decision #3 |
| Audio notification | None | Per design decision #5 |
| Non-interactive handling | No special handling | Agent exits naturally when stdin is not a TTY |

## What NOT to Change

- `helpers.sh` — no modifications needed
- `run_agent()` — works as-is for interactive mode
- `new_doc()` — returns path string, no file creation
- `Makefile` — no new targets needed
- `gen-doc` — unrelated script, unaffected

## Verification

After implementation, run `build-feature` end-to-end and confirm:

1. Debug stage starts after implementation completes
2. `05-debug.md` is created in the doc directory
3. Agent accepts interactive input (describe a bug, see it fixed)
4. Exiting agent (Ctrl+C or `/exit`) prints `"==> Debug session completed"`
5. If implementation fails (`exit 1`), debug stage does not run
6. Works with both `AGENT_TYPE=claude` and `AGENT_TYPE=cursor`

## Checklist

- [ ] Append debug stage block to `bin/build-feature` after line 134
- [ ] Include all 4 doc references (`$DESIGN_FILE`, `$RESEARCH_FILE`, `$PLAN_FILE`, `$CHECKLIST_FILE`) in debug prompt
- [ ] Use `run_agent "$STAGE"` without `--no-interactive`
- [ ] Do not use `build_prompt()` or `wait_for_user()`
- [ ] Mark TODOS.md line 14 as complete
- [ ] Manual end-to-end test with both agent types
