#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../bin/helpers.sh"
init_globals

PASS=0
FAIL=0
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

assert_contains() {
    local file="$1" expected="$2" label="$3"
    if grep -qF "$expected" "$file"; then
        ((PASS++))
        echo "  PASS: $label"
    else
        ((FAIL++))
        echo "  FAIL: $label — expected '$expected' in $file"
    fi
}

assert_file_exists() {
    local file="$1" label="$2"
    if [[ -f "$file" ]]; then
        ((PASS++))
        echo "  PASS: $label"
    else
        ((FAIL++))
        echo "  FAIL: $label — file not found: $file"
    fi
}

echo "Test 1: Creates new file with header"
F="$TMPDIR/t1.md"
append_prompt "$F" "Design" "hello world"
assert_file_exists "$F" "file created"
assert_contains "$F" "# Prompts" "has header"
assert_contains "$F" "## Design" "has stage"
assert_contains "$F" "hello world" "has content"

echo "Test 2: Appends to existing file"
append_prompt "$F" "Research" "second prompt"
assert_contains "$F" "## Design" "first section preserved"
assert_contains "$F" "## Research" "second section added"
assert_contains "$F" "second prompt" "second content"

echo "Test 3: Empty prompt"
F="$TMPDIR/t3.md"
append_prompt "$F" "Plan" ""
assert_contains "$F" "## Plan" "has stage header"

echo "Test 4: Prompt with backticks"
F="$TMPDIR/t4.md"
append_prompt "$F" "Code" 'some ```code``` here'
assert_contains "$F" '````' "4-backtick fence"
assert_contains "$F" '```code```' "backticks preserved"

echo "Test 5: Special characters"
F="$TMPDIR/t5.md"
append_prompt "$F" "Special" 'price is $100 and `cmd` with \ backslash'
assert_contains "$F" '$100' "dollar sign preserved"
assert_contains "$F" '\ backslash' "backslash preserved"

echo "Test 6: Multiple same stage name"
F="$TMPDIR/t6.md"
append_prompt "$F" "Debug" "first debug"
append_prompt "$F" "Debug" "second debug"
COUNT=$(grep -c "## Debug" "$F")
if [[ "$COUNT" -eq 2 ]]; then
    ((PASS++))
    echo "  PASS: two Debug sections"
else
    ((FAIL++))
    echo "  FAIL: expected 2 Debug sections, got $COUNT"
fi

echo "Test 7: append_resume appends resume command"
F="$TMPDIR/t7.md"
append_prompt "$F" "Design" "test prompt"
# Create a fake session file to simulate claude session
FAKE_PROJECT_DIR="$HOME/.claude/projects/$(pwd | sed 's|/|-|g')"
FAKE_SESSION="$FAKE_PROJECT_DIR/00000000-0000-0000-0000-000000000000.jsonl"
if [[ -d "$FAKE_PROJECT_DIR" ]]; then
    touch "$FAKE_SESSION"
    AGENT_TYPE="claude" append_resume "$F"
    rm -f "$FAKE_SESSION"
    assert_contains "$F" "claude --resume 00000000-0000-0000-0000-000000000000" "resume command appended"
else
    ((PASS++))
    echo "  SKIP: claude project dir not found (test only valid when claude is configured)"
fi

echo "Test 8: append_resume does nothing for cursor agent"
F="$TMPDIR/t8.md"
append_prompt "$F" "Design" "test prompt"
AGENT_TYPE="cursor" append_resume "$F"
if ! grep -q "claude --resume" "$F"; then
    ((PASS++))
    echo "  PASS: no resume command for cursor agent"
else
    ((FAIL++))
    echo "  FAIL: resume command should not appear for cursor agent"
fi

echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]] && exit 0 || exit 1
