# Implementation Checklist: Non-interactive Checklist Generation

## Tasks

- [x] **1. Add `--no-interactive` flag**
  - [x] Open `bin/build-feature`, line 142
  - [x] Change `run_agent "$STAGE"` to `run_agent "$STAGE" --no-interactive`
  - [x] Confirm the diff matches: `echo "$FINAL_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1`

- [x] **2. Remove `wait_for_user` call**
  - [x] Open `bin/build-feature`, line 144
  - [x] Delete the line `wait_for_user "$CHECKLIST_FILE"`
  - [x] Confirm `append_resume "$PROMPTS_FILE"` on line 143 remains unchanged

- [x] **3. Verify final state of lines 142–143**
  - [x] Line 142: `echo "$FINAL_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1`
  - [x] Line 143: `append_resume "$PROMPTS_FILE"`
  - [x] No other lines in the Stage 4 block are modified

## Verification

- [ ] Run `bin/build-feature` end-to-end — no interactive prompt between plan and implementation stages
- [ ] Verify `04-checklist.md` is created with valid content
- [ ] Verify checklist is appended to the plan file
- [ ] Confirm Stage 5 proceeds without errors (it references `$CHECKLIST_FILE`)
- [ ] Test with `AGENT_TYPE=claude`
- [ ] Test with `AGENT_TYPE=cursor`
