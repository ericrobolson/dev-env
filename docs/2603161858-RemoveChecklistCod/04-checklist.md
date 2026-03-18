# Checklist: Remove Checklist IDE Opening

## Implementation

- [ ] Edit `bin/build-feature` line 141
- [ ] Replace `FINAL_PROMPT=$(build_prompt "$CHECKLIST_FILE" "$CHECKLIST_PROMPT")` with inlined prompt:
  ```bash
  FINAL_PROMPT="$CHECKLIST_PROMPT

  $TERSENESS

  Output everything to the file '$CHECKLIST_FILE'."
  ```
- [ ] Verify no other lines are modified (lines 142-144 untouched)

## Verification

- [ ] Run `build-feature` end-to-end
- [ ] Confirm `04-checklist.md` is generated with correct content
- [ ] Confirm `04-checklist.md` is NOT opened in IDE
- [ ] Confirm Stages 1-3 still open their files in IDE
- [ ] Confirm checklist content is appended to `$PLAN_FILE`
