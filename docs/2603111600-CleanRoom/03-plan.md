# Clean Room Implementation Tool — Plan

## References

- Design: [docs/2603111600-CleanRoom/01-design.md](01-design.md)
- Research: [docs/2603111600-CleanRoom/02-research.md](02-research.md)

---

## Architecture

`bin/clean-room` follows `bin/build-feature`'s multi-stage pipeline pattern. Four stages, sourcing `bin/helpers.sh` for shared infrastructure.

```
clean-room <feature-name> <target-directory> <spec-output-directory> <implementation-directory> <prompt>
    │
    ├── Stage 1: Dirty Room Analysis (interactive)
    │   └── Writes spec-*.md files to spec-output-directory/TIMESTAMP-feature-name/
    │
    ├── Stage 2: Compliance Review (interactive)
    │   └── Updates spec-*.md files in-place, appends ## Compliance Review
    │
    ├── Stage 3: Clean Room Implementation (non-interactive)
    │   └── Writes source, tests, IMPLEMENTATION_NOTES.md to implementation-directory
    │
    └── Stage 4: Debug (interactive)
        └── Writes debug.md to spec-output-directory
```

---

## Implementation

### File: `bin/clean-room`

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
validate_args_min 5 "clean-room <feature-name> <target-directory> <spec-output-directory> <implementation-directory> <prompt>" "$@" || exit 1

# Arguments
FEATURE_NAME="$1"
TARGET_DIR="${2%/}"
SPEC_DIR="${3%/}"
IMPL_DIR="${4%/}"
shift 4
PROMPT="$@"

# Initialize globals
init_globals || exit 1

# Build timestamped spec directory (like build-feature)
SPEC_DIR="$SPEC_DIR/$TIME_STAMP-$FEATURE_NAME"

# Validate target directory
if [[ ! -d "$TARGET_DIR" ]]; then
    echo "Error: Target directory does not exist: $TARGET_DIR" >&2
    exit 1
fi

if [[ -z "$(ls -A "$TARGET_DIR" 2>/dev/null)" ]]; then
    echo "Error: Target directory is empty: $TARGET_DIR" >&2
    exit 1
fi

# Create output directories
mkdir -p "$SPEC_DIR" "$IMPL_DIR"

echo "==> Clean Room ($FEATURE_NAME): target=$TARGET_DIR specs=$SPEC_DIR impl=$IMPL_DIR"
```

### Stage 1: Dirty Room Analysis

```bash
#
# Stage 1: Dirty Room Analysis
#
STAGE="analysis"
ANALYSIS_PROMPT="You are a dirty room analyst performing clean-room reverse engineering. Your job is to study the target system and produce functional specifications that describe **what** it does — never **how** the original code does it.

The target system is in: '$TARGET_DIR'

Additional context from the user:
$PROMPT

**Rules:**
- Never include, reference, or paraphrase original source code
- Describe observable behavior, inputs, outputs, interfaces, and algorithms in your own words
- Each distinct component, module, or feature gets its own markdown file
- Use the naming convention: spec-<component-name>.md

**For each component, output a markdown file in '$SPEC_DIR' containing:**
- **Name:** component identifier
- **Purpose:** what it does in one sentence
- **Inputs:** what it accepts (formats, types, ranges)
- **Outputs:** what it produces (formats, types, ranges)
- **Behavior:** step-by-step description of observable functionality
- **Interfaces:** how it interacts with other components
- **Edge cases:** known boundary conditions and expected behavior
- **Constraints:** performance, size, timing, or protocol requirements

Study the target system now and produce one spec file per component.

$TERSENESS

Then open the directory '$SPEC_DIR' in the IDE '$IDE' so I can review it.

If possible, play an audio notification to alert me that the specs are ready to review."

echo "$ANALYSIS_PROMPT" | run_agent "$STAGE" || exit 1
wait_for_user "$SPEC_DIR"
```

**Key difference from `build-feature`:** Prompt is constructed manually (not via `build_prompt()`) because this stage produces multiple files in a directory, not a single file.

### Stage 2: Compliance Review

```bash
#
# Stage 2: Compliance Review (in-place)
#
STAGE="compliance"

# Collect all spec files for the prompt
SPEC_FILES=$(ls "$SPEC_DIR"/spec-*.md 2>/dev/null)
if [[ -z "$SPEC_FILES" ]]; then
    echo "Error: No spec files found in $SPEC_DIR" >&2
    exit 1
fi

COMPLIANCE_PROMPT="You are a clean-room compliance reviewer. Your job is to audit functional specifications and fix any content that could constitute copyright infringement.

Read all spec-*.md files in '$SPEC_DIR'.

**Review each spec file for:**
- Direct copies or close paraphrases of original source code
- Variable names, function names, or identifiers lifted from the original
- Code snippets, pseudocode, or logic structures that mirror the original implementation rather than describing observable behavior
- Comments or descriptions that reveal knowledge of internal implementation details rather than external behavior
- Proprietary terminology unique to the original codebase

**For each violation found:**
- Rewrite the offending section directly in the spec file with a clean description
- Append a '## Compliance Review' section to the spec file listing: what was changed, why, and the replacement text

**If the spec is clean:** Append '## Compliance Review' followed by 'PASS — no copyrighted material detected.'

Do NOT create separate compliance files. Edit the spec files in place.

$TERSENESS

Then open the directory '$SPEC_DIR' in the IDE '$IDE' so I can review the updated specs.

If possible, play an audio notification to alert me that the compliance review is complete."

echo "$COMPLIANCE_PROMPT" | run_agent "$STAGE" || exit 1
wait_for_user "$SPEC_DIR"
```

**Key difference:** Stage 2 modifies existing files rather than creating new ones. No `new_doc()` call needed.

### Stage 3: Clean Room Implementation

```bash
#
# Stage 3: Clean Room Implementation (non-interactive)
#
STAGE="implementation"
IMPLEMENTATION_PROMPT="You are a clean-room implementation engineer. You have **never** seen the original source code. Your only input is the functional specifications provided.
You are an expert at software development, project management, architecture and design.
You prefer simple, robust solutions that are easy to maintain and extend.
Make code modular wherever possible.

Read all spec-*.md files in '$SPEC_DIR'. These are your sole source of truth.

**Rules:**
- Do NOT read, search for, or reference anything in '$TARGET_DIR'
- Use only the spec files as your source of truth
- Choose your own variable names, function names, data structures, and code organization
- Choose the simplest, most idiomatic approach for the target language
- If the spec is ambiguous, document your interpretation and make a reasonable choice
- Focus on simple, modular solutions
- You prefer simple, robust solutions that are easy to maintain and extend

**Produce in '$IMPL_DIR':**
- Source code implementing the described behavior
- Modular code wherever possible.
- Implement a simple, robust, clean, and readable solution that is easy to maintain and extend.
- Unit tests covering the inputs, outputs, behavior, and edge cases listed in the specs
- An IMPLEMENTATION_NOTES.md noting any spec ambiguities and the choices you made

**After implementation:**
- Run all tests and confirm they pass
- Verify the implementation satisfies every item in each spec's Behavior section
- Flag any spec requirements you could not fulfill and explain why

If possible, play an audio notification to alert me when everything is finished."

echo "Building clean-room implementation..."
echo "$IMPLEMENTATION_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1
echo "==> Implementation completed"
```

**Pattern match:** Mirrors `build-feature`'s Stage 5 exactly — non-interactive, no `wait_for_user()`, no `build_prompt()`.

### Stage 4: Debug

```bash
#
# Stage 4: Debug (interactive)
#
STAGE="debug"
DEBUG_FILE=$(new_doc "$SPEC_DIR" "debug.md")
echo "" >> "$DEBUG_FILE"

DEBUG_PROMPT="You are an expert at debugging and fixing software issues.

Reference these documents for full context:
- Specs: all spec-*.md files in '$SPEC_DIR'
- Implementation: all files in '$IMPL_DIR'
- Implementation notes: '$IMPL_DIR/IMPLEMENTATION_NOTES.md'

**Rules:**
- Do NOT read, search for, or reference anything in '$TARGET_DIR'
- You are in a clean-room environment — only specs and implementation are available to you

Do not run anything at this point. You are waiting on input from the user.

The user will describe bugs, issues, or changes they want to make found after implementation.
Fix each item as described.

Write all conversation output, including what was changed and why, to '$DEBUG_FILE'.
Each interaction should be recorded in '$DEBUG_FILE'."

echo "==> Starting debug session. Describe bugs to fix. Output: $DEBUG_FILE"
echo "$DEBUG_PROMPT" | run_agent "$STAGE" || exit 1
echo "==> Debug session completed"
```

---

## Complete Script

Putting it all together as a single file `bin/clean-room`:

```bash
#!/bin/bash

# Clean Room Implementation Tool
# Automates clean-room reverse engineering: analyze → compliance → implement → debug

# Source shared helpers
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$SCRIPT_DIR/helpers.sh" ]]; then
    source "$SCRIPT_DIR/helpers.sh"
else
    echo "Error: helpers.sh not found at $SCRIPT_DIR/helpers.sh" >&2
    exit 1
fi

# Input validation
validate_args_min 5 "clean-room <feature-name> <target-directory> <spec-output-directory> <implementation-directory> <prompt>" "$@" || exit 1

# Arguments
FEATURE_NAME="$1"
TARGET_DIR="${2%/}"
SPEC_DIR="${3%/}"
IMPL_DIR="${4%/}"
shift 4
PROMPT="$@"

# Initialize globals
init_globals || exit 1

# Build timestamped spec directory (like build-feature)
SPEC_DIR="$SPEC_DIR/$TIME_STAMP-$FEATURE_NAME"

# Validate target directory
if [[ ! -d "$TARGET_DIR" ]]; then
    echo "Error: Target directory does not exist: $TARGET_DIR" >&2
    exit 1
fi

if [[ -z "$(ls -A "$TARGET_DIR" 2>/dev/null)" ]]; then
    echo "Error: Target directory is empty: $TARGET_DIR" >&2
    exit 1
fi

# Create output directories
mkdir -p "$SPEC_DIR" "$IMPL_DIR"

echo "==> Clean Room ($FEATURE_NAME): target=$TARGET_DIR specs=$SPEC_DIR impl=$IMPL_DIR"

#
# Stage 1: Dirty Room Analysis
#
STAGE="analysis"
ANALYSIS_PROMPT="You are a dirty room analyst performing clean-room reverse engineering. Your job is to study the target system and produce functional specifications that describe **what** it does — never **how** the original code does it.

The target system is in: '$TARGET_DIR'

Additional context from the user:
$PROMPT

**Rules:**
- Never include, reference, or paraphrase original source code
- Describe observable behavior, inputs, outputs, interfaces, and algorithms in your own words
- Each distinct component, module, or feature gets its own markdown file
- Use the naming convention: spec-<component-name>.md

**For each component, output a markdown file in '$SPEC_DIR' containing:**
- **Name:** component identifier
- **Purpose:** what it does in one sentence
- **Inputs:** what it accepts (formats, types, ranges)
- **Outputs:** what it produces (formats, types, ranges)
- **Behavior:** step-by-step description of observable functionality
- **Interfaces:** how it interacts with other components
- **Edge cases:** known boundary conditions and expected behavior
- **Constraints:** performance, size, timing, or protocol requirements

Study the target system now and produce one spec file per component.

$TERSENESS

Then open the directory '$SPEC_DIR' in the IDE '$IDE' so I can review it.

If possible, play an audio notification to alert me that the specs are ready to review."

echo "$ANALYSIS_PROMPT" | run_agent "$STAGE" || exit 1
wait_for_user "$SPEC_DIR"

#
# Stage 2: Compliance Review (in-place)
#
STAGE="compliance"

SPEC_FILES=$(ls "$SPEC_DIR"/spec-*.md 2>/dev/null)
if [[ -z "$SPEC_FILES" ]]; then
    echo "Error: No spec files found in $SPEC_DIR" >&2
    exit 1
fi

COMPLIANCE_PROMPT="You are a clean-room compliance reviewer. Your job is to audit functional specifications and fix any content that could constitute copyright infringement.

Read all spec-*.md files in '$SPEC_DIR'.

**Review each spec file for:**
- Direct copies or close paraphrases of original source code
- Variable names, function names, or identifiers lifted from the original
- Code snippets, pseudocode, or logic structures that mirror the original implementation rather than describing observable behavior
- Comments or descriptions that reveal knowledge of internal implementation details rather than external behavior
- Proprietary terminology unique to the original codebase

**For each violation found:**
- Rewrite the offending section directly in the spec file with a clean description
- Append a '## Compliance Review' section to the spec file listing: what was changed, why, and the replacement text

**If the spec is clean:** Append '## Compliance Review' followed by 'PASS — no copyrighted material detected.'

Do NOT create separate compliance files. Edit the spec files in place.

$TERSENESS

Then open the directory '$SPEC_DIR' in the IDE '$IDE' so I can review the updated specs.

If possible, play an audio notification to alert me that the compliance review is complete."

echo "$COMPLIANCE_PROMPT" | run_agent "$STAGE" || exit 1
wait_for_user "$SPEC_DIR"

#
# Stage 3: Clean Room Implementation (non-interactive)
#
STAGE="implementation"
IMPLEMENTATION_PROMPT="You are a clean-room implementation engineer. You have **never** seen the original source code. Your only input is the functional specifications provided.
You are an expert at software development, project management, architecture and design.
You prefer simple, robust solutions that are easy to maintain and extend.
Make code modular wherever possible.

Read all spec-*.md files in '$SPEC_DIR'. These are your sole source of truth.

**Rules:**
- Do NOT read, search for, or reference anything in '$TARGET_DIR'
- Use only the spec files as your source of truth
- Choose your own variable names, function names, data structures, and code organization
- Choose the simplest, most idiomatic approach for the target language
- If the spec is ambiguous, document your interpretation and make a reasonable choice
- Focus on simple, modular solutions
- You prefer simple, robust solutions that are easy to maintain and extend

**Produce in '$IMPL_DIR':**
- Source code implementing the described behavior
- Modular code wherever possible.
- Implement a simple, robust, clean, and readable solution that is easy to maintain and extend.
- Unit tests covering the inputs, outputs, behavior, and edge cases listed in the specs
- An IMPLEMENTATION_NOTES.md noting any spec ambiguities and the choices you made

**After implementation:**
- Run all tests and confirm they pass
- Verify the implementation satisfies every item in each spec's Behavior section
- Flag any spec requirements you could not fulfill and explain why

If possible, play an audio notification to alert me when everything is finished."

echo "Building clean-room implementation..."
echo "$IMPLEMENTATION_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1
echo "==> Implementation completed"

#
# Stage 4: Debug (interactive)
#
STAGE="debug"
DEBUG_FILE=$(new_doc "$SPEC_DIR" "debug.md")
echo "" >> "$DEBUG_FILE"

DEBUG_PROMPT="You are an expert at debugging and fixing software issues.

Reference these documents for full context:
- Specs: all spec-*.md files in '$SPEC_DIR'
- Implementation: all files in '$IMPL_DIR'
- Implementation notes: '$IMPL_DIR/IMPLEMENTATION_NOTES.md'

**Rules:**
- Do NOT read, search for, or reference anything in '$TARGET_DIR'
- You are in a clean-room environment — only specs and implementation are available to you

Do not run anything at this point. You are waiting on input from the user.

The user will describe bugs, issues, or changes they want to make found after implementation.
Fix each item as described.

Write all conversation output, including what was changed and why, to '$DEBUG_FILE'.
Each interaction should be recorded in '$DEBUG_FILE'."

echo "==> Starting debug session. Describe bugs to fix. Output: $DEBUG_FILE"
echo "$DEBUG_PROMPT" | run_agent "$STAGE" || exit 1
echo "==> Debug session completed"
```

---

## Decisions Made

| Question from Design Doc | Decision |
|---|---|
| Re-run individual stages? | Not in v1. User can re-run manually by editing specs and running the full tool. Add `--stage N` later if needed. |
| Agent isolation enforcement? | Prompt-only. Matches legal standard for clean-room design. |
| Component discovery? | User-driven via the prompt argument. Agent uses prompt context to scope analysis. |
| Language target? | Inferred from user prompt and specs. User specifies in prompt if needed. |
| Existing files in output dirs? | Overwrite. User controls paths. Git provides safety net. |

## Checklist

- [ ] Create `bin/clean-room` with the complete script above
- [ ] `chmod +x bin/clean-room`
- [ ] Test: no args → prints usage, exits 1
- [ ] Test: missing target dir → error, exits 1
- [ ] Test: empty target dir → error, exits 1
- [ ] Test: full run with small target directory
- [ ] Test: `AGENT_TYPE=cursor` and `AGENT_TYPE=claude` both work
- [ ] Add `test-clean-room` target to Makefile
- [ ] Add `clean-room` usage to README.md
