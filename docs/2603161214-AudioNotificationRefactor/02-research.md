# Audio Notification Refactor — Research

## 1. Current Audio Notification Locations

### 1.1 `bin/helpers.sh:135` — `build_prompt()`
Appends `"If possible, play an audio notification to alert me that the file is ready to review."` as the final line of every prompt built via `build_prompt()`. This means **every stage** that uses `build_prompt()` includes the audio instruction — design, research, plan, checklist, and overview stages in `build-feature`, the single stage in `gen-doc`, etc.

**Impact of removal:** All prompts built via `build_prompt()` will lose the audio instruction. Callers that need audio must call `play-sound` explicitly after `run_agent`.

### 1.2 `bin/build-feature:148` — Implementation stage
Line 148: `"If possible, play an audio notification to alert me when everything is finished."` — hardcoded directly in the `IMPLEMENTATION_PROMPT` string. This stage does **not** use `build_prompt()` (it's non-interactive, no file output), so the audio instruction was added manually.

### 1.3 `bin/clean-room:168` — Echo statement
Line 166-169: Audio instruction appears inside an `echo` statement, not in a prompt sent to an agent. This is dead code from a UX perspective — `echo` just prints to terminal, it doesn't trigger audio. It's a leftover from when the implementation stage was interactive.

## 2. Scripts That Use `build_prompt()`

| Script | Stages using `build_prompt()` | Currently gets audio via prompt |
|---|---|---|
| `bin/build-feature` | design, research, plan, checklist, overview | Yes (all 5) |
| `bin/gen-doc` | generate | Yes |
| `bin/clean-room` | None | No (uses custom prompts) |

### 2.1 `gen-doc` Flow
`gen-doc` calls `build_prompt()` once (line 51), then pipes to `run_agent` (line 53). After the refactor, `play-sound` should be called after line 53 (`run_agent "generate"`).

### 2.2 `build-feature` — Where to Add `play-sound`
Per the design decision: only after non-interactive stages and at pipeline completion.

| Stage | Line | Interactive? | Add `play-sound`? |
|---|---|---|---|
| design | 70 | Yes (`wait_for_user`) | No |
| research | 93 | Yes (`wait_for_user`) | No |
| plan | 118 | Yes (`wait_for_user`) | No |
| checklist | 134 | Yes (`wait_for_user`) | No |
| implementation | 152 | No (`--no-interactive`) | **Yes** — after line 152 |
| debug | 182 | Yes (interactive) | No |
| overview | 215 | No (`--no-interactive`) | **Yes** — after line 215 |

### 2.3 `clean-room` — Where to Add `play-sound`
The script currently ends at line 170 with an echo. All three stages (analysis, compliance, implementation) run with `--no-interactive`. Per design: add `play-sound` after the final echo at line 170.

Note: `clean-room` does not currently call `run_agent` for Stage 3 — it writes the prompt to a file and exits. The `play-sound` call goes at script end.

## 3. `afplay` on macOS

- **Location:** `/usr/bin/afplay`
- **Available on this system:** Yes
- **Default sound file:** `/System/Library/Sounds/Glass.aiff` (confirmed exists)
- **All available system sounds:** Basso, Blow, Bottle, Frog, Funk, Glass, Hero, Morse, Ping, Pop, Purr, Sosumi, Submarine, Tink (all `.aiff`)
- **Background execution:** `afplay /path/to/sound &` — works, non-blocking
- **Exit behavior:** `afplay` returns 0 on success, non-zero on failure (file not found, invalid format)

## 4. `bin/play-sound` Implementation Considerations

### 4.1 Structure
```bash
#!/bin/bash
# Play a system notification sound. Non-fatal — always exits 0.

if [[ "${SILENT:-}" == "1" ]]; then
    exit 0
fi

SOUND="/System/Library/Sounds/Glass.aiff"

if ! command -v afplay &>/dev/null; then
    echo "Warning: afplay not found, skipping audio notification" >&2
    exit 0
fi

if [[ ! -f "$SOUND" ]]; then
    echo "Warning: Sound file not found: $SOUND" >&2
    exit 0
fi

afplay "$SOUND" &
exit 0
```

### 4.2 Challenges
- **Background process:** `afplay ... &` spawns a child process. If the parent script exits immediately, the sound still plays (orphaned process is fine for short sounds).
- **CI environments:** `afplay` exists on macOS CI runners but may fail silently with no audio device. The `&` and `exit 0` pattern handles this.
- **Linux:** `afplay` is macOS-only. `aplay`, `paplay`, or `mpv` are alternatives but out of scope per design.
- **`SILENT` env var:** Simple boolean check. No validation needed — anything other than `"1"` means play sound.

### 4.3 Executable Permissions
Existing pattern: `bin/make_executable.sh` runs `chmod u+x $1`. New script needs `chmod +x` after creation.

## 5. Prompt Text Removal — Exact Strings to Remove

### 5.1 `bin/helpers.sh:133-135`
Remove the last two lines of `build_prompt()` output:
```
Then open the file '$filepath' in the IDE '$IDE' so I can review it.

If possible, play an audio notification to alert me that the file is ready to review."
```
**Only remove the audio line.** The IDE open instruction stays — it's unrelated to this refactor.

Exact string to remove from `build_prompt()`:
```

If possible, play an audio notification to alert me that the file is ready to review.
```

### 5.2 `bin/build-feature:148`
Remove:
```

If possible, play an audio notification to alert me when everything is finished.
```

### 5.3 `bin/clean-room:168`
Remove:
```

If possible, play an audio notification to alert me when everything is finished.
```

## 6. Existing Test Patterns

### 6.1 Test Structure
Tests live in `tests/` as `test-*.sh` files. Pattern:
- `set -euo pipefail`
- Source `helpers.sh`, call `init_globals`
- `PASS`/`FAIL` counters
- Helper functions: `assert_contains`, `assert_file_exists`
- `mktemp -d` with `trap` cleanup
- Exit 0 if all pass, exit 1 if any fail

### 6.2 Existing Tests
| File | Tests |
|---|---|
| `tests/test-append-prompt.sh` | 8 tests — `append_prompt` and `append_resume` |
| `tests/test-overview-stage.sh` | 4 tests — git baseline, `new_doc`, prompt exclusion |

### 6.3 Test Considerations for `play-sound`
- **Mocking `afplay`:** Override `PATH` to exclude `/usr/bin` or shadow with a no-op script
- **Mocking sound file:** Use a nonexistent path, capture stderr
- **`SILENT=1`:** Set env var, verify no output and exit 0
- **Integration tests:** Use `grep -v` on script source to verify audio text is absent from prompts

## 7. TODOS.md Reference

`TODOS.md:17`: `- [ ] Split out audio playing into a script and update all prompts` — this is the tracked item for this refactor.

## 8. Prior Art in Repo

### 8.1 `docs/2603111600-CleanRoom/05-debug.md:39-50`
Previous debug session already partially addressed this: removed audio from clean-room stage 1-3 prompts, added `afplay /System/Library/Sounds/Glass.aiff` at script end. This was later reverted or changed — current `clean-room` has the audio in an echo statement instead of an actual `afplay` call.

### 8.2 Documentation References
- `README.md:35-36`: Documents macOS audio support and Linux limitation
- `README.md:263-265`: Troubleshooting section for missing audio
- Multiple `docs/` research/plan files reference the audio pattern

## 9. Relevant Files

| File | Purpose |
|---|---|
| `bin/helpers.sh` | Contains `build_prompt()` with audio instruction (line 135). Primary edit target. |
| `bin/build-feature` | Contains hardcoded audio instruction in implementation prompt (line 148). Edit target. |
| `bin/clean-room` | Contains audio instruction in echo statement (line 168). Edit target. |
| `bin/gen-doc` | Uses `build_prompt()` (line 51). Needs `play-sound` call after `run_agent` (line 53). |
| `bin/make_executable.sh` | Pattern for making new scripts executable. |
| `tests/test-append-prompt.sh` | Test pattern to follow for new `test-play-sound.sh`. |
| `tests/test-overview-stage.sh` | Additional test pattern reference. |
| `TODOS.md` | Line 17 tracks this refactor item. Mark complete when done. |
| `README.md` | Documents audio notification behavior (lines 35-36, 263-265). May need update. |
