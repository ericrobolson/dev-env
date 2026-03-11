#!/bin/bash
# Unit tests for bin/helpers.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HELPERS_PATH="$SCRIPT_DIR/helpers.sh"

echo "=== Testing helpers.sh ==="
echo ""

# Test 1: helpers.sh exists and is readable
echo "Test 1: helpers.sh exists and is readable"
if [[ -r "$HELPERS_PATH" ]]; then
    echo "  ✓ helpers.sh is readable"
else
    echo "  ✗ helpers.sh is not readable"
    exit 1
fi

# Test 2: Source helpers.sh
echo "Test 2: Source helpers.sh"
if source "$HELPERS_PATH"; then
    echo "  ✓ helpers.sh sourced successfully"
else
    echo "  ✗ Failed to source helpers.sh"
    exit 1
fi

# Test 3: validate_args_min with sufficient args
echo "Test 3: validate_args_min with sufficient args"
if validate_args_min 2 "usage msg" arg1 arg2; then
    echo "  ✓ validate_args_min passes with 2 args (required 2)"
else
    echo "  ✗ validate_args_min should pass with sufficient args"
    exit 1
fi

# Test 4: validate_args_min with insufficient args
echo "Test 4: validate_args_min with insufficient args"
if ! validate_args_min 3 "usage msg" arg1 arg2 2>/dev/null; then
    echo "  ✓ validate_args_min fails with insufficient args"
else
    echo "  ✗ validate_args_min should fail with insufficient args"
    exit 1
fi

# Test 5: init_globals sets all expected variables
echo "Test 5: init_globals sets expected variables"
init_globals
if [[ -n "$TIME_STAMP" ]]; then
    echo "  ✓ TIME_STAMP is set: $TIME_STAMP"
else
    echo "  ✗ TIME_STAMP is not set"
    exit 1
fi

if [[ "$IDE" == "cursor" ]]; then
    echo "  ✓ IDE defaults to 'cursor'"
else
    echo "  ✗ IDE should default to 'cursor', got: $IDE"
    exit 1
fi

if [[ "$AGENT_TYPE" == "cursor" ]]; then
    echo "  ✓ AGENT_TYPE defaults to 'cursor'"
else
    echo "  ✗ AGENT_TYPE should default to 'cursor', got: $AGENT_TYPE"
    exit 1
fi

if [[ -n "$CURSOR_MODEL" ]]; then
    echo "  ✓ CURSOR_MODEL is set: $CURSOR_MODEL"
else
    echo "  ✗ CURSOR_MODEL is not set"
    exit 1
fi

if [[ -n "$TERSENESS" ]]; then
    echo "  ✓ TERSENESS is set"
else
    echo "  ✗ TERSENESS is not set"
    exit 1
fi

# Test 6: init_globals with invalid AGENT_TYPE
echo "Test 6: init_globals with invalid AGENT_TYPE"
AGENT_TYPE="invalid"
if ! init_globals 2>/dev/null; then
    echo "  ✓ init_globals fails with invalid AGENT_TYPE"
else
    echo "  ✗ init_globals should fail with invalid AGENT_TYPE"
    exit 1
fi
AGENT_TYPE="cursor"  # Reset

# Test 7: new_doc handles trailing slash
echo "Test 7: new_doc handles trailing slash"
result=$(new_doc "docs/test/" "file.md")
if [[ "$result" == "docs/test/file.md" ]]; then
    echo "  ✓ new_doc handles trailing slash: $result"
else
    echo "  ✗ new_doc should normalize trailing slash, got: $result"
    exit 1
fi

# Test 8: new_doc handles no trailing slash
echo "Test 8: new_doc handles no trailing slash"
result=$(new_doc "docs/test" "file.md")
if [[ "$result" == "docs/test/file.md" ]]; then
    echo "  ✓ new_doc works without trailing slash: $result"
else
    echo "  ✗ new_doc should work without trailing slash, got: $result"
    exit 1
fi

# Test 9: build_prompt includes TERSENESS
echo "Test 9: build_prompt includes TERSENESS"
init_globals
prompt=$(build_prompt "test.md" "Do something")
if [[ "$prompt" == *"$TERSENESS"* ]]; then
    echo "  ✓ build_prompt includes TERSENESS"
else
    echo "  ✗ build_prompt should include TERSENESS"
    exit 1
fi

# Test 10: build_prompt includes filepath
echo "Test 10: build_prompt includes filepath"
if [[ "$prompt" == *"test.md"* ]]; then
    echo "  ✓ build_prompt includes filepath"
else
    echo "  ✗ build_prompt should include filepath"
    exit 1
fi

# Test 11: TIME_STAMP format (YYMMDDHHMM)
echo "Test 11: TIME_STAMP format (YYMMDDHHMM)"
init_globals
if [[ "$TIME_STAMP" =~ ^[0-9]{10}$ ]]; then
    echo "  ✓ TIME_STAMP is 10 digits: $TIME_STAMP"
else
    echo "  ✗ TIME_STAMP should be 10 digits (YYMMDDHHMM), got: $TIME_STAMP"
    exit 1
fi

# Test 12: CURSOR_MODEL can be overridden via environment
echo "Test 12: CURSOR_MODEL can be overridden via environment"
CURSOR_MODEL="custom-model"
init_globals
if [[ "$CURSOR_MODEL" == "custom-model" ]]; then
    echo "  ✓ CURSOR_MODEL respects environment variable"
else
    echo "  ✗ CURSOR_MODEL should respect environment variable, got: $CURSOR_MODEL"
    exit 1
fi

echo ""
echo "=== All tests passed! ==="
