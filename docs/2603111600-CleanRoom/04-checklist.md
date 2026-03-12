# Clean Room Implementation Tool — Checklist

## References

- Design: [01-design.md](01-design.md)
- Research: [02-research.md](02-research.md)
- Plan: [03-plan.md](03-plan.md)

---

## Implementation Checklist

### 1. Create `bin/clean-room` script

- [x] Create file `bin/clean-room`
- [x] Add shebang and header comment
- [x] Source `helpers.sh` with existence check
- [x] `chmod +x bin/clean-room`

### 2. Argument parsing & validation

- [x] Call `validate_args_min 5` with usage string
- [x] Parse positional args: `FEATURE_NAME`, `TARGET_DIR`, `SPEC_DIR`, `IMPL_DIR`, `PROMPT`
- [x] Strip trailing slashes from directory args
- [x] Call `init_globals`
- [x] Build timestamped spec directory: `$SPEC_DIR/$TIME_STAMP-$FEATURE_NAME`
- [x] Validate `TARGET_DIR` exists
- [x] Validate `TARGET_DIR` is not empty
- [x] `mkdir -p` for `SPEC_DIR` and `IMPL_DIR`
- [x] Print summary line with all paths

### 3. Stage 1 — Dirty Room Analysis

- [x] Set `STAGE="analysis"`
- [x] Construct `ANALYSIS_PROMPT` with:
  - [x] Dirty-room analyst role and rules
  - [x] `TARGET_DIR` path
  - [x] User `PROMPT` context
  - [x] `spec-<component>.md` naming convention
  - [x] Required spec schema (Name, Purpose, Inputs, Outputs, Behavior, Interfaces, Edge cases, Constraints)
  - [x] `$TERSENESS` variable
  - [x] IDE open and audio notification instructions
- [x] Pipe prompt to `run_agent "$STAGE"`
- [x] Call `wait_for_user "$SPEC_DIR"`

### 4. Stage 2 — Compliance Review

- [x] Set `STAGE="compliance"`
- [x] Check that `spec-*.md` files exist in `$SPEC_DIR`; exit 1 if none found
- [x] Construct `COMPLIANCE_PROMPT` with:
  - [x] Compliance reviewer role
  - [x] Review criteria (copied code, lifted identifiers, paraphrased logic, proprietary terms)
  - [x] In-place edit instructions (rewrite violations, append `## Compliance Review`)
  - [x] PASS directive for clean specs
  - [x] `$TERSENESS`, IDE open, audio notification
- [x] Pipe prompt to `run_agent "$STAGE"`
- [x] Call `wait_for_user "$SPEC_DIR"`

### 5. Stage 3 — Clean Room Implementation

- [x] Set `STAGE="implementation"`
- [x] Construct `IMPLEMENTATION_PROMPT` with:
  - [x] Clean-room engineer role (has never seen original code)
  - [x] Isolation rule: do NOT read/reference `$TARGET_DIR`
  - [x] Spec files as sole source of truth
  - [x] Own naming/structure choices
  - [x] Output requirements: source code, unit tests, `IMPLEMENTATION_NOTES.md` in `$IMPL_DIR`
  - [x] Post-implementation: run tests, verify spec coverage, flag gaps
  - [x] Audio notification
- [x] Pipe prompt to `run_agent "$STAGE" --no-interactive`
- [x] Print completion message

### 6. Stage 4 — Debug

- [x] Set `STAGE="debug"`
- [x] Create `debug.md` via `new_doc "$SPEC_DIR" "debug.md"`
- [x] Construct `DEBUG_PROMPT` with:
  - [x] Debugger role
  - [x] References to specs, implementation, and `IMPLEMENTATION_NOTES.md`
  - [x] Isolation rule: do NOT read/reference `$TARGET_DIR`
  - [x] Wait-for-user-input directive
  - [x] Log all changes to `$DEBUG_FILE`
- [x] Pipe prompt to `run_agent "$STAGE"`
- [x] Print completion message

---

## Testing Checklist

### Input validation tests

- [x] No args → prints usage, exits 1
- [x] 4 args (missing prompt) → prints usage, exits 1
- [x] Non-existent target dir → error, exits 1
- [x] Empty target dir → error, exits 1
- [x] Non-existent spec dir parent → creates via `mkdir -p`
- [x] Non-existent impl dir parent → creates via `mkdir -p`

### Stage tests

- [ ] Stage 1: generates ≥1 `spec-*.md` in spec dir
- [ ] Stage 1: each spec has all required sections
- [ ] Stage 2: clean spec → `PASS` in `## Compliance Review`
- [ ] Stage 2: dirty spec → violations flagged and rewritten
- [ ] Stage 3: impl dir contains source, tests, `IMPLEMENTATION_NOTES.md`
- [ ] Stage 3: all tests pass
- [ ] Stage 4: debug session starts interactively
- [ ] Stage 4: changes logged to `debug.md`

### End-to-end tests

- [ ] Full run on small target directory completes all 4 stages
- [ ] `AGENT_TYPE=cursor` works
- [ ] `AGENT_TYPE=claude` works

### Integration

- [x] Add `test-clean-room` target to `Makefile`
- [x] Add `clean-room` usage to `README.md`
