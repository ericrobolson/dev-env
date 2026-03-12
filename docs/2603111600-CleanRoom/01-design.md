# Clean Room Implementation Tool — Design Document

## Overview

`bin/clean-room` is a CLI tool that automates the clean-room reverse engineering process described in `docs/2603111550-CleanRoomImplementation.md`. It uses an AI agent to: (1) analyze a target system and produce functional specs, (2) audit specs for compliance, (3) implement code from specs alone, and (4) enter an interactive debug session for post-implementation fixes.

## Usage

```
clean-room <target-directory> <spec-output-directory> <implementation-directory> <prompt>
```

| Argument | Description |
|---|---|
| `target-directory` | Path to the codebase to reverse-engineer |
| `spec-output-directory` | Where `spec-*.md` files are written |
| `implementation-directory` | Where clean-room source code is written |
| `prompt` | Additional context/instructions for the agent |

## Stages

### Stage 1: Dirty Room Analysis
- Agent reads `target-directory` and generates `spec-<component>.md` files into `spec-output-directory`.
- Uses the dirty-room analyst prompt from the checklist doc.
- Each component gets its own spec file following the defined schema (Name, Purpose, Inputs, Outputs, Behavior, Interfaces, Edge cases, Constraints).
- **Wait for user review** before proceeding.

### Stage 2: Compliance Review
- Agent reviews each `spec-*.md` file for copyright violations.
- Uses the compliance reviewer prompt from the checklist doc.
- Outputs a `compliance-<component>.md` report per spec.
- If any spec fails: flags violations and suggests replacements. User must approve before continuing.
- If spec passes: outputs `PASS`.
- **Wait for user review** before proceeding.

### Stage 3: Clean Room Implementation
- Agent implements code from specs only. It must NOT read `target-directory`.
- Uses the implementation engineer prompt from the checklist doc.
- Outputs source code, unit tests, and `IMPLEMENTATION_NOTES.md` into `implementation-directory`.
- Runs tests and verifies spec coverage.
- **Non-interactive** (runs to completion like build-feature's implementation stage).

### Stage 4: Debug
- Interactive session where the user describes bugs, issues, or changes found after implementation.
- Agent has full context: all spec files, compliance reports, and implementation output.
- Agent must NOT read `target-directory` (same isolation rule as stage 3).
- Creates `debug.md` in `spec-output-directory` to log all changes and reasoning.
- User drives the conversation; agent fixes issues as described.
- Session ends when user exits.

## User Flow

### Happy Path
1. User runs `clean-room ./vendor/libfoo ./specs ./impl "Reimplement libfoo's public API"`
2. Stage 1 generates spec files → user reviews in IDE → confirms
3. Stage 2 audits specs → all pass → user confirms
4. Stage 3 implements from specs → tests pass → audio notification
5. Stage 4 debug session starts → user describes issues → agent fixes them → changes logged to `debug.md`
6. User has clean-room implementation in `./impl`

### Unhappy Paths

| Scenario | Behavior |
|---|---|
| Target directory doesn't exist | Exit with error message |
| Spec or impl directory not writable | Exit with error message |
| No prompt provided | Exit with usage message |
| Agent fails mid-stage | Exit with error, partial output preserved |
| Compliance review finds violations | Violations flagged in report; user reviews and can re-run stage 1 with fixes (manual for now) |
| User rejects specs at review step | User can edit specs manually, then re-run from stage 2 |
| Tests fail in stage 3 | Agent reports failures; user addresses in stage 4 debug session |
| Debug session agent reads target dir | Violation — prompt enforces isolation, but no filesystem guard |
| Target directory is empty | Agent reports nothing to analyze; exit gracefully |
| Spec files reference original code identifiers | Caught in stage 2 compliance review |

## Unknowns / Questions

1. **Re-run individual stages?** Should the tool support `--stage 2` to re-run from a specific stage? (e.g., after manually editing specs)
2. **Agent isolation in stage 3** — How do we enforce that the agent does NOT read `target-directory` during implementation? Prompt-only enforcement or filesystem-level? (Prompt-only is the current approach in the checklist doc.)
3. **Component discovery** — Stage 1 relies on the agent to identify components. Should the user be able to provide a component list?
4. **Language target** — The implementation prompt says "target language." Should the user specify this, or is it inferred from the specs/prompt?
5. **Existing files in output dirs** — Should we error, overwrite, or merge if spec/impl dirs already contain files?
6. ~~**Debug stage** — Should there be a stage 4 debug session like `build-feature` has?~~ **Resolved: Yes, added as Stage 4.**

## Testing Plan

### Unit Tests (helpers/arg validation)
| # | Case | Expected |
|---|---|---|
| 1 | No arguments | Prints usage, exits 1 |
| 2 | Missing prompt (only 3 args) | Prints usage, exits 1 |
| 3 | Target dir doesn't exist | Error message, exits 1 |
| 4 | Non-existent spec output dir parent | Creates dir (mkdir -p) |
| 5 | Non-existent impl dir parent | Creates dir (mkdir -p) |

### Integration Tests (stage execution)
| # | Case | Expected |
|---|---|---|
| 6 | Stage 1 with valid target | Generates ≥1 `spec-*.md` file in spec dir |
| 7 | Stage 1 spec file schema | Each spec has all required sections (Name, Purpose, Inputs, Outputs, Behavior, Interfaces, Edge cases, Constraints) |
| 8 | Stage 2 on clean spec | Outputs `PASS` in compliance report |
| 9 | Stage 2 on dirty spec (contains code) | Flags violations with suggestions |
| 10 | Stage 3 produces source + tests | Implementation dir contains source files, test files, and `IMPLEMENTATION_NOTES.md` |
| 11 | Stage 3 tests pass | Agent confirms all tests pass |

### Debug Tests (stage 4)
| # | Case | Expected |
|---|---|---|
| 12 | Debug session starts after implementation | Agent enters interactive mode with full context |
| 13 | User describes a bug | Agent fixes it and logs change to `debug.md` |
| 14 | Debug session `debug.md` logging | Each interaction recorded with what changed and why |
| 15 | Debug agent does not reference target dir | Prompt enforces clean-room isolation |

### End-to-End Tests
| # | Case | Expected |
|---|---|---|
| 16 | Full run on small known target | All 4 stages complete, implementation matches spec behavior |
| 17 | Full run with agent failure mid-stage | Exits with error, partial output intact |
| 18 | Full run with `AGENT_TYPE=cursor` | Works with cursor-agent |
| 19 | Full run with `AGENT_TYPE=claude` | Works with claude agent, skips wait_for_user |

### Manual Verification
- Review generated specs for quality and completeness
- Confirm no original code leaks into specs or implementation
- Verify audio notification fires on completion
