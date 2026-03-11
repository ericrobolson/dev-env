# Design: Consolidate bin/ Script Patterns

## Overview
Abstract shared patterns from `bin/build-feature` and `bin/gen-doc` into a `bin/helpers` file.

## Current State

### Common Patterns Identified
| Pattern | build-feature | gen-doc |
|---------|---------------|---------|
| Input validation | Lines 19-22 | Lines 13-16 |
| Global vars (TIME_STAMP, IDE, AGENT_TYPE) | Lines 28-42 | Lines 24-29 |
| run_cursor_agent() | Lines 50-62 | Lines 33-44 |
| run_claude_agent() | Lines 64-82 | Lines 46-57 |
| new_doc() helper | Lines 90-94 | N/A |
| wait_for_user() | Lines 98-113 | N/A |
| build_prompt() | Lines 115-132 | N/A |
| Prompt construction pattern | Lines 140-151, 161-175, etc. | Lines 59-69 |
| Audio notification pattern | Embedded in build_prompt | Embedded in LLM_PROMPT |

### Key Differences
- build-feature: Multi-stage workflow (design → research → plan → checklist → implementation)
- gen-doc: Single-stage, simple document generation
- build-feature: Uses `cursor-agent --model $CURSOR_MODEL`, gen-doc uses bare `cursor-agent`
- build-feature has `new_doc()`, `wait_for_user()`, `build_prompt()`; gen-doc does not

## Proposed Solution

### File Structure
```
bin/
  helpers          # Shared functions library
  build-feature    # Refactored to source helpers
  gen-doc          # Refactored to source helpers
```

### Helpers API

#### Variables (sourced)
- `TIME_STAMP` - Current timestamp (YYMMDDHHMM)
- `IDE` - Target IDE (default: cursor)
- `AGENT_TYPE` - Agent type (cursor|claude)
- `CURSOR_MODEL` - Model for cursor-agent
- `TERSENESS` - Standard verbosity directive

#### Functions
```bash
validate_args_min(count, usage_msg)   # Exit if $# < count
init_globals()                        # Set TIME_STAMP, IDE, AGENT_TYPE, CURSOR_MODEL
run_agent(stage, prompt)              # Run cursor or claude agent based on AGENT_TYPE
new_doc(directory, filename)          # Returns filepath: directory/TIMESTAMP-filename
wait_for_user(filepath)               # Loop until user confirms (y/Y/yes)
build_prompt(filepath, instructions)  # Append standard suffix to prompt
```

## User Flows

### Happy Path
1. User runs `bin/build-feature` or `bin/gen-doc`
2. Script sources `bin/helpers`
3. Validation passes
4. Globals initialized
5. Agent runs successfully
6. File created and opened in IDE
7. (build-feature only) User confirms edits at each stage

### Unhappy Paths

| Trigger | Error Handling |
|---------|---------------|
| helpers file missing | Source fails → script exits with error |
| Insufficient args | validate_args_min prints usage, exits 1 |
| Agent execution fails | run_agent logs error to stderr, exits 1 |
| User aborts wait_for_user | Script continues (no exit, just logs) |
| Directory creation fails | new_doc fails via mkdir -p (handled by shell) |

## Unknowns / Questions

1. Should gen-doc adopt the `wait_for_user()` pattern from build-feature? **DECISION**: No, it doesn't need it.
2. Should gen-doc use `cursor-agent --model $CURSOR_MODEL` for consistency? **DECISION**: yes, do it.
3. Is there a preference for `bin/helpers` vs `bin/lib/helpers.sh` naming? **DECISION**: `bin/helpers.sh` is fine.
4. Should `run_agent()` support additional CLI flags (temperature, etc)? **DECISION**: No, not yet.
5. Are there other bin/ scripts that would use this library? **DECISION**: No, not yet.

## Testing Plan

### Unit Tests (helpers file)
- validate_args_min with sufficient/insufficient args
- init_globals sets expected variables
- new_doc returns correct path format
- build_prompt appends standard suffix

### Integration Tests
- build-feature complete run with mocked agent
- gen-doc complete run with mocked agent
- AGENT_TYPE=claude path verification
- AGENT_TYPE=cursor path verification

### Edge Cases
- Missing helpers file (source failure)
- Empty prompt string handling
- Document directory with trailing slash
- Non-interactive mode (skip wait_for_user)

## Test Cases

| ID | Scenario | Input | Expected |
|----|----------|-------|----------|
| TC1 | build-feature happy path | 3 valid args | Creates docs, runs 5 stages |
| TC2 | gen-doc happy path | 2 valid args | Creates single doc |
| TC3 | Missing helpers | Any script | Exit with "No such file" |
| TC4 | Insufficient args | 1 arg to build-feature | Usage message, exit 1 |
| TC5 | Agent failure | Valid args, agent exits 1 | Error logged, exit 1 |
| TC6 | AGENT_TYPE=claude | Set env var | Uses claude CLI |
| TC7 | User skip wait | AGENT_TYPE=claude | No prompt loop |
| TC8 | Trailing slash dir | "docs/" | Normalized to "docs" |
