# Plan: Remove Checklist IDE Opening

- [Design Doc](01-design.md)
- [Research Doc](02-research.md)

## Change

**File**: `bin/build-feature`, line 141

Replace `build_prompt` call with an inlined prompt that omits the IDE-open instruction.

## Before

```bash
FINAL_PROMPT=$(build_prompt "$CHECKLIST_FILE" "$CHECKLIST_PROMPT")
```

`build_prompt` expands to:
```
$CHECKLIST_PROMPT

$TERSENESS

Output everything to the file '$CHECKLIST_FILE'.

Then open the file '$CHECKLIST_FILE' in the IDE '$IDE' so I can review it.
```

## After

```bash
FINAL_PROMPT="$CHECKLIST_PROMPT

$TERSENESS

Output everything to the file '$CHECKLIST_FILE'."
```

The IDE-open line is removed. Everything else stays identical.

## Steps

1. Edit `bin/build-feature` line 141
2. Replace `FINAL_PROMPT=$(build_prompt "$CHECKLIST_FILE" "$CHECKLIST_PROMPT")` with the inlined version above
3. No other files change

## Context

- Lines 142-144 (`append_prompt`, `run_agent`, `append_resume`) are untouched
- `$TERSENESS` is set by `init_globals` in `helpers.sh` — already available in scope
- `$CHECKLIST_FILE` and `$CHECKLIST_PROMPT` are set on lines 134-138 — unchanged
- Follows the same pattern used by Stage 5 (line 149+), Stage 6, and `bin/clean-room`

## Verification

1. Run `build-feature` end-to-end
2. Confirm `04-checklist.md` is generated with correct content
3. Confirm `04-checklist.md` is NOT opened in IDE
4. Confirm Stages 1-3 still open their files in IDE
5. Confirm checklist content is appended to `$PLAN_FILE`
