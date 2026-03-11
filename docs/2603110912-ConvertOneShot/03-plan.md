# Implementation Plan: Convert bin/one-shot to bin/gen-doc

## References

- [01-design.md](./01-design.md) - Design document with user flows and decisions
- [02-research.md](./02-research.md) - Research document with current state analysis

---

## Phase 1: Create gen-doc Script

### Step 1.1: Create bin/gen-doc

Create new file `bin/gen-doc` by copying and modifying `bin/one-shot`.

**Key changes:**
- Rename `FEATURE_NAME` → `DOC_NAME`
- Update usage message: `one-shot` → `gen-doc`, `<feature-name>` → `<doc-name>`
- Update LLM prompt references

**Code snippet for bin/gen-doc:**

```bash
#! /bin/bash

# This script will create a new document for a project.

# General flow is
# - Get doc name + document directory from user
# - Create a markdown file with the name and directory path
# - Work through the prompt and store everything in a markdown file.

#
# Validate input
#
if [[ -z "$1" || -z "$2" ]]; then
  echo "Usage: gen-doc <doc-name> <prompt>"
  exit 1
fi

#
# Build out global variables
#
DOC_NAME="$1"
PROMPT="${@:2}"

TIME_STAMP=$(date +%y%m%d%H%M)
FILE_PATH="$TIME_STAMP-$DOC_NAME.md"

echo "==> File path: $FILE_PATH"
IDE="cursor"
AGENT_TYPE="cursor"  # Set to "claude" to use claude CLI directly

# Run an agent using Cursor
# Uses stdin to pass the prompt to the agent.
run_cursor_agent() {
  echo "==> Running cursor-agent for stage: $STAGE"

  local STDIN_PROMPT=$(cat) 
  echo "$STDIN_PROMPT"  

  if ! cursor-agent "$STDIN_PROMPT"; then
    echo "ERROR: cursor-agent failed at stage: $STAGE" >&2
    exit 1
  fi
  echo "==> Stage: $STAGE completed"
}

run_claude_agent() {
  echo "==> Running claude for stage: $STAGE"

  local STDIN_PROMPT=$(cat) 
  echo "$STDIN_PROMPT"  

  if ! claude --dangerously-skip-permissions "$STDIN_PROMPT"; then
    echo "ERROR: claude failed at stage: $STAGE" >&2
    exit 1
  fi
  echo "==> Stage: $STAGE completed"
}

LLM_PROMPT="You prefer simple, robust solutions that are easy to maintain and extend.
You are tasked to work on $DOC_NAME.

$PROMPT

Output everything to the file '$FILE_PATH'.

Open the file '$FILE_PATH' in the IDE '$IDE' so I can review it.

If possible, play an audio notification to alert me that the file is ready to review.
"

if [[ "$AGENT_TYPE" == "claude" ]]; then
  echo "$LLM_PROMPT" | run_claude_agent
else
  echo "$LLM_PROMPT" | run_cursor_agent
fi
```

### Step 1.2: Make gen-doc Executable

```bash
chmod +x bin/gen-doc
```

### Step 1.3: Delete bin/one-shot

```bash
rm bin/one-shot
```

---

## Phase 2: Update Documentation

### Step 2.1: Update README.md

**Line 9: Core Tools table**

```markdown
| `gen-doc` | Single-prompt AI task runner. Used for building out markdown files. |
```

**Line 51-53: Shell function definition**

```bash
gen-doc() {
    ~/dev/dev-env/bin/gen-doc "$@"
}
```

**Line 63: Usage section header**

```markdown
## Usage: gen-doc
```

**Line 70: Syntax**

```bash
gen-doc <doc-name> <prompt>
```

**Line 76: Example command**

```bash
gen-doc AddLogging "Add debug logging to all API endpoints"
```

**Line 81: Output description**

```markdown
- Creates: `docs/YYMMDDHHMM-<doc-name>.md`
```

**Line 158: Configuration section**

```markdown
Currently hardcoded in `bin/build-feature` (line 40) and `bin/gen-doc` (line 33):
```

**Line 182: Troubleshooting section**

```markdown
**Workaround:** Set `AGENT_TYPE="claude"` in `bin/build-feature` or `bin/gen-doc` to use the claude CLI directly.
```

### Step 2.2: Update TODOS.md

**Line 1: Mark conversion complete**

```markdown
- [x] Convert 'one-shot' to 'gen-doc'
```

**Line 7-8: Update references to gen-doc**

```markdown
- [ ] Update `gen-doc` to follow patterns in `build-feature` for agent agnosticism
- - [ ] Maybe even combine functionality into a shared file that gets imported into both `gen-doc` and `build-feature`
```

---

## Phase 3: Verification

### Test Commands

```bash
# GD-1: gen-doc exists and is executable
test -x bin/gen-doc && echo "PASS: gen-doc is executable"

# GD-2: one-shot does not exist
test ! -f bin/one-shot && echo "PASS: one-shot removed"

# GD-3: Usage shows <doc-name>
bin/gen-doc 2>&1 | grep -q "<doc-name>" && echo "PASS: Usage shows <doc-name>"

# DOC-1: README mentions gen-doc
grep -q "gen-doc" README.md && echo "PASS: README mentions gen-doc"

# DOC-4: No one-shot references remain
! grep -q "one-shot" README.md TODOS.md && echo "PASS: No one-shot references"
```

---

## Unknowns / Clarifications Needed

1. **Documentation timing**: Update README before or after script rename?
   - **Decision**: Update README after script rename to avoid broken references during transition

2. **Shell function migration**: Users with `one-shot()` in `.zshrc`/`.bashrc` must manually update
   - **Action**: Add note in commit message or changelog

---

# Implementation Checklist

See [04-checklist.md](./04-checklist.md) for the generated checklist.
