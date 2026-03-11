# Design: Update README.md

## Objective
Update README.md to document:
1. What this repo does
2. How to use `bin/one-shot`
3. How to use `bin/build-feature`

---

## What This Repo Does
A reproducible development environment with AI-assisted feature building tools.

**Core Tools:**
- `one-shot` - Quick, single-prompt AI task runner
- `build-feature` - Multi-stage feature development pipeline (design → research → plan → checklist → implementation)

---

## User Flows

### bin/one-shot

**Usage:** `one-shot <feature-name> <prompt>`

#### Happy Path
1. User runs `one-shot MyFeature "Do something"`
2. Script creates timestamped markdown file (e.g., `2603110850-MyFeature.md`)
3. Claude agent executes prompt
4. File opens in Cursor IDE
5. Audio notification plays

#### Unhappy Paths
| Scenario | Behavior |
|----------|----------|
| Missing arguments | Prints usage, exits 1 |
| `cursor-agent` fails | Prints error, exits 1 |
| `claude` fails | Prints error, exits 1 |

---

### bin/build-feature

**Usage:** `build-feature <feature-name> <doc-directory> <prompt>`

#### Happy Path
1. User runs `build-feature MyFeature docs "Build a widget"`
2. Creates directory: `docs/<timestamp>-MyFeature/`
3. Runs 5 stages sequentially:
   - Stage 1: `01-design.md` (product design)
   - Stage 2: `02-research.md` (deep research)
   - Stage 3: `03-plan.md` (implementation plan)
   - Stage 4: `04-checklist.md` (task breakdown)
   - Stage 5: Implementation (executes plan)
4. After each stage: opens file in Cursor, plays audio, waits for user confirmation (cursor agent type only)

#### Unhappy Paths
| Scenario | Behavior |
|----------|----------|
| Missing arguments | Prints usage, exits 1 |
| Agent fails at any stage | Prints error with stage name, exits 1 |
| User declines to continue | Keeps prompting until `y/yes` entered |

---

## Unknowns & Questions

1. **Dependencies not documented:**
   - Is `cursor-agent` a separate tool? Where does it come from?
   - Is `claude` the Anthropic CLI? What version?
   - Are there other dependencies (e.g., `cursor` CLI)?

2. **Environment variables:**
   - Are any env vars required (API keys, etc.)?

3. **Model configuration:**
   - `build-feature` uses `opus-4.5-thinking` for planning, `kimi-k2.5` for implementation
   - `one-shot` references `$MODEL` but it's never set — bug?

4. **Agent type toggle:**
   - `build-feature` has `AGENT_TYPE="cursor"` hardcoded — is this configurable?

5. **Error handling:**
   - No cleanup on failure (partial files may remain)
   - No retry mechanism

6. **Installation:**
   - Current README shows shell functions but doesn't explain prerequisites

---

## Testing Plan

### Test Cases: one-shot

| ID | Test Case | Expected Result |
|----|-----------|-----------------|
| OS-1 | Run with no args | Prints usage, exits 1 |
| OS-2 | Run with 1 arg only | Prints usage, exits 1 |
| OS-3 | Run with valid args | Creates file, runs agent, opens IDE |
| OS-4 | Agent failure | Prints error, exits 1 |

### Test Cases: build-feature

| ID | Test Case | Expected Result |
|----|-----------|-----------------|
| BF-1 | Run with no args | Prints usage, exits 1 |
| BF-2 | Run with 1 arg only | Prints usage, exits 1 |
| BF-3 | Run with 2 args only | Prints usage, exits 1 |
| BF-4 | Run with valid args | Creates directory, runs all stages |
| BF-5 | Agent fails at stage 1 | Prints error for DESIGN stage, exits 1 |
| BF-6 | Agent fails at stage 3 | Prints error for PLAN stage, exits 1 |
| BF-7 | User enters 'n' at prompt | Keeps prompting |
| BF-8 | User enters 'y' at prompt | Proceeds to next stage |

### Manual Validation
- Verify files are created in correct location
- Verify audio notification plays
- Verify Cursor IDE opens with correct file

---

## Out of Scope (for now)
- Automated testing harness
- CI/CD integration
- Configuration file support
