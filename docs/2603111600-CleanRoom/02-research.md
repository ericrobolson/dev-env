# Clean Room Implementation Tool — Research

## 1. Stage-by-Stage Analysis

### Stage 1: Dirty Room Analysis

**What it does:** Agent reads target codebase, produces `spec-<component>.md` files describing observable behavior.

**Challenges:**
- Component discovery is agent-dependent. No deterministic way to ensure all components are found.
- Agent may inadvertently copy identifiers, variable names, or structure from the target.
- Spec schema enforcement (Name, Purpose, Inputs, Outputs, Behavior, Interfaces, Edge cases, Constraints) relies on prompt compliance — no validation.
- Large codebases may exceed agent context windows. No chunking strategy defined.
- The agent must understand the *entire* target system to produce complete specs. Partial reads lead to incomplete specs.

**Open question from design doc:** Should the user be able to provide a component list? Currently agent-driven only. **DECISION** user specified. This is a part of the prompt they provide.

### Stage 2: Compliance Review (In-Place)

**What it does:** Agent reviews each `spec-*.md` for copyright violations and **updates the spec files directly** with fixes. No separate compliance output files. The agent reads each spec, identifies violations, rewrites offending sections in place, and appends a `## Compliance Review` section to each spec documenting what was changed and why. If nothing was changed, appends `## Compliance Review\nPASS — no copyrighted material detected.`

**Why in-place instead of separate files:**
- Eliminates file proliferation (`compliance-*.md` files that duplicate spec content).
- Fixes are applied immediately — no manual edit-and-re-run cycle.
- The spec files become the single source of truth (spec + compliance status in one file).
- User reviews the updated specs directly, not a separate report.

**Challenges:**
- Compliance checking is inherently subjective — what constitutes "close paraphrase" vs. independent description?
- No ground-truth comparison. Agent reviews spec in isolation without comparing against original (by design, to maintain separation).
- False positives may frustrate users. False negatives defeat the purpose.
- Proprietary terminology detection requires domain knowledge the agent may lack.
- In-place edits are destructive — original spec text is lost unless user has version control. Mitigated by git or by the user copying the spec dir before running stage 2.
- Agent must preserve the spec schema (Name, Purpose, Inputs, Outputs, etc.) while rewriting only the offending content.

**Process:**
1. Agent reads all `spec-*.md` files in `spec-output-directory`.
2. For each spec, identifies violations (copied identifiers, paraphrased code, proprietary terms).
3. Rewrites offending sections directly in the spec file with clean descriptions.
4. Appends `## Compliance Review` section listing: what was changed, why, and the replacement text.
5. If no violations found, appends `PASS` to the compliance section.
6. User reviews updated specs. If satisfied, proceeds to stage 3.

### Stage 3: Clean Room Implementation

**What it does:** Agent implements code from specs only. Must NOT read target directory.

**Challenges:**
- **Isolation enforcement is prompt-only.** No filesystem guard prevents the agent from reading `target-directory`. The agent (claude/cursor) has full filesystem access via `--dangerously-skip-permissions`.
- Language target is unspecified — inferred from prompt or specs. Could lead to mismatches.
- Spec ambiguity handling is left to the agent. `IMPLEMENTATION_NOTES.md` documents choices but user has no review gate before code is written.
- Test execution depends on the target language's toolchain being available.
- This stage runs non-interactively (`--no-interactive`), matching `build-feature`'s implementation stage pattern.

**Open question:** Should we validate that no files from `target-directory` were read during this stage? Agent logs could be parsed, but this is fragile.

### Stage 4: Debug

**What it does:** Interactive session for post-implementation fixes. Agent has specs (with inline compliance reviews) + implementation context.

**Challenges:**
- Same isolation concern as Stage 3 — agent must not read target directory, enforced only by prompt.
- `debug.md` logging depends on agent compliance. No structured format defined.
- Session state is ephemeral — if agent crashes, debug context is lost (no checkpoint/resume).
- User must manually describe bugs. No automated diff or test failure integration.

---

## 2. Prior Art in This Repo

### `bin/build-feature` — Direct Ancestor
The clean-room tool mirrors `build-feature`'s architecture almost exactly:

| Aspect | `build-feature` | `clean-room` (proposed) |
|--------|-----------------|------------------------|
| Stages | 6 (design → research → plan → checklist → implement → debug) | 4 (analysis → compliance → implement → debug) |
| Interactive stages | design, research, plan, checklist, debug | analysis, compliance, debug |
| Non-interactive | implementation | implementation |
| Output files | `01-design.md` through `05-debug.md` | `spec-*.md` (with inline compliance sections), `IMPLEMENTATION_NOTES.md`, `debug.md` |
| User review gates | `wait_for_user()` after each interactive stage | Same pattern needed |
| Agent execution | `run_agent()` with stage name | Same |
| Prompt construction | `build_prompt()` | Same |

**Key difference:** `clean-room` stage 1 produces *multiple* output files (one per component), while `build-feature` has exactly one file per stage. Stage 2 modifies those same spec files in place rather than creating new files.

### `bin/gen-doc` — Single-Stage Pattern
Useful as reference for the simplest agent invocation. Shows minimal viable prompt construction.

### `bin/helpers.sh` — Shared Infrastructure
All functions needed by `clean-room` already exist:

| Function | Usage in clean-room |
|----------|-------------------|
| `validate_args_min()` | Validate 4 required args |
| `init_globals()` | Set `TIME_STAMP`, `IDE`, `AGENT_TYPE`, etc. |
| `run_agent()` | Execute each stage (interactive or `--no-interactive`) |
| `new_doc()` | Generate file paths for spec/compliance/debug files |
| `wait_for_user()` | Pause after stages 1 and 2 |
| `build_prompt()` | Construct prompts with standard suffix (IDE open, audio, terseness) |

**Gap:** `build_prompt()` appends a single output file path. Stage 1 and 2 produce *multiple* files (one per component). The prompt must instruct the agent to write multiple files, but `build_prompt()` can only reference one path. Options:
1. Pass the *directory* instead of a file path to `build_prompt()`.
2. Skip `build_prompt()` for multi-file stages and construct the prompt manually (like `build-feature`'s implementation stage does).
3. Extend `build_prompt()` to accept a directory.

### `docs/2603111550-CleanRoomImplementation.md` — Prompt Templates
Contains the three core prompts (dirty-room analyst, compliance reviewer, implementation engineer). These are the exact prompts to embed in `bin/clean-room`.

### `.tmp/CLEAN_ROOM.md` and `.tmp/CHINESE_ROOM.md` — Legal Background
Wikipedia references on clean-room design legal precedent (IBM BIOS/Phoenix, NEC v. Intel, Sony v. Connectix) and information barrier protocols.

---

## 3. Argument Handling & Directory Structure

**Proposed CLI signature:**
```
clean-room <target-directory> <spec-output-directory> <implementation-directory> <prompt>
```

**Comparison with existing tools:**

| Tool | Args | Directory creation |
|------|------|--------------------|
| `build-feature` | 3 (name, doc-dir, prompt) | `mkdir -p $DOC_DIRECTORY/$TIME_STAMP-$FEATURE_NAME` |
| `gen-doc` | 2 (name, prompt) | None (writes to cwd) |
| `clean-room` | 4 (target, spec-dir, impl-dir, prompt) | `mkdir -p` for both spec and impl dirs |

**Differences from `build-feature`:**
- No timestamped directory wrapping. User provides exact paths.
- Two output directories instead of one.
- Target directory is read-only input, not an output.

**Validation needed:**
1. `target-directory` must exist and be readable.
2. `spec-output-directory` parent must exist (or `mkdir -p`).
3. `implementation-directory` parent must exist (or `mkdir -p`).
4. Prompt must be non-empty.
5. Target directory must not be empty.

---

## 4. Agent Isolation Problem

The design acknowledges this as an open question. Current state:

- `run_agent()` passes prompts to `claude --dangerously-skip-permissions` or `cursor-agent`.
- Both agents have full filesystem access.
- Stage 3 and 4 isolation relies entirely on the prompt: *"Do not search for, read, or reference the original codebase."*
- No filesystem-level sandboxing exists in the toolchain.

**Options considered:**
1. **Prompt-only** (current approach) — Simple, no guarantees.
2. **Filesystem guard** — Could use `chmod` to remove read permissions on target dir before stages 3-4, restore after. Fragile if agent runs as same user.
3. **Separate working directory** — Copy specs to a temp dir, run agent from there with no path to target. Agent could still `find` or navigate to it.
4. **Agent log parsing** — Post-hoc check that no files from target-directory appear in agent's tool calls. Possible with claude's `--output-format json` but not implemented.

**Recommendation:** Prompt-only is pragmatically sufficient. Clean-room design in practice relies on organizational controls, not technical enforcement. The legal standard is independent creation, which the spec-mediated process satisfies.

---

## 5. Multi-File Output Handling

`build-feature` produces exactly one file per stage. `clean-room` stage 1 produces N spec files (one per component). Stage 2 modifies those same N files in place (no new files created).

**Impact on existing patterns:**
- `build_prompt()` takes a single filepath — needs adaptation for directory-based output (stage 1) or skipped entirely (stage 2 modifies existing files).
- `wait_for_user()` takes a single filepath — pass directory path instead for stages 1 and 2.
- `run_agent()` is unaffected (prompt-in, agent writes/edits files).
- File references in later stages must glob or list all spec files.

**Approach:** For stage 1, construct the prompt manually (as `build-feature` does for implementation) pointing to the spec directory. For stage 2, prompt the agent to read and update all `spec-*.md` files in the directory. For stages 3 and 4, reference the spec directory.

---

## 6. Stage Re-run Support

Design asks: *"Should the tool support `--stage 2` to re-run from a specific stage?"*

`build-feature` does not support this. Each run starts from stage 1. However, because `clean-room` produces persistent output files, re-running from a specific stage is feasible:
- Stage 2 only needs `spec-*.md` files in the spec directory.
- Stage 3 only needs approved specs.
- Stage 4 only needs implementation + specs.

**Implementation:** Add optional `--stage N` flag. If provided, skip earlier stages and validate that required input files exist.

---

## 7. Existing Files in Output Directories

Design asks what to do if spec/impl dirs already contain files.

**`build-feature` behavior:** Creates a new timestamped directory each run, so collisions don't occur.

**`clean-room` behavior:** User provides exact paths, so collisions are possible.

**Options:**
1. Error if non-empty (safest).
2. Overwrite (destructive).
3. Merge (complex, undefined semantics).
4. Warn and prompt user (matches interactive pattern).

---

## 8. Relevant Files

| File | Relevance |
|------|-----------|
| `bin/helpers.sh` | All shared functions (`run_agent`, `build_prompt`, `wait_for_user`, `validate_args_min`, `init_globals`, `new_doc`). Core infrastructure for `clean-room`. |
| `bin/build-feature` | Architecture template. `clean-room` follows the same multi-stage pipeline pattern with interactive/non-interactive modes and debug session. |
| `bin/gen-doc` | Minimal single-stage example. Shows simplest possible agent invocation. |
| `docs/2603111550-CleanRoomImplementation.md` | Contains the three prompt templates (analyst, reviewer, implementer) to embed in `bin/clean-room`. |
| `docs/2603111600-CleanRoom/01-design.md` | Design document defining all stages, user flows, error handling, and test plan. |
| `.tmp/CLEAN_ROOM.md` | Legal background on clean-room reverse engineering precedent. |
| `.tmp/CHINESE_ROOM.md` | Information barrier / Chinese wall concepts relevant to isolation design. |
| `Makefile` | Will need a `test-clean-room` target added. |
| `README.md` | Will need `clean-room` tool documented (usage, args, env vars). |
| `TODOS.md` | Tracks outstanding work items. `clean-room` should be added. |
| `.gitignore` | Currently ignores `.*` except `.git*`. No impact on clean-room. |

---

## 9. Implementation Complexity Assessment

**Low complexity (reuse existing patterns):**
- Argument parsing, validation, directory creation
- Agent invocation via `run_agent()`
- User review gates via `wait_for_user()`
- Audio notifications and IDE opening via `build_prompt()`

**Medium complexity (adaptation needed):**
- Multi-file output handling (spec-*.md in stage 1, in-place edits in stage 2)
- Prompt construction for stages with directory-based output
- Referencing all spec files in later stage prompts
- `--stage N` re-run support (if implemented)

**Low complexity but high risk:**
- Agent isolation enforcement (prompt-only, no technical guarantee)
- Compliance review accuracy (dependent on agent quality)
- Component discovery completeness (dependent on agent quality)
