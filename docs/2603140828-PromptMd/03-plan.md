# Implementation Plan: Prompt Logging (`00-prompts.md`)

- Design: [01-design.md](./01-design.md)
- Research: [02-research.md](./02-research.md)

---

## Step 1: Add `append_prompt` to `bin/helpers.sh`

Add after the `build_prompt` function (line 136):

```bash
# append_prompt: Log a prompt to a markdown file
# Usage: append_prompt <filepath> <stage_name> <prompt_text>
# Non-critical — warns on failure, never exits
append_prompt() {
    local filepath="$1"
    local stage_name="$2"
    local prompt_text="$3"

    # Create file with header if it doesn't exist
    if [[ ! -f "$filepath" ]]; then
        printf '# Prompts\n\n' > "$filepath" 2>/dev/null || {
            echo "Warning: Could not create prompt log: $filepath" >&2
            return 0
        }
    fi

    # Append stage section with 4-backtick fence
    {
        printf '## %s\n\n' "$stage_name"
        printf '````\n'
        printf '%s\n' "$prompt_text"
        printf '````\n\n'
    } >> "$filepath" 2>/dev/null || {
        echo "Warning: Could not write to prompt log: $filepath" >&2
    }

    return 0
}
```

Key points:
- Uses `printf '%s\n'` (not `echo`) to avoid shell interpretation of `-n`, `-e`, etc.
- 4-backtick fences to handle prompts containing triple backticks.
- Always returns 0 — prompt logging is non-critical.
- Warns to stderr on failure.

---

## Step 2: Modify `bin/build-feature`

### 2a. Define `PROMPTS_FILE` (after line 40, after `mkdir -p`)

```bash
PROMPTS_FILE="$DOC_DIRECTORY/00-prompts.md"
```

### 2b. Stages 1-4: Break pipe into capture + log + pipe

Each stage currently does:
```bash
build_prompt "$FILE" "$PROMPT_VAR" | run_agent "$STAGE" || exit 1
```

Change to:
```bash
FINAL_PROMPT=$(build_prompt "$FILE" "$PROMPT_VAR")
append_prompt "$PROMPTS_FILE" "StageName" "$FINAL_PROMPT"
echo "$FINAL_PROMPT" | run_agent "$STAGE" || exit 1
```

Concrete changes per stage:

**Stage 1 — Design** (line 60):
```bash
# Before:
build_prompt "$DESIGN_FILE" "$DESIGN_PROMPT" | run_agent "$STAGE" || exit 1

# After:
FINAL_PROMPT=$(build_prompt "$DESIGN_FILE" "$DESIGN_PROMPT")
append_prompt "$PROMPTS_FILE" "Design" "$FINAL_PROMPT"
echo "$FINAL_PROMPT" | run_agent "$STAGE" || exit 1
```

**Stage 2 — Research** (line 80):
```bash
FINAL_PROMPT=$(build_prompt "$RESEARCH_FILE" "$RESEARCH_PROMPT")
append_prompt "$PROMPTS_FILE" "Research" "$FINAL_PROMPT"
echo "$FINAL_PROMPT" | run_agent "$STAGE" || exit 1
```

**Stage 3 — Plan** (line 102):
```bash
FINAL_PROMPT=$(build_prompt "$PLAN_FILE" "$PLAN_PROMPT")
append_prompt "$PROMPTS_FILE" "Plan" "$FINAL_PROMPT"
echo "$FINAL_PROMPT" | run_agent "$STAGE" || exit 1
```

**Stage 4 — Checklist** (line 115):
```bash
FINAL_PROMPT=$(build_prompt "$CHECKLIST_FILE" "$CHECKLIST_PROMPT")
append_prompt "$PROMPTS_FILE" "Checklist" "$FINAL_PROMPT"
echo "$FINAL_PROMPT" | run_agent "$STAGE" || exit 1
```

### 2c. Stage 5 — Implementation (line 131)

No `build_prompt` used. Add `append_prompt` before existing line:

```bash
append_prompt "$PROMPTS_FILE" "Implementation" "$IMPLEMENTATION_PROMPT"
echo "$IMPLEMENTATION_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1
```

### 2d. Stage 6 — Debug (line 160)

Log initial prompt only (per design decision #4):

```bash
append_prompt "$PROMPTS_FILE" "Debug" "$DEBUG_PROMPT"
echo "$DEBUG_PROMPT" | run_agent "$STAGE" || exit 1
```

---

## Step 3: Modify `bin/gen-doc`

### 3a. Define `PROMPTS_FILE` (after line 35, after `mkdir -p`)

```bash
PROMPTS_FILE="$DOC_DIRECTORY/00-prompts.md"
```

### 3b. Break pipe (line 49)

```bash
# Before:
 build_prompt "$FILE_PATH" "$FULL_PROMPT" | run_agent "generate" || exit 1

# After:
FINAL_PROMPT=$(build_prompt "$FILE_PATH" "$FULL_PROMPT")
append_prompt "$PROMPTS_FILE" "Generate" "$FINAL_PROMPT"
echo "$FINAL_PROMPT" | run_agent "generate" || exit 1
```

---

## Step 4: Modify `bin/clean-room`

### 4a. Define `PROMPTS_FILE` (after line 45, after `mkdir -p`)

```bash
PROMPTS_FILE="$SPEC_DIR/00-prompts.md"
```

### 4b. Stage 1 — Analysis (line 80)

```bash
append_prompt "$PROMPTS_FILE" "Analysis" "$ANALYSIS_PROMPT"
echo "$ANALYSIS_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1
```

### 4c. Stage 2 — Compliance (line 112)

```bash
append_prompt "$PROMPTS_FILE" "Compliance" "$COMPLIANCE_PROMPT"
echo "$COMPLIANCE_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1
```

### 4d. Stage 3 — Implementation (line 158)

No `run_agent` call — prompt is written to `IMPLEMENTATION.md`. Still log for consistency:

```bash
append_prompt "$PROMPTS_FILE" "Implementation" "$IMPLEMENTATION_PROMPT"
echo "$IMPLEMENTATION_PROMPT" > "$IMPL_PROMPT_FILE"
```

---

## Step 5: Tests

Create `tests/test-append-prompt.sh`:

```bash
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

echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]] && exit 0 || exit 1
```

Add to `Makefile`:

```makefile
test-append-prompt:
	bash tests/test-append-prompt.sh
```

---

## Checklist

- [ ] Add `append_prompt` function to `bin/helpers.sh`
- [ ] Add `PROMPTS_FILE` variable to `bin/build-feature`
- [ ] Refactor stages 1-4 in `build-feature` (break pipe pattern)
- [ ] Add `append_prompt` call before stage 5 in `build-feature`
- [ ] Add `append_prompt` call before stage 6 in `build-feature`
- [ ] Add `PROMPTS_FILE` variable to `bin/gen-doc`
- [ ] Refactor `gen-doc` stage (break pipe pattern)
- [ ] Add `PROMPTS_FILE` variable to `bin/clean-room`
- [ ] Add `append_prompt` call before stage 1 in `clean-room`
- [ ] Add `append_prompt` call before stage 2 in `clean-room`
- [ ] Add `append_prompt` call before stage 3 in `clean-room`
- [ ] Create `tests/test-append-prompt.sh`
- [ ] Add `test-append-prompt` target to `Makefile`
- [ ] Run tests, verify all pass
- [ ] Manual integration test: run `make test-gen-doc`, verify `00-prompts.md` created
