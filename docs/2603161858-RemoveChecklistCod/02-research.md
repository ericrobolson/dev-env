# Research: Remove Checklist IDE Opening

## 1. Problem Statement

Stage 4 (Checklist) in `bin/build-feature` calls `build_prompt` which appends `"Then open the file '<filepath>' in the IDE '<IDE>' so I can review it."` to the prompt. The checklist runs with `--no-interactive`, so opening the file is pointless — the user never reviews it before proceeding.

---

## 2. Current Code Path

### Stage 4 — `bin/build-feature:131-144`

```bash
STAGE="checklist"
CHECKLIST_FILE=$(new_doc "$DOC_DIRECTORY" "04-checklist.md")
CHECKLIST_PROMPT="Given the following plan file in '$PLAN_FILE', generate a checklist..."

echo "==> Generating checklist..."
FINAL_PROMPT=$(build_prompt "$CHECKLIST_FILE" "$CHECKLIST_PROMPT")
append_prompt "$PROMPTS_FILE" "Checklist" "$FINAL_PROMPT"
echo "$FINAL_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1
append_resume "$PROMPTS_FILE"
```

Key: `build_prompt` (line 141) injects both `"Output everything to the file"` AND `"Then open the file in IDE"`.

### `build_prompt` — `bin/helpers.sh:122-134`

```bash
build_prompt() {
    local filepath="$1"
    shift
    local instructions="$@"
    echo "$instructions

$TERSENESS

Output everything to the file '$filepath'.

Then open the file '$filepath' in the IDE '$IDE' so I can review it."
}
```

The IDE-open line is hardcoded. No flag to skip it.

---

## 3. The Specific Issue

The checklist stage already runs `--no-interactive` (line 143). The agent runs in `--print` mode, writes the file, then also opens it in the IDE. Since no `wait_for_user` follows, the opened file is never reviewed — it's visual noise.

Previous feature `2603161256-NoninteractiveChecklist` already made the stage non-interactive and removed `wait_for_user`. But `build_prompt` still injects the IDE-open instruction.

---

## 4. `build_prompt` Usage Across the Codebase

| Caller | File | Line | Stage | Interactive? | IDE open needed? |
|---|---|---|---|---|---|
| Design | `bin/build-feature` | 72 | Yes | Yes | **Yes** |
| Research | `bin/build-feature` | 97 | Yes | Yes | **Yes** |
| Plan | `bin/build-feature` | 124 | Yes | Yes | **Yes** |
| **Checklist** | `bin/build-feature` | **141** | **No** | **No** | **No** |
| Overview | `bin/build-feature` | 225 | No | No | No (but uses `build_prompt`) |
| gen-doc | `bin/gen-doc` | 54 | Yes | Yes | **Yes** |

Note: Stage 99 (Overview) also runs non-interactive but still uses `build_prompt`. Same issue applies there, but the design doc scopes this change to Stage 4 only.

---

## 5. Proposed Fix (from Design Doc)

Stop using `build_prompt` for the checklist stage. Inline the prompt with only the "output to file" instruction:

```bash
FINAL_PROMPT="$CHECKLIST_PROMPT

$TERSENESS

Output everything to the file '$CHECKLIST_FILE'."
```

This removes the IDE-open instruction while keeping the file-output instruction and terseness directive.

---

## 6. Prior Art: Stages That Skip `build_prompt`

### Stage 5 (Implementation) — `bin/build-feature:149-160`

Does **not** use `build_prompt`. Constructs its own prompt. Does not include IDE-open or file-output instructions (agent writes code files directly). This is the precedent for inlining prompts.

### Stage 6 (Debug) — `bin/build-feature:166-193`

Also does **not** use `build_prompt`. Constructs its own prompt with explicit file write instructions.

### `bin/clean-room` — all stages

None use `build_prompt`. All construct prompts inline with `$TERSENESS` appended directly.

---

## 7. Alternative: Add Flag to `build_prompt`

Design doc explicitly rejects this: "Inlining is simpler and avoids changing the shared helper. **DECISION: inlining the prompt is sufficient.**"

Adding a flag would affect the shared helper, require updating all callers (risk), and is unnecessary for a single-stage change.

---

## 8. Downstream Impact

### `CHECKLIST_FILE` consumers
- Stage 5 (line 154): references `$CHECKLIST_FILE` — unaffected. File is still written by the agent.
- `$PLAN_FILE` append: checklist prompt instructs agent to append to plan — unaffected.

### `append_resume`
Called on line 144 — stays. Consistent with all other stages.

### `append_prompt`
Called on line 142 — stays. Logs the prompt to `00-prompts.md`.

---

## 9. Edge Cases

1. **`--print` mode still writes files**: `claude --print` executes tool calls (file writes). Verified by Stages 5 and 99 which both produce output files in `--print` mode.

2. **Overview stage (99) has same issue**: Uses `build_prompt` with `--no-interactive`. Out of scope per design doc but worth noting for a follow-up.

3. **TODOS.md line 19**: `"Update the whole 'open file in ide' section to use similar pattern to 'play-sound'"` — a broader refactor is planned. This fix is an interim step.

---

## 10. Relevant Files

| File | Path | Summary |
|---|---|---|
| build-feature | `bin/build-feature` | Target file. Lines 131-144 (Stage 4). Lines 149-160 (Stage 5) and 199-228 (Stage 99) show inlined prompt patterns. |
| helpers.sh | `bin/helpers.sh` | `build_prompt` (122-134) — the function that injects IDE-open. `run_agent` (40-79), `init_globals` (25-38) for `$TERSENESS`. |
| gen-doc | `bin/gen-doc` | Uses `build_prompt` (line 54). Not affected by this change. |
| clean-room | `bin/clean-room` | All stages inline prompts without `build_prompt`. Prior art for the pattern. |
| TODOS.md | `TODOS.md` | Line 18: checklist non-interactive (done). Line 19: broader IDE-open refactor (future). |
| Prior research | `docs/2603161256-NoninteractiveChecklist/02-research.md` | Previous research on making checklist non-interactive. Covers `run_agent`, `wait_for_user`, `--print` mode behavior. |
| Design doc | `docs/2603161858-RemoveChecklistCod/01-design.md` | Design specification for this feature. |

---

## 11. Change Summary

**One file**: `bin/build-feature`, line 141.

Replace:
```bash
FINAL_PROMPT=$(build_prompt "$CHECKLIST_FILE" "$CHECKLIST_PROMPT")
```

With:
```bash
FINAL_PROMPT="$CHECKLIST_PROMPT

$TERSENESS

Output everything to the file '$CHECKLIST_FILE'."
```

One line changed. No new functions. No helper changes. Follows existing patterns from Stages 5, 6, and `clean-room`.
