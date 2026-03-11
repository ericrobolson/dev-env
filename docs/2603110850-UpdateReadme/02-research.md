# Research: Update README.md

## Design Reference
- [01-design.md](./01-design.md)

---

## 1. Core Scripts Analysis

### bin/one-shot

**Purpose:** Single-prompt AI task runner.

**Implementation Details:**
- Uses `claude --dangerously-skip-permissions` (Anthropic CLI)
- Creates timestamped markdown file: `YYMMDDHHMM-<feature>.md`
- Opens file in Cursor IDE
- Plays audio notification

**Bug Found:**
- Line 36: References `$MODEL` but never sets it
- Line 41: `cursor-agent --model $MODEL` would fail
- `run_cursor_agent()` is defined but never called (line 34-46)

**Actual flow:** Always uses `run_claude_agent()` (line 73)

---

### bin/build-feature

**Purpose:** Multi-stage feature development pipeline.

**5 Stages:**
1. `01-design.md` — Product design (opus-4.5-thinking)
2. `02-research.md` — Deep research (opus-4.5-thinking)
3. `03-plan.md` — Implementation plan (opus-4.5-thinking)
4. `04-checklist.md` — Task breakdown (opus-4.5-thinking)
5. Implementation — Execute plan (kimi-k2.5)

**Implementation Details:**
- Creates directory: `<doc-dir>/<YYMMDDHHMM>-<feature>/`
- `AGENT_TYPE="cursor"` hardcoded (line 40) — confusing naming
- Despite `AGENT_TYPE="cursor"`, uses `cursor-agent` via `run_claude_agent()` when AGENT_TYPE != "claude"
- Switches to `kimi-k2.5` model for implementation (line 228)

**Flow Logic (lines 69-80):**
```
if AGENT_TYPE == "claude":
    use `claude` CLI
else:
    use `cursor-agent`
```

---

## 2. External Dependencies

| Dependency | Source | Required |
|------------|--------|----------|
| `cursor-agent` | Unknown — not documented | Yes (default) |
| `claude` | Anthropic CLI (`anthropic-cli` npm package or direct binary) | Yes (alternative) |
| `cursor` | Cursor IDE CLI | Yes (opens files) |
| `afplay` / audio | macOS built-in (for notification) | Optional |

**Unknown:**
- Where `cursor-agent` comes from (not in repo, not documented)
- Specific version requirements for `claude` CLI
- Whether API keys are needed (likely `ANTHROPIC_API_KEY` env var)

---

## 3. Environment Variables

**Documented:** None

**Implied:**
| Variable | Purpose | Source |
|----------|---------|--------|
| `ANTHROPIC_API_KEY` | Claude API authentication | User must set |
| Cursor auth | cursor-agent authentication | Unknown mechanism |

---

## 4. Models Used

| Model | Used In | Stage |
|-------|---------|-------|
| `opus-4.5-thinking` | build-feature | Design, Research, Plan, Checklist |
| `kimi-k2.5` | build-feature | Implementation |
| (undefined) | one-shot | All — **BUG** |

---

## 5. Prior Art in Repo

### Existing Patterns

**Prompt Building (build-feature lines 114-131):**
```bash
build_prompt() {
    # Appends standard suffix:
    # - Unknown/unclear callout
    # - Terseness instruction
    # - Output file path
    # - IDE open command
    # - Audio notification
}
```

**User Wait Loop (build-feature lines 97-112):**
- Only triggers when `AGENT_TYPE != "claude"`
- Prompts until `y/yes` entered
- Claude mode skips (agent stays running)

---

## 6. Current README Gap Analysis

**Current README contains:**
- Brief intro
- Shell function installation for `.zshrc`/`.bashrc`

**Missing from README:**
- Prerequisites/dependencies
- What `cursor-agent` and `claude` are
- How to install dependencies
- Environment variable requirements
- Usage examples with expected output
- Explanation of multi-stage pipeline
- Model information
- Troubleshooting

---

## 7. Relevant Files

| File | Purpose |
|------|---------|
| `bin/one-shot` | Single-prompt AI runner |
| `bin/build-feature` | Multi-stage feature pipeline |
| `README.md` | Current documentation (sparse) |
| `TODOS.md` | Tracks planned improvements (agent agnosticism) |
| `Makefile` | Contains `test-build-feature` target |
| `bin/init-git-lfs.sh` | Git LFS initialization (unrelated to core tools) |
| `bin/make_executable.sh` | Utility for chmod (simple) |
| `LICENSE` | MIT License |

---

## 8. Challenges & Issues

### Critical
1. **$MODEL undefined in one-shot** — Script may fail or use empty model
2. **cursor-agent not documented** — Users can't install it
3. **No dependency installation instructions**

### Medium
4. **Hardcoded AGENT_TYPE** — Design says it's configurable, but it's not
5. **No error cleanup** — Partial files remain on failure
6. **run_cursor_agent() dead code in one-shot** — Defined but never used

### Low
7. **Inconsistent naming** — `AGENT_TYPE="cursor"` but uses `cursor-agent`, not `cursor`
8. **No retry mechanism**
9. **No configuration file support**

---

## 9. TODOS.md Alignment

Current `TODOS.md` goals:
- [ ] Agent type configurable (cursor, claude)
- [ ] Planning model configurable
- [ ] Implementation model configurable
- [ ] IDE configurable

**Status:** None implemented. All values hardcoded.

---

## 10. Unknowns

1. **cursor-agent installation** — Not documented anywhere
2. **Model availability** — Are `opus-4.5-thinking` and `kimi-k2.5` available to all users?
3. **Authentication flow** — How do users authenticate with cursor-agent?
4. **Audio notification method** — Relies on system capability, not explicit command
5. **Cross-platform support** — Scripts are bash, audio notification may be macOS-specific

---

## 11. Recommendations for README

1. Add Prerequisites section with all dependencies
2. Document cursor-agent installation (once source is known)
3. Add Environment Variables section
4. Provide usage examples with expected output
5. Explain the 5-stage pipeline visually
6. Add Troubleshooting section
7. Fix or document the $MODEL bug in one-shot
