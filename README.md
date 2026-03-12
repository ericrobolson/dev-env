# dev-env

A reproducible development environment with AI-assisted feature building tools.

## Core Tools

| Tool | Purpose |
|------|---------|
| `gen-doc` | Single-prompt AI task runner. Used for building out markdown files. |
| `build-feature` | Multi-stage feature development pipeline |
| `clean-room` | Clean-room reverse engineering pipeline |

---

## Prerequisites

### Required

| Dependency | Description | Installation |
|------------|-------------|--------------|
| `cursor` | Cursor IDE CLI | Install [Cursor IDE](https://cursor.sh), CLI included |
| `cursor-agent` | Cursor agent CLI | Available within Cursor IDE |
| `claude` | Anthropic CLI | `npm install -g @anthropic-ai/claude-cli` or binary |

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `AGENT_TYPE` | `claude` | Agent to use: `claude` or `cursor` |
| `IDE` | `cursor` | IDE to open files: `cursor`, `vscode`, etc |
| `CURSOR_MODEL` | `kimi-k2.5` | Model for cursor-agent (when `AGENT_TYPE=cursor`) |

### Platform Support

- **macOS**: Full support (audio notifications use system sounds)
- **Linux**: Partial (audio notifications may not work)
- **Windows**: Not tested

---

## Installation

1. Clone the repo:
   ```bash
   git clone <repo-url> ~/dev/dev-env
   ```

2. Add shell functions to `~/.zshrc` or `~/.bashrc`:
(assume installed at ~/dev/dev-env for the following examples)

   ```bash
   
   build-feature() {
       ~/dev/dev-env/bin/build-feature "$@"
   }
   export -f build-feature
  ```bash

  ```bash
  gen-doc() {
      ~/dev/dev-env/bin/gen-doc "$@"
  }
  export -f gen-doc
   ```

3. Reload shell:
   ```bash
   source ~/.zshrc  # or ~/.bashrc
   ```

---

## Usage: gen-doc

Quick, single-prompt AI task runner.

### Syntax

```bash
gen-doc <doc-name> <prompt>
```

### Example

```bash
gen-doc SonataOverview "Write up an overview of the sonata form"
```

### Output

- Creates: `docs/YYMMDDHHMM-<doc-name>.md`
- Opens file in Cursor IDE
- Plays audio notification

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Missing arguments or agent failure |

---

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
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

| Stage | Output File | Purpose |
|-------|-------------|-------|
| Design | `01-design.md` | Design the feature |
| Research | `02-research.md` | Research the feature |
| Plan | `03-plan.md` | Plan the feature |
| Checklist | `04-checklist.md` | Create a checklist of tasks to complete the feature |
| Implementation | (modifies codebase) | Implement the feature |

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
1. File opens in IDE/displays file name
2. Audio notification plays
3. Script prompts: "Continue? (y/n)" to move to next stage
4. Enter `y` or `yes` to proceed

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All stages complete |
| 1 | Missing arguments or stage failure |

---

## Usage: clean-room

Clean-room reverse engineering pipeline. Analyzes a target system, produces functional specs, reviews for compliance, and generates an implementation prompt for manual execution.

### Syntax

```bash
clean-room <feature-name> <target-directory> <spec-output-directory> <implementation-directory> <test-directory> <prompt>
```

### Example

```bash
clean-room LibFoo ./vendor/libfoo ./specs ./impl ./tests "Reimplement libfoo's public API"
```

### Pipeline Stages

```
┌─────────────┐    ┌─────────────┐    ┌─────────────────────────┐
│ 1. Analysis │───▶│2. Compliance│───▶│ 3. Implementation Prep  │
└─────────────┘    └─────────────┘    └─────────────────────────┘
```

| Stage | Output | Purpose |
|-------|--------|---------|
| Analysis | `spec-*.md` | Study target, produce functional specs (non-interactive) |
| Compliance | Updates `spec-*.md` in-place | Audit specs for copyright violations (non-interactive) |
| Implementation Prep | `IMPLEMENTATION.md`, `debug.md` | Write implementation prompt with built-in debug mode for manual use |

Stages 1 and 2 run automatically via the configured agent. Stage 3 writes the implementation prompt to `IMPLEMENTATION.md` for you to execute manually. The implementation prompt includes a built-in debug mode — after implementation, the agent stays in a conversational loop where you describe bugs or changes and it fixes them, logging all changes to `debug.md`.

### Output Directory

```
<spec-output-directory>/YYMMDDHHMM-<feature-name>/
├── spec-<component-1>.md
├── spec-<component-2>.md
├── ...
├── IMPLEMENTATION.md
├── IMPLEMENTATION_NOTES.md
└── debug.md

<implementation-directory>/
└── (source files)

<test-directory>/
└── (test files)
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All stages complete |
| 1 | Missing arguments, invalid target dir, or stage failure |

---

## Configuration

### Agent Type

Set via environment variable:

```bash
export AGENT_TYPE=claude  # Uses claude CLI (default)
export AGENT_TYPE=cursor  # Uses cursor-agent
```

### IDE

Set which IDE to open generated files in:

```bash
export IDE=cursor   # Default
export IDE=vscode   # VSCode
```

### Models (cursor-agent only)

When using `AGENT_TYPE=cursor`, you can configure the model:

```bash
export CURSOR_MODEL=kimi-k2.5        # Default for implementation
export CURSOR_MODEL=opus-4.5-thinking # For planning stages
```

---

## Troubleshooting

### No audio notification

Audio notifications use system sounds. On non-macOS systems, this may fail silently.

### Agent fails mid-stage

Partial files may remain in the output directory. No automatic cleanup is performed. Delete manually if needed.
