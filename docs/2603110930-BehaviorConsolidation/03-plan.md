# Implementation Plan: Consolidate bin/ Script Patterns

## References
- [Design Document](./01-design.md)
- [Research Document](./02-research.md)

---

## Deliverables

| File | Action | Purpose |
|------|--------|---------|
| `bin/helpers.sh` | Create | Shared library with common functions |
| `bin/build-feature` | Refactor | Use helpers, fix bugs |
| `bin/gen-doc` | Refactor | Use helpers, add --model flag |

---

## Phase 1: Create helpers.sh

### File: bin/helpers.sh

```bash
#!/bin/bash
# Shared helper functions for bin/ scripts

# Exit if sourced from non-bash shell
if [ -z "$BASH_VERSION" ]; then
    echo "Error: helpers.sh must be sourced from bash" >&2
    return 1
fi

# validate_args_min: Exit if $# < count
# Usage: validate_args_min <count> <usage_message>
validate_args_min() {
    local min_count="$1"
    local usage_msg="$2"
    shift 2

    if [[ $# -lt $min_count ]]; then
        echo "Usage: $usage_msg" >&2
        return 1
    fi
}

# init_globals: Set shared global variables
# Sets: TIME_STAMP, IDE, AGENT_TYPE, CURSOR_MODEL, TERSENESS
init_globals() {
    TIME_STAMP=$(date +%y%m%d%H%M)
    IDE="${IDE:-cursor}"
    AGENT_TYPE="${AGENT_TYPE:-cursor}"
    CURSOR_MODEL="${CURSOR_MODEL:-kimi-k2.5}"
    TERSENESS="Be concise and to the point. Stick to facts. Be succinct and terse. Don't be verbose."

    # Validate AGENT_TYPE
    if [[ "$AGENT_TYPE" != "cursor" && "$AGENT_TYPE" != "claude" ]]; then
        echo "Error: AGENT_TYPE must be 'cursor' or 'claude', got: $AGENT_TYPE" >&2
        return 1
    fi
}

# run_agent: Execute cursor or claude agent based on AGENT_TYPE
# Usage: echo "prompt" | run_agent <stage_name>
run_agent() {
    local stage="$1"
    local prompt
    prompt=$(cat)

    if [[ -z "$prompt" ]]; then
        echo "Error: No prompt provided to run_agent" >&2
        return 1
    fi

    echo "$prompt"

    if [[ "$AGENT_TYPE" == "claude" ]]; then
        if ! echo "$prompt" | claude --dangerously-skip-permissions; then
            echo "Error: claude agent failed at stage '$stage'" >&2
            return 1
        fi
    else
        if ! echo "$prompt" | cursor-agent --model "$CURSOR_MODEL"; then
            echo "Error: cursor-agent failed at stage '$stage'" >&2
            return 1
        fi
    fi

    echo "✓ Stage '$stage' complete"
}

# new_doc: Return filepath for new document
# Usage: filepath=$(new_doc <directory> <filename>)
new_doc() {
    local dir="${1%/}"  # Remove trailing slash
    local filename="$2"
    echo "$dir/$filename"
}

# wait_for_user: Loop until user confirms (skip if AGENT_TYPE=claude or non-interactive)
# Usage: wait_for_user <filepath>
wait_for_user() {
    local filepath="$1"

    # Skip for claude agent or non-interactive shells
    if [[ "$AGENT_TYPE" == "claude" ]] || [[ ! -t 0 ]]; then
        return 0
    fi

    echo ""
    echo "Review: $filepath"
    echo "Open file in IDE to review. Continue? (y/n)"

    while true; do
        read -r response
        case "$response" in
            [yY]|[yY][eE][sS])
                return 0
                ;;
            [nN]|[nN][oO])
                echo "Continuing without confirmation..."
                return 0
                ;;
            *)
                echo "Please enter 'y' or 'n':"
                ;;
        esac
    done
}

# build_prompt: Append standard suffix to prompt
# Usage: prompt=$(build_prompt <filepath> <instructions>...)
build_prompt() {
    local filepath="$1"
    shift
    local instructions="$*"

    echo "$instructions

$TERSENESS

Output everything to the file '$filepath'.

Then open the file '$filepath' in the IDE '$IDE' so I can review it.

If possible, play an audio notification to alert me that the file is ready to review."
}
```

### Setup

```bash
chmod 644 bin/helpers.sh  # Not executable (sourced)
```

---

## Phase 2: Refactor bin/build-feature

### Changes Summary
- Source helpers.sh
- Remove duplicate function definitions
- Remove CURSOR_MODEL override bug (line 39)
- Replace run_cursor_agent/run_claude_agent with run_agent
- Fix AGENT_TYPE conditional at end

### File: bin/build-feature (refactored)

```bash
#!/bin/bash

# Source shared helpers
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$SCRIPT_DIR/helpers.sh" ]]; then
    source "$SCRIPT_DIR/helpers.sh"
else
    echo "Error: helpers.sh not found at $SCRIPT_DIR/helpers.sh" >&2
    exit 1
fi

# Input validation
validate_args_min 3 "build-feature <feature-name> <doc-directory> <prompt>" "$@" || exit 1

# Arguments
FEATURE_NAME="$1"
DOC_DIRECTORY="${2%/}/$TIME_STAMP-$FEATURE_NAME"
shift 2
PROMPT="$*"

# Initialize globals
init_globals || exit 1

# Create directory
mkdir -p "$DOC_DIRECTORY"

# Stage 1: Design
DESIGN_FILE=$(new_doc "$DOC_DIRECTORY" "01-design.md")
STAGE1_PROMPT=$(build_prompt "$DESIGN_FILE" "Write a design document for the feature '$FEATURE_NAME'.

User's requirements: $PROMPT

Include:
- Overview
- Goals
- Non-goals
- Proposed solution
- Alternative solutions considered")
echo "$STAGE1_PROMPT" | run_agent "design" || exit 1
wait_for_user "$DESIGN_FILE"

# Stage 2: Research
RESEARCH_FILE=$(new_doc "$DOC_DIRECTORY" "02-research.md")
STAGE2_PROMPT=$(build_prompt "$RESEARCH_FILE" "Research existing solutions for '$FEATURE_NAME'.

Reference the design at: $DESIGN_FILE

Include:
- Prior art
- Relevant documentation
- Technical constraints")
echo "$STAGE2_PROMPT" | run_agent "research" || exit 1
wait_for_user "$RESEARCH_FILE"

# Stage 3: Plan
PLAN_FILE=$(new_doc "$DOC_DIRECTORY" "03-plan.md")
STAGE3_PROMPT=$(build_prompt "$PLAN_FILE" "Create an implementation plan for '$FEATURE_NAME'.

Reference:
- Design: $DESIGN_FILE
- Research: $RESEARCH_FILE

Include code snippets and file structure.")
echo "$STAGE3_PROMPT" | run_agent "plan" || exit 1
wait_for_user "$PLAN_FILE"

# Stage 4: Checklist
CHECKLIST_FILE=$(new_doc "$DOC_DIRECTORY" "04-checklist.md")
STAGE4_PROMPT=$(build_prompt "$CHECKLIST_FILE" "Create a task checklist for '$FEATURE_NAME'.

Reference the plan at: $PLAN_FILE

Use this format:
- [ ] Task 1
- [ ] Task 2")
echo "$STAGE4_PROMPT" | run_agent "checklist" || exit 1
wait_for_user "$CHECKLIST_FILE"

# Stage 5: Implement
echo ""
echo "All documents created in: $DOC_DIRECTORY"
echo "Review checklist at: $CHECKLIST_FILE"
echo "Begin implementation."
```

---

## Phase 3: Refactor bin/gen-doc

### Changes Summary
- Source helpers.sh
- Remove duplicate function definitions
- Add --model flag to cursor-agent call
- Use run_agent() abstraction

### File: bin/gen-doc (refactored)

```bash
#!/bin/bash

# Source shared helpers
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$SCRIPT_DIR/helpers.sh" ]]; then
    source "$SCRIPT_DIR/helpers.sh"
else
    echo "Error: helpers.sh not found at $SCRIPT_DIR/helpers.sh" >&2
    exit 1
fi

# Input validation
validate_args_min 2 "gen-doc <doc-name> <prompt>" "$@" || exit 1

# Arguments
DOC_NAME="$1"
shift
PROMPT="$*"

# Initialize globals
init_globals || exit 1

# Build output path
FILE_PATH="$TIME_STAMP-$DOC_NAME.md"

# Build and execute prompt
FULL_PROMPT=$(build_prompt "$FILE_PATH" "$PROMPT")
echo "$FULL_PROMPT" | run_agent "generate" || exit 1
```

---

## Testing Plan

### Unit Tests for helpers.sh

```bash
#!/bin/bash
# Test helpers.sh functions

source bin/helpers.sh

# Test validate_args_min
validate_args_min 2 "usage" arg1 arg2 || exit 1
! validate_args_min 3 "usage" arg1 arg2 2>/dev/null || exit 1

# Test init_globals
init_globals
[[ -n "$TIME_STAMP" ]] || exit 1
[[ "$IDE" == "cursor" ]] || exit 1
[[ "$AGENT_TYPE" == "cursor" ]] || exit 1
[[ -n "$CURSOR_MODEL" ]] || exit 1
[[ -n "$TERSENESS" ]] || exit 1

# Test new_doc
result=$(new_doc "docs/test" "file.md")
[[ "$result" == "docs/test/file.md" ]] || exit 1
result=$(new_doc "docs/test/" "file.md")
[[ "$result" == "docs/test/file.md" ]] || exit 1

echo "All tests passed"
```

### Integration Tests

```bash
#!/bin/bash
# Integration test for refactored scripts

# Test build-feature (mock agent)
export AGENT_TYPE=cursor
export PATH="./test-bin:$PATH"

# Test gen-doc
echo "Testing gen-doc..."
./bin/gen-doc test-doc "Create test document"
[[ -f *-test-doc.md ]] || exit 1

echo "Integration tests passed"
```

### Test Matrix

| ID | Test | Expected |
|----|------|----------|
| T1 | `source bin/helpers.sh` | Success, no errors |
| T2 | `validate_args_min 2 "msg" one` | Returns 1, prints usage |
| T3 | `init_globals; echo $TIME_STAMP` | YYMMDDHHMM format |
| T4 | `AGENT_TYPE=foo init_globals` | Returns 1, error message |
| T5 | `new_doc "dir/" "file"` | "dir/file" (no double slash) |
| T6 | `AGENT_TYPE=cursor run_agent test <<< "prompt"` | Uses cursor-agent --model |
| T7 | `AGENT_TYPE=claude run_agent test <<< "prompt"` | Uses claude CLI |
| T8 | `build-feature a b c` with mocked agent | Creates docs/260311...-a/ |
| T9 | `gen-doc name "prompt"` with mocked agent | Creates 260311...-name.md |
| T10 | `wait_for_user` non-interactive | Returns immediately |

---

## Migration Steps

1. **Backup existing scripts**
   ```bash
   cp bin/build-feature bin/build-feature.bak
   cp bin/gen-doc bin/gen-doc.bak
   ```

2. **Create helpers.sh**
   - Write file content
   - Set permissions: `chmod 644 bin/helpers.sh`

3. **Refactor build-feature**
   - Remove duplicate functions
   - Add source line
   - Fix CURSOR_MODEL bug
   - Update agent calls

4. **Refactor gen-doc**
   - Remove duplicate functions
   - Add source line
   - Update agent calls

5. **Test both scripts**
   - Run with AGENT_TYPE=cursor
   - Run with AGENT_TYPE=claude (if available)
   - Verify output files created

6. **Clean up backups** (after verification)
   ```bash
   rm bin/build-feature.bak bin/gen-doc.bak
   ```

---

## Bugs Fixed

| Bug | Location | Fix |
|-----|----------|-----|
| CURSOR_MODEL overwritten | build-feature L38-39 | Remove duplicate assignment |
| run_claude_agent confusing fallback | build-feature L64-82 | Single run_agent function |
| gen-doc missing --model flag | gen-doc L37 | Add --model in helpers |
| AGENT_TYPE validation | Both scripts | Add validation in init_globals |

---

## Open Questions (Resolved in Design)

1. **File naming**: Use `bin/helpers.sh` (not `bin/helpers` or `bin/lib/helpers.sh`)
2. **gen-doc wait_for_user**: Not needed, keep simple
3. **CURSOR_MODEL env var**: Support via environment variable
4. **AGENT_TYPE validation**: Validate in init_globals()

---

## Verification Checklist

- [ ] `bin/helpers.sh` created and sourced successfully
- [ ] `bin/build-feature` runs without errors
- [ ] `bin/gen-doc` runs without errors
- [ ] `AGENT_TYPE=cursor` uses cursor-agent --model
- [ ] `AGENT_TYPE=claude` uses claude CLI
- [ ] Both scripts create files in expected format
- [ ] `TIME_STAMP` format is YYMMDDHHMM
- [ ] Trailing slash in directory handled correctly
- [ ] Audio notification requested in prompts
- [ ] Files open in IDE after creation

---

## Generated Checklist

See `04-checklist.md` for the full implementation checklist.
