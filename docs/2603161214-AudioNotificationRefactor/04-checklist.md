# Audio Notification Refactor — Implementation Checklist

- Design: [01-design.md](01-design.md)
- Research: [02-research.md](02-research.md)
- Plan: [03-plan.md](03-plan.md)

## 1. Create `bin/play-sound`

- [x] Create `bin/play-sound` with SILENT check, afplay guard, sound file guard, and background play
- [x] `chmod +x bin/play-sound`

## 2. Remove audio text from `bin/helpers.sh`

- [x] In `build_prompt()`, remove the line: `If possible, play an audio notification to alert me that the file is ready to review.`

## 3. Remove audio text from `bin/build-feature`

- [x] Remove the audio line from `IMPLEMENTATION_PROMPT` (~line 148)

## 4. Add `play-sound` to `bin/build-feature` implementation stage

- [x] Add `bin/play-sound` after `run_agent` call (~line 152), before `append_resume`

## 5. Add `play-sound` to `bin/build-feature` overview stage

- [x] Add `bin/play-sound` after `run_agent` call (~line 215), before `append_resume`

## 6. Fix `bin/clean-room`

- [x] Remove audio text from the echo block (~lines 166-169)
- [x] Add `bin/play-sound` after the "Clean Room complete" echo

## 7. Add `play-sound` to `bin/gen-doc`

- [x] Add `bin/play-sound` after `run_agent` call (~line 53), before `append_resume`

## 8. Create `tests/test-play-sound.sh`

- [x] Create test file with all 9 test cases from plan
- [x] `chmod +x tests/test-play-sound.sh`

## 9. Verify

- [x] Run `tests/test-play-sound.sh` — all tests pass (11/11)
- [ ] Manual test: run `bin/gen-doc`, confirm sound plays

## 10. Update `TODOS.md`

- [x] Mark line 17 complete: `- [x] Split out audio playing into a script and update all prompts`
