# dev-env

A reproducible development environment with AI-assisted feature building tools.

## Core Tools

| Tool | Purpose |
|------|---------|
| `gen-doc` | Single-prompt document generator. Creates structured markdown documents in a timestamped directory. |
| `build-feature` | Multi-stage feature development pipeline |
| `clean-room` | Clean-room reverse engineering pipeline |
| `play-sound` | Standalone audio notification script (used by other tools) |

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
| `SOUND` | `/System/Library/Sounds/Glass.aiff` | Path to audio file for notifications |
| `SILENT` | (unset) | Set to `1` to suppress all audio notifications |

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

2. Add shell functions to `~/.zshrc` or `~/.bashrc` (assumes installed at `~/dev/dev-env`):

   ```bash
   build-feature() {
       ~/dev/dev-env/bin/build-feature "$@"
   }
   export -f build-feature

   clean-room() {
       ~/dev/dev-env/bin/clean-room "$@"
   }
   export -f clean-room

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

Single-prompt document generator. Creates a markdown document in a timestamped directory.

### Syntax

```bash
gen-doc <doc-name> <doc-directory> <prompt>
```

### Example

```bash
gen-doc SonataOverview docs "Write up an overview of the sonata form"
```

### Output Directory

```
<doc-directory>/YYMMDDHHMM-<doc-name>/
└── DOCUMENT.md
```

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
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌──────────────┐
│  1. Design  │───▶│ 2. Research │───▶│   3. Plan   │───▶│ 4. Checklist│───▶│ 5. Implement│───▶│  6. Debug   │───▶│ 99. Overview │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘    └──────────────┘
```

| Stage | Output File | Purpose |
|-------|-------------|---------|
| Design | `01-design.md` | Product discovery, requirements, user flows (happy and unhappy paths) |
| Research | `02-research.md` | Deep research on each element, prior art, relevant files in repo |
| Plan | `03-plan.md` | Detailed implementation plan with code snippets |
| Checklist | `04-checklist.md` | Task checklist (non-interactive) |
| Implementation | (modifies codebase) | Implement the plan (non-interactive) |
| Debug | `05-debug.md` | Interactive debug session to fix post-implementation issues |
| Overview | `99-overview.md` | Pipeline summary of all documents and code changes (non-interactive) |

### Output Directory

```
<doc-directory>/YYMMDDHHMM-<feature-name>/
├── 00-prompts.md
├── 01-design.md
├── 02-research.md
├── 03-plan.md
├── 04-checklist.md
├── 05-debug.md
└── 99-overview.md
```

### Prompt Preservation

Every prompt sent to the agent is logged to `00-prompts.md` in the feature directory. After each stage, the claude session resume command is also recorded, enabling you to resume any stage's session later with `claude --resume <session-id>`.

### Interactive Flow

After stages 1–3:
1. File opens in IDE
2. Audio notification plays
3. Script prompts: "Continue? (y/n)" to move to next stage
4. Enter `y` or `yes` to proceed

Stage 4 (Checklist) and Stage 5 (Implementation) run non-interactively. Stage 6 (Debug) starts an interactive session where you describe bugs or changes and the agent fixes them, logging all changes to `05-debug.md`. Stage 99 (Overview) runs non-interactively after debug, generating a summary of all pipeline documents and code changes including a git diff from the pipeline baseline.

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

## Usage: play-sound

Standalone audio notification script. Used internally by `build-feature` and `gen-doc`, but can also be called directly.

```bash
bin/play-sound
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SOUND` | `/System/Library/Sounds/Glass.aiff` | Path to the audio file to play |
| `SILENT` | (unset) | Set to `1` to suppress audio entirely |

The script is non-fatal — it always exits 0, even if `afplay` is not found or the sound file is missing.

---

## Skills

Skills are reusable prompt templates in `skills/<skill-name>/SKILL.md` that extend Claude Code with custom slash commands.

| Skill | Purpose |
|-------|---------|
| `f-new-skill` | Interactive wizard to design and scaffold a new Claude Code skill |
| `f-plan` | Interactive product planning wizard — walks through product description, use cases, interfaces, actions, data model, error handling, replayability, persistence, tech stack, observability, and epic/phase/task breakdown, outputting a timestamped `.plan.md` file |
| `f-build-epics` | Generates epic breakdown folders from a `.plan.md` file — one markdown file per epic with summary, guiding tenets, and per-phase implementation checklists |
| `f-grill-me` | Interview the user relentlessly about a plan or design until reaching shared understanding, resolving each branch of the decision tree |
| `f-refine-epic` | Interactively refine an existing epic file by grilling the user on design decisions, resolving ambiguities, and updating the epic with concrete decisions and an expanded checklist |

---

## Configuration

### Install Skills

Copy the `skills/` directory to another project:

```bash
make install-skills path=/Users/ericolson/dev/forge/.claude/
```

This copies `skills/` to `/Users/ericolson/dev/forge/.claude/skills/`.

---

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

Audio notifications use `afplay` and system sounds (macOS only). On non-macOS systems, the script warns and continues. Set `SILENT=1` to suppress audio entirely, or set `SOUND` to a custom audio file path.

### Agent fails mid-stage

Partial files may remain in the output directory. No automatic cleanup is performed. Delete manually if needed. All prompts are preserved in `00-prompts.md`, and session resume commands let you re-enter a failed stage.

---

## Makefile Targets

| Target | Purpose |
|--------|---------|
| `test-build-feature` | Run `build-feature` with a test prompt |
| `test-gen-doc` | Run `gen-doc` with a test prompt |
| `test-clean-room` | Run `clean-room` with a test prompt |
| `test-append-prompt` | Run prompt-logging unit tests |
| `install-skills` | Copy skills to another project (`make install-skills path=<dest>`) |
| `finalize` | Update README.md to reflect new features/changes via Claude |
| `ensure-consistency` | Verify related skills (e.g., `f-build-epics` & `f-plan`) are consistent and complete |

Test scripts are in the `tests/` directory.
