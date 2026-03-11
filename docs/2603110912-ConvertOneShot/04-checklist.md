# Implementation Checklist: Convert bin/one-shot to bin/gen-doc

## Phase 1: Create gen-doc Script

### 1.1 Create bin/gen-doc
- [ ] Copy bin/one-shot to bin/gen-doc
- [ ] Rename FEATURE_NAME variable to DOC_NAME
- [ ] Update usage message: "one-shot" → "gen-doc", "<feature-name>" → "<doc-name>"
- [ ] Update FILE_PATH construction (remove docs/ prefix since gen-doc outputs to root)
- [ ] Update LLM prompt to reference doc-name instead of feature-name
- [ ] Verify script syntax with `bash -n bin/gen-doc`

### 1.2 Make gen-doc Executable
- [ ] Run `chmod +x bin/gen-doc`
- [ ] Verify with `test -x bin/gen-doc`

### 1.3 Delete bin/one-shot
- [ ] Run `rm bin/one-shot`
- [ ] Verify with `test ! -f bin/one-shot`

## Phase 2: Update Documentation

### 2.1 Update README.md
- [ ] Line 9: Update Core Tools table entry
- [ ] Line 51-53: Update shell function definition name
- [ ] Line 63: Update Usage section header
- [ ] Line 70: Update syntax example
- [ ] Line 76: Update example command
- [ ] Line 81: Update output description path format
- [ ] Line 158: Update Configuration section reference
- [ ] Line 182: Update Troubleshooting section reference
- [ ] Verify: `grep -q "gen-doc" README.md`
- [ ] Verify no "one-shot" references remain: `! grep -q "one-shot" README.md`

### 2.2 Update TODOS.md
- [ ] Line 1: Mark conversion complete: `- [x] Convert 'one-shot' to 'gen-doc'`
- [ ] Line 7-8: Update references from one-shot to gen-doc
- [ ] Verify no "one-shot" references remain: `! grep -q "one-shot" TODOS.md`

## Phase 3: Verification

### 3.1 Script Tests
- [ ] GD-1: gen-doc exists and is executable
- [ ] GD-2: one-shot does not exist
- [ ] GD-3: Usage shows <doc-name>

### 3.2 Documentation Tests
- [ ] DOC-1: README mentions gen-doc
- [ ] DOC-4: No one-shot references remain in README.md or TODOS.md

## Post-Implementation Notes

- Users with `one-shot()` shell functions in `.zshrc`/`.bashrc` must manually update to `gen-doc()`
- Add migration note in commit message
