# Design: Remove Checklist File Opening in build-feature

## Overview

Remove the IDE-open instruction from the checklist stage (Stage 4) in `bin/build-feature`. The checklist is auto-generated and non-interactive (`--no-interactive`), so opening it in the IDE is unnecessary.

## Current Behavior

Stage 4 uses `build_prompt` which appends:
```
Output everything to the file '<filepath>'.
Then open the file '<filepath>' in the IDE '<IDE>' so I can review it.
```

This causes the agent to open `04-checklist.md` in Cursor, even though the stage is non-interactive and the user is never asked to review it (`wait_for_user` is not called).

## Proposed Change

Stop using `build_prompt` for the checklist stage. Instead, construct the prompt manually with only the "output to file" instruction — no "open in IDE" instruction.

Alternatively, the checklist prompt could inline its own output instruction without calling `build_prompt`.

## Scope

- **File**: `bin/build-feature`, lines ~134-143 (Stage 4: Checklist)
- **No changes** to `helpers.sh`, other stages, or any other behavior.

## User Flow

### Happy Path
1. User runs `build-feature`
2. Stages 1-3 proceed as normal (design, research, plan — each opens in IDE for review)
3. Stage 4 (checklist) runs non-interactively, generates checklist, does **not** open in IDE
4. Stages 5+ proceed as normal

### Unhappy Paths
1. **Checklist generation fails** — no change in behavior; script still exits on error via `|| exit 1`
2. **User wants to review checklist** — checklist is appended to the plan file (`$PLAN_FILE`), which was already reviewed. User can open `04-checklist.md` manually if needed.

## Unknowns / Questions

1. Should the implementation stage (Stage 5) also skip IDE opening? It also uses `--no-interactive` but currently doesn't use `build_prompt` — it already skips this. **No action needed.**
2. Does `build_prompt` need a flag to optionally skip the IDE-open line, or is inlining the prompt sufficient? Inlining is simpler and avoids changing the shared helper. **DECISION: inlining the prompt is sufficient**

## Testing Plan

| # | Test Case | Expected Result |
|---|-----------|-----------------|
| 1 | Run `build-feature` end-to-end | Checklist stage does NOT open `04-checklist.md` in IDE |
| 2 | Run `build-feature` end-to-end | Stages 1-3 still open their files in IDE as before |
| 3 | Run `build-feature` end-to-end | `04-checklist.md` is still generated with correct content |
| 4 | Run `build-feature` end-to-end | Checklist is still appended to `$PLAN_FILE` |
| 5 | Verify `build_prompt` helper | No changes — other stages using it are unaffected |
| 6 | Interrupt during checklist stage | Script exits cleanly (existing behavior preserved) |
