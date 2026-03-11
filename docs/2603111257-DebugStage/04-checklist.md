# Debug Stage - Implementation Checklist

## References

- [Plan](03-plan.md)

## Tasks

### 1. Add Debug Stage to `bin/build-feature`

- [ ] Open `bin/build-feature` and locate line 134 (`echo "==> Implementation completed"`)
- [ ] Insert `STAGE="debug"` assignment after line 134
- [ ] Insert `DEBUG_FILE=$(new_doc "$DOC_DIRECTORY" "05-debug.md")` call
- [ ] Insert `DEBUG_PROMPT` variable with:
  - [ ] Expert debugging system prompt
  - [ ] `$FEATURE_NAME` reference
  - [ ] `$DESIGN_FILE` reference
  - [ ] `$RESEARCH_FILE` reference
  - [ ] `$PLAN_FILE` reference
  - [ ] `$CHECKLIST_FILE` reference
  - [ ] Instructions to fix described bugs
  - [ ] Instructions to write output to `$DEBUG_FILE`
- [ ] Insert `echo "==> Starting debug session..."` status line
- [ ] Insert `echo "$DEBUG_PROMPT" | run_agent "$STAGE" || exit 1`
  - [ ] Do NOT pass `--no-interactive` (debug is conversational)
  - [ ] Do NOT use `build_prompt()` or `wait_for_user()`
- [ ] Insert `echo "==> Debug session completed"` status line

### 2. Update `TODOS.md`

- [ ] Open `TODOS.md`
- [ ] Change line 14 from `[ ]` to `[x]` for the debug stage TODO

### 3. Verification

- [ ] Run `build-feature` end-to-end
- [ ] Confirm debug stage starts after implementation completes
- [ ] Confirm `05-debug.md` is created in the doc directory
- [ ] Confirm agent accepts interactive input
- [ ] Confirm exiting agent prints `"==> Debug session completed"`
- [ ] Confirm debug stage does NOT run if implementation fails (`exit 1`)
- [ ] Test with `AGENT_TYPE=claude`
- [ ] Test with `AGENT_TYPE=cursor`
