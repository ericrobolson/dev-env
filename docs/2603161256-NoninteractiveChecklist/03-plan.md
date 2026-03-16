# Implementation Plan: Non-interactive Checklist Generation

**Related docs:**
- [Design](./01-design.md)
- [Research](./02-research.md)

---

## Change Summary

Make Stage 4 (Checklist) in `bin/build-feature` non-interactive. Two edits to one file.

---

## Steps

### Step 1: Add `--no-interactive` flag to `run_agent` call

**File:** `bin/build-feature`, line 142

```diff
-echo "$FINAL_PROMPT" | run_agent "$STAGE" || exit 1
+echo "$FINAL_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1
```

This passes `--print` to the underlying agent CLI via `run_agent` (defined in `bin/helpers.sh:40-79`). The agent still executes file writes — it only suppresses the interactive TUI. This matches the pattern used by Stage 5 (line 159) and Stage 99 (line 227).

### Step 2: Remove `wait_for_user` call

**File:** `bin/build-feature`, line 144

```diff
 append_resume "$PROMPTS_FILE"
-wait_for_user "$CHECKLIST_FILE"
```

Delete this line. No review pause is needed — the plan was already reviewed in Stage 3, and the debug stage (Stage 6) exists for corrections.

### Step 3: Keep `append_resume` (no change)

Line 143 stays as-is. In `--print` mode, `append_resume` either finds a session file and records it, or silently no-ops. Keeping it is consistent with Stages 5 and 99.

---

## Final Result

Lines 142–144 go from:

```bash
echo "$FINAL_PROMPT" | run_agent "$STAGE" || exit 1
append_resume "$PROMPTS_FILE"
wait_for_user "$CHECKLIST_FILE"
```

To:

```bash
echo "$FINAL_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1
append_resume "$PROMPTS_FILE"
```

---

## Verification

1. Run `bin/build-feature` end-to-end — confirm no interactive prompt between plan and implementation stages
2. Verify `04-checklist.md` is created with valid content
3. Verify checklist is appended to the plan file
4. Confirm Stage 5 proceeds without errors (it references `$CHECKLIST_FILE`)
5. Test with both `AGENT_TYPE=claude` and `AGENT_TYPE=cursor`

---

## Risk

Low. Two-line change following an established pattern. Failure mode (`exit 1`) is identical to other non-interactive stages.
