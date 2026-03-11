# Implementation Plan: Update README.md

## Reference Documents
- [01-design.md](./01-design.md)
- [02-research.md](./02-research.md)

---

## Overview

Replace current README with comprehensive documentation covering:
1. Repository purpose
2. Prerequisites and dependencies
3. Installation
4. Tool usage (`one-shot`, `build-feature`)
5. Configuration
6. Troubleshooting

---

## Implementation Steps

### Step 1: Header & Introduction

Replace intro paragraph:

```markdown
# dev-env

A reproducible development environment with AI-assisted feature building tools.

## Core Tools

| Tool | Purpose |
|------|---------|
| `one-shot` | Single-prompt AI task runner |
| `build-feature` | Multi-stage feature development pipeline |
```

---

### Step 2: Prerequisites Section

Add new section after intro:

```markdown
## Prerequisites

### Required

| Dependency | Description | Installation |
|------------|-------------|--------------|
| `cursor` | Cursor IDE CLI | Install [Cursor IDE](https://cursor.sh), CLI included |
| `cursor-agent` | Cursor agent CLI | **[UNKNOWN - source not documented]** |
| `claude` | Anthropic CLI | `npm install -g @anthropic-ai/claude-cli` or binary |

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `ANTHROPIC_API_KEY` | Yes (claude mode) | Anthropic API key |

### Platform Support

- **macOS**: Full support (audio notifications use system sounds)
- **Linux**: Partial (audio notifications may not work)
- **Windows**: Not tested
```

**UNKNOWN:** Exact installation method for `cursor-agent` is not documented in the codebase.

---

### Step 3: Installation Section

Update existing installation:

```markdown
## Installation

1. Clone the repo:
   ```bash
   git clone <repo-url> ~/dev/dev-env
   ```

2. Add shell functions to `~/.zshrc` or `~/.bashrc`:
   ```bash
   build-feature() {
       ~/dev/dev-env/bin/build-feature "$@"
   }

   one-shot() {
       ~/dev/dev-env/bin/one-shot "$@"
   }
   ```

3. Reload shell:
   ```bash
   source ~/.zshrc  # or ~/.bashrc
   ```
```

---

### Step 4: one-shot Usage Section

Add usage documentation:

```markdown
## Usage: one-shot

Quick, single-prompt AI task runner.

### Syntax

```bash
one-shot <feature-name> <prompt>
```

### Example

```bash
one-shot AddLogging "Add debug logging to all API endpoints"
```

### Output

- Creates: `docs/YYMMDDHHMM-<feature-name>.md`
- Opens file in Cursor IDE
- Plays audio notification

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Missing arguments or agent failure |
```

**KNOWN BUG:** `$MODEL` variable is referenced but never set (line 36 of `bin/one-shot`). Script uses `run_claude_agent()` directly, ignoring `run_cursor_agent()`.

---

### Step 5: build-feature Usage Section

Add detailed usage documentation:

```markdown
## Usage: build-feature

Multi-stage feature development pipeline.

### Syntax

```bash
build-feature <feature-name> <doc-directory> <prompt>
```

### Example

```bash
build-feature UserAuth docs "Build user authentication with OAuth2"
```

### Pipeline Stages

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  1. Design  │───▶│ 2. Research │───▶│   3. Plan   │───▶│ 4. Checklist│───▶│ 5. Implement│
│   (opus)    │    │   (opus)    │    │   (opus)    │    │   (opus)    │    │   (kimi)    │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

| Stage | Output File | Model |
|-------|-------------|-------|
| Design | `01-design.md` | opus-4.5-thinking |
| Research | `02-research.md` | opus-4.5-thinking |
| Plan | `03-plan.md` | opus-4.5-thinking |
| Checklist | `04-checklist.md` | opus-4.5-thinking |
| Implementation | (modifies codebase) | kimi-k2.5 |

### Output Directory

```
<doc-directory>/YYMMDDHHMM-<feature-name>/
├── 01-design.md
├── 02-research.md
├── 03-plan.md
└── 04-checklist.md
```

### Interactive Flow

After each stage (cursor agent mode):
1. File opens in Cursor IDE
2. Audio notification plays
3. Script prompts: "Continue? (y/n)"
4. Enter `y` or `yes` to proceed

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All stages complete |
| 1 | Missing arguments or stage failure |
```

---

### Step 6: Configuration Section

Add configuration documentation:

```markdown
## Configuration

### Agent Type

Currently hardcoded in `bin/build-feature` (line 40):

```bash
AGENT_TYPE="cursor"  # Uses cursor-agent
# AGENT_TYPE="claude"  # Uses claude CLI directly
```

### Models

| Stage | Model | Hardcoded Location |
|-------|-------|-------------------|
| Planning stages (1-4) | opus-4.5-thinking | build-feature line 50 |
| Implementation (5) | kimi-k2.5 | build-feature line 228 |

**Note:** Models are not yet configurable. See TODOS.md for planned improvements.
```

---

### Step 7: Troubleshooting Section

Add troubleshooting:

```markdown
## Troubleshooting

### "cursor-agent: command not found"

The `cursor-agent` CLI is required but installation is not documented.
**Workaround:** Set `AGENT_TYPE="claude"` in `bin/build-feature`.

### "claude: command not found"

Install Anthropic CLI:
```bash
npm install -g @anthropic-ai/claude-cli
```

### No audio notification

Audio notifications use system sounds. On non-macOS systems, this may fail silently.

### Agent fails mid-stage

Partial files may remain in the output directory. No automatic cleanup is performed. Delete manually if needed.
```

---

## File Diff Summary

Replace entire `README.md` with new content combining all sections above.

**Estimated line count:** ~150-180 lines (vs current 20 lines)

---

## Unknowns / Blockers

| Item | Status | Impact |
|------|--------|--------|
| `cursor-agent` installation | **Unknown** | Cannot document installation |
| `$MODEL` bug in one-shot | Known bug | Document as limitation |
| Model availability | **Unknown** | Users may not have access to opus-4.5-thinking or kimi-k2.5 |
| Authentication flow | **Unknown** | Cannot document cursor-agent auth |

---

## Validation Checklist

- [ ] Run `one-shot` with documented example, verify behavior matches docs
- [ ] Run `build-feature` with documented example, verify behavior matches docs
- [ ] Verify all file paths in examples are accurate
- [ ] Test shell function installation steps
- [ ] Review with user to confirm cursor-agent installation method
