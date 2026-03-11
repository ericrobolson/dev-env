# Implementation Checklist

## Phase 1: Create bin/helpers.sh

- [ ] Create `bin/helpers.sh` with all shared functions
  - [ ] Add bash shell validation check
  - [ ] Implement `validate_args_min()` function
  - [ ] Implement `init_globals()` function
  - [ ] Implement `run_agent()` function
  - [ ] Implement `new_doc()` function
  - [ ] Implement `wait_for_user()` function
  - [ ] Implement `build_prompt()` function
- [ ] Set permissions: `chmod 644 bin/helpers.sh`
- [ ] Verify file is NOT executable (should be sourced, not run)

## Phase 2: Refactor bin/build-feature

- [ ] Backup existing `bin/build-feature` to `bin/build-feature.bak`
- [ ] Remove duplicate function definitions from build-feature
  - [ ] Remove `TIME_STAMP` assignment
  - [ ] Remove `IDE` assignment
  - [ ] Remove `AGENT_TYPE` assignment
  - [ ] Remove `CURSOR_MODEL` assignment (fixes bug at L38-39)
  - [ ] Remove `TERSENESS` assignment
  - [ ] Remove `validate_args_min()` function
  - [ ] Remove `init_globals()` function
  - [ ] Remove `run_cursor_agent()` function
  - [ ] Remove `run_claude_agent()` function
  - [ ] Remove `new_doc()` function
  - [ ] Remove `wait_for_user()` function
  - [ ] Remove `build_prompt()` function
- [ ] Add source line for helpers.sh
  - [ ] Add SCRIPT_DIR calculation
  - [ ] Add source check with error handling
- [ ] Update function calls to use helpers
  - [ ] Replace `validate_args_min` call (update syntax)
  - [ ] Replace `init_globals` call (add error handling)
  - [ ] Replace all `run_cursor_agent`/`run_claude_agent` with `run_agent`
  - [ ] Replace `new_doc` calls
  - [ ] Replace `build_prompt` calls
  - [ ] Replace `wait_for_user` calls
- [ ] Remove AGENT_TYPE conditional at end (now handled by helpers)
- [ ] Test refactored build-feature script

## Phase 3: Refactor bin/gen-doc

- [ ] Backup existing `bin/gen-doc` to `bin/gen-doc.bak`
- [ ] Remove duplicate function definitions from gen-doc
  - [ ] Remove `TIME_STAMP` assignment
  - [ ] Remove `IDE` assignment
  - [ ] Remove `AGENT_TYPE` assignment
  - [ ] Remove `CURSOR_MODEL` assignment
  - [ ] Remove `TERSENESS` assignment
  - [ ] Remove `validate_args_min()` function
  - [ ] Remove `init_globals()` function
  - [ ] Remove `run_cursor_agent()` function
  - [ ] Remove `new_doc()` function
  - [ ] Remove `build_prompt()` function
- [ ] Add source line for helpers.sh
  - [ ] Add SCRIPT_DIR calculation
  - [ ] Add source check with error handling
- [ ] Update function calls to use helpers
  - [ ] Replace `validate_args_min` call (update syntax)
  - [ ] Replace `init_globals` call (add error handling)
  - [ ] Replace `run_cursor_agent` with `run_agent`
  - [ ] Replace `build_prompt` call
- [ ] Test refactored gen-doc script

## Phase 4: Testing

- [ ] Unit test helpers.sh functions
  - [ ] Test `validate_args_min` with sufficient args (passes)
  - [ ] Test `validate_args_min` with insufficient args (fails)
  - [ ] Test `init_globals` sets all expected variables
  - [ ] Test `init_globals` with invalid AGENT_TYPE (fails)
  - [ ] Test `new_doc` handles trailing slash correctly
  - [ ] Test `new_doc` handles no trailing slash correctly
- [ ] Integration test build-feature
  - [ ] Test with AGENT_TYPE=cursor (mock agent)
  - [ ] Verify directory structure created
  - [ ] Verify all 4 documents generated
- [ ] Integration test gen-doc
  - [ ] Test with AGENT_TYPE=cursor (mock agent)
  - [ ] Verify file created with correct naming
- [ ] Verify wait_for_user behavior
  - [ ] Non-interactive shell returns immediately
  - [ ] AGENT_TYPE=claude skips wait

## Phase 5: Verification & Cleanup

- [ ] Verify TIME_STAMP format is YYMMDDHHMM
- [ ] Verify CURSOR_MODEL can be overridden via environment
- [ ] Verify both scripts handle missing helpers.sh gracefully
- [ ] Clean up backup files after successful verification
  - [ ] Remove `bin/build-feature.bak`
  - [ ] Remove `bin/gen-doc.bak`

## Unknown / Clarification Needed

- [ ] Confirm exact test framework for unit tests (bare assertions vs bats)
- [ ] Determine if gen-doc needs `wait_for_user` (design says no, but verify)
- [ ] Verify mock agent setup for testing (create test-bin/ mock)
- [ ] Confirm audio notification mechanism (platform-specific?)
