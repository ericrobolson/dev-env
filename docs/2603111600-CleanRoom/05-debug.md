# Clean Room — Debug Log

## 2026-03-11: Stage 3 — Write prompt to file instead of executing

**Issue:** User wants Stage 3 (Clean Room Implementation) to output the implementation prompt as an `IMPLEMENTATION.md` file in the spec directory (`$SPEC_DIR`) rather than executing it via `run_agent`.

**Change:** In `bin/clean-room`, replaced the `run_agent` call in Stage 3 with:
- Write `$IMPLEMENTATION_PROMPT` to `$SPEC_DIR/IMPLEMENTATION.md` using `new_doc`
- Print confirmation message with the file path

**Before:**
```bash
echo "Building clean-room implementation..."
echo "$IMPLEMENTATION_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1
echo "==> Implementation completed"
```

**After:**
```bash
IMPL_PROMPT_FILE=$(new_doc "$SPEC_DIR" "IMPLEMENTATION.md")
echo "$IMPLEMENTATION_PROMPT" > "$IMPL_PROMPT_FILE"
echo "==> Implementation prompt written to $IMPL_PROMPT_FILE"
```

**Why:** The user wants the prompt saved as a deliverable in the spec folder rather than auto-executed, allowing manual control over when and how the implementation is run.

## 2026-03-11: Make all agent calls non-interactive

**Issue:** User wants every `run_agent` call to be non-interactive (no `wait_for_user` pauses).

**Changes in `bin/clean-room`:**

1. **Stage 1 (Analysis):** Added `--no-interactive` flag to `run_agent` and removed `wait_for_user "$SPEC_DIR"`.
2. **Stage 2 (Compliance):** Added `--no-interactive` flag to `run_agent` and removed `wait_for_user "$SPEC_DIR"`.
3. **Stage 3 (Implementation):** Already non-interactive (writes prompt to file, no `run_agent` call).

**Why:** The tool should run all stages to completion without pausing for user input, since Stage 3 now outputs a prompt file for manual execution anyway.

## 2026-03-11: Remove IDE open instructions, consolidate audio notification

**Issue:** User wants no files opened in the IDE during execution, and only a single audio notification at the very end.

**Changes in `bin/clean-room`:**

1. **Stage 1 (Analysis):** Removed "open the directory in IDE" and "play audio notification" lines from prompt.
2. **Stage 2 (Compliance):** Removed "open the directory in IDE" and "play audio notification" lines from prompt.
3. **Stage 3 (Implementation):** Removed mid-prompt "play audio notification" line.
4. **End of script:** Added `afplay /System/Library/Sounds/Glass.aiff` to play a sound only when all stages are complete.

**Why:** Stages run non-interactively now, so opening files mid-pipeline is disruptive. A single notification at the end signals the user that everything is ready for review.

## 2026-03-11: Remove clean-room and $TARGET_DIR references from IMPLEMENTATION_PROMPT

**Issue:** The implementation prompt (written to `IMPLEMENTATION.md`) should not mention clean-room concepts or `$TARGET_DIR`, since the engineer receiving this prompt should have no knowledge of the original source.

**Changes in `bin/clean-room` IMPLEMENTATION_PROMPT:**

1. Changed role from "clean-room implementation engineer" to "implementation engineer".
2. Removed "You have **never** seen the original source code."
3. Removed rule "Do NOT read, search for, or reference anything in '$TARGET_DIR'".

**Why:** The `IMPLEMENTATION.md` file is a standalone prompt given to someone who should not know a target directory exists. Mentioning it would leak awareness of the original codebase, undermining the clean-room isolation.

## 2026-03-11: Move IMPLEMENTATION_NOTES.md path to spec directory

**Issue:** `IMPLEMENTATION_NOTES.md` was being written to `$IMPL_DIR` but should go to `$SPEC_DIR` alongside the other spec/documentation files.

**Change:** Updated the IMPLEMENTATION_PROMPT line from:
- `An IMPLEMENTATION_NOTES.md noting any spec ambiguities...`
to:
- `An IMPLEMENTATION_NOTES.md in '$SPEC_DIR' noting any spec ambiguities...`

**Why:** Keeps all documentation artifacts (specs, compliance reviews, implementation notes, debug log) together in the spec directory, separate from the source code output.
