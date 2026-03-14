# Debug Log: Prompt Logging (`00-prompts.md`)

## Issue 1: Add claude resume session ID to 00-prompts.md after each agent call

**Request:** After each `run_agent` call, find the most recent Claude session ID and append `claude --resume <session-id>` to `00-prompts.md` so users can resume the session. If no session ID is found, do nothing.

**Changes:**

### `bin/helpers.sh` — Added `append_resume` function

New function `append_resume <prompts_file>` that:
- Checks `AGENT_TYPE` is "claude" (skips for cursor)
- Derives the Claude project directory from `cwd` (`~/.claude/projects/<path-with-dashes>/`)
- Finds the most recent `.jsonl` session file by modification time
- Extracts the session UUID from the filename
- Appends a blockquote with the resume command to the prompts file
- Non-critical: silently returns 0 on any failure

### `bin/build-feature` — Added `append_resume` after all 6 stages

Added `append_resume "$PROMPTS_FILE"` after each `run_agent` call:
- Stage 1 (Design) — line 65
- Stage 2 (Research) — line 87
- Stage 3 (Plan) — line 111
- Stage 4 (Checklist) — line 126
- Stage 5 (Implementation) — line 144
- Stage 6 (Debug) — line 174

### `bin/gen-doc` — Added `append_resume` after generate stage

Added `append_resume "$PROMPTS_FILE"` after the single `run_agent` call (line 54).

### `bin/clean-room` — Added `append_resume` after stages 1 and 2

- Stage 1 (Analysis) — line 84
- Stage 2 (Compliance) — line 118
- Stage 3 (Implementation) — no `run_agent` call, so no resume ID to capture

### `tests/test-append-prompt.sh` — Added 2 new tests

- Test 7: Verifies `append_resume` appends the correct `claude --resume <uuid>` command using a fake session file
- Test 8: Verifies `append_resume` does nothing when `AGENT_TYPE` is "cursor"

**Result format in `00-prompts.md`:**
```markdown
## Design

````
<prompt text>
````

> To resume this session, run:
> `claude --resume 7f7c56bf-f51f-4124-b4b4-5f1ec9ff3fc9`

## Research
...
```

**Tests:** All 15 tests pass (13 original + 2 new).
