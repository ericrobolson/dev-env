# Audio Notification Refactor — Implementation Plan

- Design: [01-design.md](01-design.md)
- Research: [02-research.md](02-research.md)

## Step 1: Create `bin/play-sound`

New file:

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

Then: `chmod +x bin/play-sound`

## Step 2: Remove audio text from `bin/helpers.sh`

In `build_prompt()` (line 135), remove the audio instruction line. Change:

```bash
    echo "$instructions

$TERSENESS

Output everything to the file '$filepath'.

Then open the file '$filepath' in the IDE '$IDE' so I can review it.

If possible, play an audio notification to alert me that the file is ready to review."
```

To:

```bash
    echo "$instructions

$TERSENESS

Output everything to the file '$filepath'.

Then open the file '$filepath' in the IDE '$IDE' so I can review it."
```

## Step 3: Remove audio text from `bin/build-feature` implementation prompt

Line 148 — remove the audio line from `IMPLEMENTATION_PROMPT`. Change:

```bash
IMPLEMENTATION_PROMPT="You are an expert at software development, project management, architecture and design.
You prefer simple, robust solutions that are easy to maintain and extend.
Make code modular wherever possible.
You are tasked to work on $FEATURE_NAME.
Implement the following plan document in '$CHECKLIST_FILE'.

If possible, play an audio notification to alert me when everything is finished."
```

To:

```bash
IMPLEMENTATION_PROMPT="You are an expert at software development, project management, architecture and design.
You prefer simple, robust solutions that are easy to maintain and extend.
Make code modular wherever possible.
You are tasked to work on $FEATURE_NAME.
Implement the following plan document in '$CHECKLIST_FILE'."
```

## Step 4: Add `play-sound` call after implementation stage in `bin/build-feature`

After line 152 (`run_agent`), add the call:

```bash
echo "$IMPLEMENTATION_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1
bin/play-sound
append_resume "$PROMPTS_FILE"
```

## Step 5: Add `play-sound` call after overview stage in `bin/build-feature`

After line 215 (`run_agent`), add the call:

```bash
echo "$FINAL_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1
bin/play-sound
append_resume "$PROMPTS_FILE"
```

## Step 6: Fix `bin/clean-room` — remove audio text, add `play-sound`

Remove the audio line from the echo (lines 166-169). Change:

```bash
echo "==> Implementation prompt written to $IMPL_PROMPT_FILE

If possible, play an audio notification to alert me when everything is finished.
"
echo "==> Clean Room complete"
```

To:

```bash
echo "==> Implementation prompt written to $IMPL_PROMPT_FILE"
echo "==> Clean Room complete"
bin/play-sound
```

## Step 7: Add `play-sound` call to `bin/gen-doc`

After line 53 (`run_agent`), add the call:

```bash
echo "$FINAL_PROMPT" | run_agent "generate" || exit 1
bin/play-sound
append_resume "$PROMPTS_FILE"
```

## Step 8: Create `tests/test-play-sound.sh`

```bash
#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$SCRIPT_DIR/../bin"

PASS=0
FAIL=0
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

assert_exit_0() {
    local label="$1"
    shift
    if "$@" >/dev/null 2>&1; then
        ((PASS++))
        echo "  PASS: $label"
    else
        ((FAIL++))
        echo "  FAIL: $label — exited non-zero"
    fi
}

assert_stderr_contains() {
    local label="$1" expected="$2"
    shift 2
    local stderr_out
    stderr_out=$("$@" 2>&1 >/dev/null)
    if echo "$stderr_out" | grep -qF "$expected"; then
        ((PASS++))
        echo "  PASS: $label"
    else
        ((FAIL++))
        echo "  FAIL: $label — expected '$expected' in stderr"
    fi
}

assert_no_output() {
    local label="$1"
    shift
    local output
    output=$("$@" 2>&1)
    if [[ -z "$output" ]]; then
        ((PASS++))
        echo "  PASS: $label"
    else
        ((FAIL++))
        echo "  FAIL: $label — expected no output, got '$output'"
    fi
}

echo "=== play-sound tests ==="

# Test 1: Script is executable
if [[ -x "$BIN_DIR/play-sound" ]]; then
    ((PASS++))
    echo "  PASS: play-sound is executable"
else
    ((FAIL++))
    echo "  FAIL: play-sound is not executable"
fi

# Test 2: Exits 0 with SILENT=1
assert_exit_0 "SILENT=1 exits 0" env SILENT=1 "$BIN_DIR/play-sound"

# Test 3: No output with SILENT=1
assert_no_output "SILENT=1 produces no output" env SILENT=1 "$BIN_DIR/play-sound"

# Test 4: Exits 0 when afplay not in PATH
assert_exit_0 "Missing afplay exits 0" env PATH="$TMPDIR" "$BIN_DIR/play-sound"

# Test 5: Warns when afplay not in PATH
assert_stderr_contains "Missing afplay warns on stderr" "afplay not found" env PATH="$TMPDIR" "$BIN_DIR/play-sound"

echo ""
echo "=== Integration: audio text removed from prompts ==="

# Test 6: build_prompt() output has no audio instruction
source "$BIN_DIR/helpers.sh"
init_globals
PROMPT_OUT=$(build_prompt "/tmp/test.md" "Test instructions")
if echo "$PROMPT_OUT" | grep -qi "audio notification"; then
    ((FAIL++))
    echo "  FAIL: build_prompt still contains audio instruction"
else
    ((PASS++))
    echo "  PASS: build_prompt has no audio instruction"
fi

# Test 7: build-feature has no audio text in IMPLEMENTATION_PROMPT
if grep -q "play an audio notification" "$BIN_DIR/build-feature"; then
    ((FAIL++))
    echo "  FAIL: build-feature still contains audio text"
else
    ((PASS++))
    echo "  PASS: build-feature has no audio text"
fi

# Test 8: clean-room has no audio text
if grep -q "play an audio notification" "$BIN_DIR/clean-room"; then
    ((FAIL++))
    echo "  FAIL: clean-room still contains audio text"
else
    ((PASS++))
    echo "  PASS: clean-room has no audio text"
fi

# Test 9: play-sound is called in pipeline scripts
for script in build-feature gen-doc clean-room; do
    if grep -q "play-sound" "$BIN_DIR/$script"; then
        ((PASS++))
        echo "  PASS: $script calls play-sound"
    else
        ((FAIL++))
        echo "  FAIL: $script does not call play-sound"
    fi
done

echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]] && exit 0 || exit 1
```

Then: `chmod +x tests/test-play-sound.sh`

## Step 9: Update `TODOS.md`

Mark line 17 complete: `- [x] Split out audio playing into a script and update all prompts`

## Execution Order

1. Create `bin/play-sound` + chmod
2. Edit `bin/helpers.sh` — remove audio line
3. Edit `bin/build-feature` — remove audio text, add `play-sound` calls
4. Edit `bin/clean-room` — remove audio text, add `play-sound` call
5. Edit `bin/gen-doc` — add `play-sound` call
6. Create `tests/test-play-sound.sh` + chmod
7. Run `tests/test-play-sound.sh` — verify all pass
8. Update `TODOS.md`
9. Manual verification: run `gen-doc`, confirm sound plays
