# Audio Notification Refactor — Design

## Problem

Audio notifications are currently embedded as text instructions in prompts asking the AI agent to play sounds. This is unreliable — the agent may or may not execute it, and the mechanism is opaque.

Three locations today:
1. `bin/helpers.sh:135` — `build_prompt()` appends "If possible, play an audio notification..."
2. `bin/build-feature:148` — Implementation stage prompt includes audio request
3. `bin/clean-room:168` — Echo statement includes audio request

## Proposed Solution

Create `bin/play-sound` — a standalone executable that plays a system sound. Then update all scripts to call `play-sound` directly at the appropriate points instead of embedding audio requests in prompts.

## Scope

### In Scope
- New `bin/play-sound` script
- Update `helpers.sh` `build_prompt()` to remove audio instruction from prompt text
- Update `build-feature` to call `play-sound` after key stages
- Update `clean-room` to call `play-sound` after final stage
- `gen-doc` gets audio via updated `build_prompt()` flow (script calls `play-sound` after `run_agent`)

### Out of Scope
- Custom sound files or user-selectable sounds
- Volume control
- Cross-platform support beyond macOS (document limitation)

## User Flow

### Happy Path
1. User runs `build-feature`, `clean-room`, or `gen-doc`
2. Pipeline stages execute normally
3. At completion points, script calls `bin/play-sound`
4. macOS system sound plays (`afplay /System/Library/Sounds/Glass.aiff`)
5. User hears notification, knows stage is done

### Unhappy Paths

| Scenario | Behavior |
|---|---|
| `afplay` not found (Linux/CI) | `play-sound` prints warning to stderr, exits 0 (non-fatal) |
| Sound file missing | `play-sound` prints warning to stderr, exits 0 (non-fatal) |
| `play-sound` called with no arguments | Plays default sound |
| Script running in non-interactive/CI env | Still attempts playback; failure is silent and non-blocking |
| `play-sound` itself fails | Must never cause pipeline to fail — always exit 0 |

## Key Design Decisions

1. **Non-fatal**: `play-sound` must never cause a pipeline failure. Always exit 0.
2. **Background playback**: Use `afplay ... &` so sound doesn't block script execution.
3. **Remove from prompts**: Strip all "play an audio notification" text from prompts. The scripts handle it directly.
4. **Where to play**: After each `run_agent` call completes (not inside prompts).

## Unknowns & Questions

1. Should `play-sound` accept an argument to select different sounds (e.g., success vs. error)? — **DECISION: no, keep it simple for now. Single default sound.**
2. Should there be an env var to disable sound (e.g., `SILENT=1`)? — **DECISION: yes, check `SILENT` env var.**
3. Should `play-sound` be called after every stage, or only after non-interactive stages where the user is waiting? — **DECISION: only after non-interactive stages (implementation, overview) and at pipeline completion. Stick to only places where it is asked to play a sound..**
4. On `clean-room`, the audio is currently in an `echo` statement, not in a prompt — is that intentional? — **Appears to be a leftover; should be replaced with `play-sound` call.**

## Testing Plan

### Unit Tests (`tests/test-play-sound.sh`)

| # | Test Case | Expected |
|---|---|---|
| 1 | `play-sound` runs on macOS with default sound | Exits 0, sound plays |
| 2 | `play-sound` with `SILENT=1` | Exits 0, no sound plays |
| 3 | `play-sound` when `afplay` is not in PATH | Exits 0, prints warning to stderr |
| 4 | `play-sound` when sound file doesn't exist | Exits 0, prints warning to stderr |
| 5 | `play-sound` is executable | File has +x permission |

### Integration Tests

| # | Test Case | Expected |
|---|---|---|
| 6 | `build_prompt()` output does NOT contain "audio notification" text | Prompt is clean of audio instructions |
| 7 | `build-feature` implementation stage prompt does NOT contain audio text | Audio instruction removed |
| 8 | `clean-room` echo does NOT contain audio text | Audio instruction removed |
| 9 | `play-sound` is called after `run_agent` in pipeline scripts | Verify `play-sound` calls exist at correct locations |

### Manual Verification

- Run `gen-doc` end-to-end — hear sound at completion
- Run `build-feature` — hear sound after implementation and overview stages
- Run with `SILENT=1` — no sound
- Run on Linux — no crash, warning printed
