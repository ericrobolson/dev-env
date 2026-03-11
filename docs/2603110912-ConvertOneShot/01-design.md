# Design: Convert bin/one-shot to bin/gen-doc

## Objective
Rename `bin/one-shot` to `bin/gen-doc` and change parameter naming from `feature-name` to `doc-name`. Update all documentation accordingly.

---

## What This Feature Does
Improves naming clarity by:
1. Using `gen-doc` to indicate document generation (vs. generic "one-shot")
2. Using `doc-name` to indicate the output is a document (vs. ambiguous "feature-name")

---

## User Flows

### Happy Path: Rename Script

1. Rename file: `bin/one-shot` → `bin/gen-doc`
2. Update internal variable: `FEATURE_NAME` → `DOC_NAME`
3. Update usage message: `<feature-name>` → `<doc-name>`
4. Update LLM prompt references to `$FEATURE_NAME`

### Happy Path: Update Documentation

1. Update README.md:
   - Change `one-shot` → `gen-doc` in all references
   - Change `<feature-name>` → `<doc-name>` in syntax
   - Update shell function example
   - Update example usage command
2. Update TODOS.md:
   - Mark existing item as complete
   - Remove/update related backlog items about one-shot

---

## Unknowns & Questions

1. **Backwards compatibility:**
   - Should `one-shot` remain as a symlink or alias to `gen-doc` temporarily?
   - Or is a hard break acceptable?
   - **DECISION**: Hard break acceptable

2. **TODOS.md items:**
   - Line 1: "Convert 'one-shot' to 'gen-doc'" — this task
   - Lines 7-8: Items about updating `one-shot` for agent agnosticism — should these move to `gen-doc`? 
   - **DECISION**: Yes, update


3. **File output naming:**
   - Current: `docs/YYMMDDHHMM-<feature-name>.md`
   - Keep same pattern or change to `docs/YYMMDDHHMM-<doc-name>.md`?
   - (Pattern is already correct, just variable rename)
   - **DECISION**: Keep same pattern


4. **Git history:**
   - Use `git mv` to preserve history, or delete/add?
   - delete/add is simpler so go that way

---

## Testing Plan

### Test Cases: Script Rename

| ID | Test Case | Expected Result |
|----|-----------|-----------------|
| GD-1 | `bin/gen-doc` exists | File is executable |
| GD-2 | `bin/one-shot` does not exist | No stale file remains |
| GD-3 | Run `gen-doc` with no args | Prints usage with `<doc-name>`, exits 1 |
| GD-4 | Run `gen-doc TestDoc "prompt"` | Creates `docs/YYMMDDHHMM-TestDoc.md` |

### Test Cases: Documentation

| ID | Test Case | Expected Result |
|----|-----------|-----------------|
| DOC-1 | README mentions `gen-doc` | Found in Core Tools table |
| DOC-2 | README has `gen-doc` shell function | Code block shows correct function |
| DOC-3 | README syntax shows `<doc-name>` | Usage section updated |
| DOC-4 | No `one-shot` references remain | `grep -r "one-shot" README.md` returns empty |
| DOC-5 | TODOS.md item marked complete | Checkbox `[x]` for conversion task |

### Manual Validation
- Run `gen-doc Test "Write a test document"` end-to-end
- Verify file opens in Cursor IDE
- Verify audio notification plays
- Verify shell function in README works when copied to shell config

---

## Implementation Notes

### Files to Modify

| File | Changes |
|------|---------|
| `bin/one-shot` | Rename to `gen-doc`, update variables |
| `README.md` | Replace all `one-shot` with `gen-doc`, update syntax |
| `TODOS.md` | Mark conversion complete, update related items |

### Variables to Rename in `bin/gen-doc`

| Old | New |
|-----|-----|
| `FEATURE_NAME` | `DOC_NAME` |
| `$1` (feature name param) | `$1` (doc name param) |

### README Sections to Update

- Core Tools table (line 9)
- Installation shell function (lines 51-53)
- Usage: one-shot → Usage: gen-doc (line 63)
- Syntax: `<feature-name>` → `<doc-name>` (line 70)
- Example: `one-shot AddLogging` → `gen-doc AddLogging` (line 76)
- Output: `<feature-name>` → `<doc-name>` (line 81)
- Configuration section (line 158)
- Troubleshooting section (lines 178, 182)

---

## Out of Scope

- Adding new functionality to `gen-doc`
- Changing output file format or location
- Agent configuration improvements (separate task in TODOS.md)
- Combining `gen-doc` and `build-feature` (separate task)
