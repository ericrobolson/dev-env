# Audio Notification Refactor — Debug Log

## Issue 1: SOUND variable should be configurable via environment

**Problem:** The `SOUND` variable in `bin/play-sound` was hardcoded to `/System/Library/Sounds/Glass.aiff`. User requested it pull from an environment variable with the hardcoded path as the default.

**Fix:** Changed line 8 in `bin/play-sound` from:
```bash
SOUND="/System/Library/Sounds/Glass.aiff"
```
To:
```bash
SOUND="${SOUND:-/System/Library/Sounds/Glass.aiff}"
```

**Effect:** Users can now set `SOUND=/path/to/custom.aiff` to use a different sound file. If unset, the default Glass.aiff is used.

## Issue 2: Move `bin/play-sound` from standalone binary to function in `helpers.sh`

**Problem:** `bin/play-sound` was a standalone script. User requested it be converted to a function in `helpers.sh` and all call sites updated.

**Changes:**

1. **`bin/helpers.sh`** — Added `play_sound()` function (between `build_prompt` and `append_prompt`) containing the same logic as the former `bin/play-sound` script: SILENT check, afplay guard, sound file guard, background playback. Uses local variable for sound path with `${SOUND:-}` env override.

2. **`bin/build-feature`** (lines 151, 215) — Changed `bin/play-sound` → `play_sound` (2 call sites: after implementation stage and after overview stage).

3. **`bin/clean-room`** (line 168) — Changed `bin/play-sound` → `play_sound`.

4. **`bin/gen-doc`** (line 54) — Changed `bin/play-sound` → `play_sound`.

5. **`tests/test-play-sound.sh`** — Rewrote tests to source `helpers.sh` and test the `play_sound` function instead of invoking `bin/play-sound` as an executable. Test 1 now checks function existence via `declare -f`. Tests 2-5 invoke the function via `bash -c "source helpers.sh && play_sound"`. Tests 6-9 updated grep patterns from `play-sound` to `play_sound`.

**Note:** `bin/play-sound` is now unused and can be deleted.

## Issue 3: Sound doesn't play when `gen-doc` runs

**Problem:** After converting to a function, `play_sound` called `afplay "$sound" &` (background). Since `play_sound` is called near the end of scripts (e.g., `gen-doc` line 54), the parent shell exits immediately after, killing the background `afplay` process before it can produce audio.

**Fix:** Changed `afplay "$sound" &` to `afplay "$sound"` (foreground) in `helpers.sh:155`. The sound file (Glass.aiff) is ~0.5 seconds, so blocking is negligible — especially at the end of a long-running pipeline where the user is waiting for the notification.

**Effect:** Sound now plays reliably at all call sites.

## Issue 4: Convert `play_sound()` to return path; inject into prompts for agent execution

**Problem:** Instead of the script calling `play_sound` directly (which had timing issues), the user wants `play_sound()` in `helpers.sh` to return the absolute path to `bin/play-sound`, and that path to be injected into prompts so the AI agent runs it.

**Changes:**

1. **`bin/helpers.sh`** — Replaced the `play_sound()` function body. Instead of containing the full afplay logic, it now resolves and echoes the absolute path to `bin/play-sound` using `BASH_SOURCE`.

2. **`bin/play-sound`** — Kept as the standalone executable (unchanged from Issue 1). Still has `afplay "$SOUND" &` with background execution, which is fine since the agent's shell session stays alive.

3. **`bin/build-feature`** — Removed direct `play_sound` calls after `run_agent`. Instead:
   - Set `PLAY_SOUND=$(play_sound)` before the implementation prompt.
   - Added `"When finished, run '$PLAY_SOUND' to play an audio notification."` to both `IMPLEMENTATION_PROMPT` and `OVERVIEW_PROMPT`.

4. **`bin/clean-room`** — Removed direct `play_sound` call. Added audio instruction with `$(play_sound)` inline into `IMPLEMENTATION_PROMPT`.

5. **`bin/gen-doc`** — Removed direct `play_sound` call. Set `PLAY_SOUND=$(play_sound)` and added audio instruction to `FULL_PROMPT`.

6. **`tests/test-play-sound.sh`** — Rewritten to test both `bin/play-sound` (executable) and `play_sound()` (path helper). Tests verify the function returns the correct path, the binary works, and prompts are wired correctly.

**Effect:** The AI agent now receives the explicit path to `bin/play-sound` in its prompt and executes it directly, avoiding shell lifecycle issues.

## Issue 5: Extract `get_bin_dir()` from `play_sound()`

**Problem:** `play_sound()` inlined the `BASH_SOURCE` resolution to find the bin directory. User requested extracting this into a reusable function.

**Fix:** In `bin/helpers.sh`:
- Added `get_bin_dir()` function that returns the absolute path to the `bin/` directory via `BASH_SOURCE[0]`.
- Refactored `play_sound()` to call `get_bin_dir()` instead of resolving the path inline.

**Effect:** `get_bin_dir()` is now available for any future helpers that need to reference sibling scripts in `bin/`.
