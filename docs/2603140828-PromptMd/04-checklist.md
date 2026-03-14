# Implementation Checklist: Prompt Logging (`00-prompts.md`)

- Plan: [03-plan.md](./03-plan.md)

---

## Step 1: Add `append_prompt` to `bin/helpers.sh`

- [ ] 1.1 Open `bin/helpers.sh`, locate `build_prompt` function (around line 136)
- [ ] 1.2 Add `append_prompt()` function after `build_prompt`
  - [ ] 1.2.1 Accept 3 args: `filepath`, `stage_name`, `prompt_text`
  - [ ] 1.2.2 Create file with `# Prompts` header if it doesn't exist
  - [ ] 1.2.3 Append `## <stage_name>` heading + 4-backtick fenced prompt text
  - [ ] 1.2.4 Use `printf '%s\n'` (not `echo`) for safe output
  - [ ] 1.2.5 Always return 0; warn to stderr on failure

---

## Step 2: Modify `bin/build-feature`

- [ ] 2.1 Define `PROMPTS_FILE="$DOC_DIRECTORY/00-prompts.md"` after `mkdir -p` (after line 40)

### Stages 1–4: Break pipe into capture + log + pipe

- [ ] 2.2 Stage 1 — Design (line 60)
  - [ ] Capture: `FINAL_PROMPT=$(build_prompt "$DESIGN_FILE" "$DESIGN_PROMPT")`
  - [ ] Log: `append_prompt "$PROMPTS_FILE" "Design" "$FINAL_PROMPT"`
  - [ ] Pipe: `echo "$FINAL_PROMPT" | run_agent "$STAGE" || exit 1`
- [ ] 2.3 Stage 2 — Research (line 80)
  - [ ] Capture: `FINAL_PROMPT=$(build_prompt "$RESEARCH_FILE" "$RESEARCH_PROMPT")`
  - [ ] Log: `append_prompt "$PROMPTS_FILE" "Research" "$FINAL_PROMPT"`
  - [ ] Pipe: `echo "$FINAL_PROMPT" | run_agent "$STAGE" || exit 1`
- [ ] 2.4 Stage 3 — Plan (line 102)
  - [ ] Capture: `FINAL_PROMPT=$(build_prompt "$PLAN_FILE" "$PLAN_PROMPT")`
  - [ ] Log: `append_prompt "$PROMPTS_FILE" "Plan" "$FINAL_PROMPT"`
  - [ ] Pipe: `echo "$FINAL_PROMPT" | run_agent "$STAGE" || exit 1`
- [ ] 2.5 Stage 4 — Checklist (line 115)
  - [ ] Capture: `FINAL_PROMPT=$(build_prompt "$CHECKLIST_FILE" "$CHECKLIST_PROMPT")`
  - [ ] Log: `append_prompt "$PROMPTS_FILE" "Checklist" "$FINAL_PROMPT"`
  - [ ] Pipe: `echo "$FINAL_PROMPT" | run_agent "$STAGE" || exit 1`

### Stage 5 — Implementation (line 131)

- [ ] 2.6 Add `append_prompt "$PROMPTS_FILE" "Implementation" "$IMPLEMENTATION_PROMPT"` before existing `echo | run_agent` line

### Stage 6 — Debug (line 160)

- [ ] 2.7 Add `append_prompt "$PROMPTS_FILE" "Debug" "$DEBUG_PROMPT"` before existing `echo | run_agent` line
- [ ] 2.8 Log initial prompt only (not retry loop prompts)

---

## Step 3: Modify `bin/gen-doc`

- [ ] 3.1 Define `PROMPTS_FILE="$DOC_DIRECTORY/00-prompts.md"` after `mkdir -p` (after line 35)
- [ ] 3.2 Break pipe at line 49
  - [ ] Capture: `FINAL_PROMPT=$(build_prompt "$FILE_PATH" "$FULL_PROMPT")`
  - [ ] Log: `append_prompt "$PROMPTS_FILE" "Generate" "$FINAL_PROMPT"`
  - [ ] Pipe: `echo "$FINAL_PROMPT" | run_agent "generate" || exit 1`

---

## Step 4: Modify `bin/clean-room`

- [ ] 4.1 Define `PROMPTS_FILE="$SPEC_DIR/00-prompts.md"` after `mkdir -p` (after line 45)
- [ ] 4.2 Stage 1 — Analysis (line 80)
  - [ ] Add `append_prompt "$PROMPTS_FILE" "Analysis" "$ANALYSIS_PROMPT"` before `echo | run_agent`
- [ ] 4.3 Stage 2 — Compliance (line 112)
  - [ ] Add `append_prompt "$PROMPTS_FILE" "Compliance" "$COMPLIANCE_PROMPT"` before `echo | run_agent`
- [ ] 4.4 Stage 3 — Implementation (line 158)
  - [ ] Add `append_prompt "$PROMPTS_FILE" "Implementation" "$IMPLEMENTATION_PROMPT"` before writing to `IMPLEMENTATION.md`

---

## Step 5: Tests

- [ ] 5.1 Create `tests/test-append-prompt.sh`
  - [ ] 5.1.1 Test: creates new file with `# Prompts` header
  - [ ] 5.1.2 Test: appends to existing file, preserves prior sections
  - [ ] 5.1.3 Test: empty prompt handled gracefully
  - [ ] 5.1.4 Test: prompt containing triple backticks uses 4-backtick fence
  - [ ] 5.1.5 Test: special characters (`$`, `\`, backticks) preserved
  - [ ] 5.1.6 Test: multiple sections with same stage name both appear
- [ ] 5.2 Add `test-append-prompt` target to `Makefile`
- [ ] 5.3 Run `make test-append-prompt`, verify all pass

---

## Step 6: Integration Verification

- [ ] 6.1 Run `make test-gen-doc` (or equivalent), verify `00-prompts.md` is created in output dir
- [ ] 6.2 Inspect generated `00-prompts.md` — confirm stages, fencing, and content are correct
- [ ] 6.3 Run `bin/build-feature` on a test feature, verify all 6 stages logged
- [ ] 6.4 Run `bin/clean-room` on a test spec, verify all 3 stages logged
