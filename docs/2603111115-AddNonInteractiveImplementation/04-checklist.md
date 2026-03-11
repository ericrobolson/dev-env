# Implementation Checklist: Non-Interactive Mode for `run_agent`

Reference: [03-plan.md](./03-plan.md)

---

## 1. Modify `run_agent()` in `bin/helpers.sh`

- [ ] Add `local interactive=true` variable after `local stage="$1"`
- [ ] Add `if [[ "$2" == "--no-interactive" ]]; then interactive=false; fi` block
- [ ] Replace inline `claude --dangerously-skip-permissions` with array-based command building:
  - [ ] `local cmd=(claude --dangerously-skip-permissions)`
  - [ ] Conditionally append `--print`: `if [[ "$interactive" == false ]]; then cmd+=(--print); fi`
  - [ ] Update invocation to `echo "$prompt" | "${cmd[@]}"`
- [ ] Replace inline `cursor-agent --model "$CURSOR_MODEL"` with array-based command building:
  - [ ] `local cmd=(cursor-agent --model "$CURSOR_MODEL")`
  - [ ] Conditionally append `--print`: `if [[ "$interactive" == false ]]; then cmd+=(--print); fi`
  - [ ] Update invocation to `echo "$prompt" | "${cmd[@]}"`

## 2. Update `bin/build-feature` Stage 5

- [ ] Change `echo "$IMPLEMENTATION_PROMPT" | run_agent "$STAGE"` to `echo "$IMPLEMENTATION_PROMPT" | run_agent "$STAGE" --no-interactive`

## 3. Verify No Changes Needed

- [ ] Confirm `bin/gen-doc` calls to `run_agent` have no second argument (interactive by default)

## 4. Testing

- [ ] Run `build-feature` — verify stages 1-4 behave identically (interactive)
- [ ] Run `build-feature` — verify stage 5 runs with `--print` (non-interactive, no prompts)
- [ ] Test with `AGENT_TYPE=claude`
- [ ] Test with `AGENT_TYPE=cursor`
- [ ] Kill agent mid-stage-5 — verify script exits with error message
- [ ] Smoke test: `AGENT_TYPE=claude bash -x bin/build-feature test-feature docs "test prompt" 2>&1 | grep -- '--print'`
