#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$SCRIPT_DIR/../bin"

PASS=0
FAIL=0
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

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
if env SILENT=1 "$BIN_DIR/play-sound" >/dev/null 2>&1; then
    ((PASS++))
    echo "  PASS: SILENT=1 exits 0"
else
    ((FAIL++))
    echo "  FAIL: SILENT=1 exits non-zero"
fi

# Test 3: No output with SILENT=1
output=$(env SILENT=1 "$BIN_DIR/play-sound" 2>&1)
if [[ -z "$output" ]]; then
    ((PASS++))
    echo "  PASS: SILENT=1 produces no output"
else
    ((FAIL++))
    echo "  FAIL: SILENT=1 produced output: '$output'"
fi

# Test 4: Exits 0 when afplay not in PATH
if env PATH="$TMPDIR" "$BIN_DIR/play-sound" >/dev/null 2>&1; then
    ((PASS++))
    echo "  PASS: Missing afplay exits 0"
else
    ((FAIL++))
    echo "  FAIL: Missing afplay exits non-zero"
fi

# Test 5: Warns when afplay not in PATH
stderr_out=$(env PATH="$TMPDIR" "$BIN_DIR/play-sound" 2>&1 >/dev/null)
if echo "$stderr_out" | grep -qF "afplay not found"; then
    ((PASS++))
    echo "  PASS: Missing afplay warns on stderr"
else
    ((FAIL++))
    echo "  FAIL: Missing afplay did not warn on stderr"
fi

echo ""
echo "=== play_sound() helper tests ==="

# Test 6: play_sound returns path to bin/play-sound
source "$BIN_DIR/helpers.sh"
init_globals
PLAY_SOUND_PATH=$(play_sound)
if [[ "$PLAY_SOUND_PATH" == "$BIN_DIR/play-sound" ]]; then
    ((PASS++))
    echo "  PASS: play_sound returns correct path"
else
    ((FAIL++))
    echo "  FAIL: play_sound returned '$PLAY_SOUND_PATH', expected '$BIN_DIR/play-sound'"
fi

echo ""
echo "=== Integration: audio text removed from prompts ==="

# Test 7: build_prompt() output has no audio instruction
PROMPT_OUT=$(build_prompt "/tmp/test.md" "Test instructions")
if echo "$PROMPT_OUT" | grep -qi "audio notification"; then
    ((FAIL++))
    echo "  FAIL: build_prompt still contains audio instruction"
else
    ((PASS++))
    echo "  PASS: build_prompt has no audio instruction"
fi

# Test 8: build-feature has no old-style audio text in prompts
if grep -q "play an audio notification" "$BIN_DIR/build-feature"; then
    ((FAIL++))
    echo "  FAIL: build-feature still contains old audio text"
else
    ((PASS++))
    echo "  PASS: build-feature has no old audio text"
fi

# Test 9: clean-room has no old-style audio text in prompts
if grep -q "play an audio notification" "$BIN_DIR/clean-room"; then
    ((FAIL++))
    echo "  FAIL: clean-room still contains old audio text"
else
    ((PASS++))
    echo "  PASS: clean-room has no old audio text"
fi

# Test 10: play-sound path is injected into pipeline script prompts
for script in build-feature gen-doc clean-room; do
    if grep -q "play_sound" "$BIN_DIR/$script"; then
        ((PASS++))
        echo "  PASS: $script uses play_sound"
    else
        ((FAIL++))
        echo "  FAIL: $script does not use play_sound"
    fi
done

echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]] && exit 0 || exit 1
