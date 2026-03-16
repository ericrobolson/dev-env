# Research: Non-interactive Checklist Generation

## 1. Current Implementation Analysis

### Stage 4 (Checklist) — `bin/build-feature:130-144`

```bash
STAGE="checklist"
CHECKLIST_FILE=$(new_doc "$DOC_DIRECTORY" "04-checklist.md")
CHECKLIST_PROMPT="Given the following plan file in '$PLAN_FILE', generate a checklist..."
FINAL_PROMPT=$(build_prompt "$CHECKLIST_FILE" "$CHECKLIST_PROMPT")
append_prompt "$PROMPTS_FILE" "Checklist" "$FINAL_PROMPT"
echo "$FINAL_PROMPT" | run_agent "$STAGE" || exit 1
append_resume "$PROMPTS_FILE"
wait_for_user "$CHECKLIST_FILE"
```

Three operations to change:
1. **Line 142**: `run_agent "$STAGE"` → `run_agent "$STAGE" --no-interactive`
2. **Line 144**: Remove `wait_for_user "$CHECKLIST_FILE"`
3. **Line 143**: Keep `append_resume` (harmless no-op in `--print` mode)

---

## 2. How `run_agent` Handles `--no-interactive`

Defined in `bin/helpers.sh:40-79`.

- Parses `$2 == "--no-interactive"` → sets `interactive=false`
- Appends `--print` to the agent CLI command
- For `claude`: `claude --dangerously-skip-permissions --print`
- For `cursor`: `cursor-agent --model "$CURSOR_MODEL" --print`

**Key detail**: `--print` mode in `claude` CLI still executes tool calls (file writes, edits). It only suppresses the interactive TUI. The agent can still create/modify files on disk. This is confirmed by Stage 5 (Implementation) which runs `--no-interactive` and successfully writes code files.

---

## 3. How `wait_for_user` Works

Defined in `bin/helpers.sh:89-118`.

- **Skips immediately** if `AGENT_TYPE=claude` (line 95)
- **Skips immediately** if non-interactive shell (`[[ ! -t 0 ]]`, line 95)
- **Blocks only** for `AGENT_TYPE=cursor` in interactive TTY

**Impact of removal**: For `claude` users, no behavior change (already skipped). For `cursor` users, removes the only review point between plan and implementation.

---

## 4. Prior Art: Existing Non-interactive Stages

### `bin/build-feature`

| Stage | Line | Mode | `wait_for_user` |
|---|---|---|---|
| 1 Design | 74 | Interactive | Yes |
| 2 Research | 99 | Interactive | Yes |
| 3 Plan | 126 | Interactive | Yes |
| **4 Checklist** | **142** | **Interactive** | **Yes** |
| 5 Implementation | 159 | `--no-interactive` | No |
| 99 Overview | 227 | `--no-interactive` | No |

Stages 5 and 99 are the established pattern. Both:
- Pass `--no-interactive` to `run_agent`
- Skip `wait_for_user`
- Still call `append_resume` (line 160, 228)

### `bin/clean-room`

Both analysis (line 83) and compliance (line 117) stages run `--no-interactive`. Neither calls `wait_for_user`. Both call `append_resume`. Same pattern.

---

## 5. `CHECKLIST_FILE` Dependency Chain

### Creation
- `bin/build-feature:134`: `CHECKLIST_FILE=$(new_doc "$DOC_DIRECTORY" "04-checklist.md")`
- `new_doc` (`helpers.sh:83-87`) just returns `"$dir/$filename"` — does **not** create the file

### Who writes to it?
- The checklist prompt (`build_prompt`) includes: `"Output everything to the file '$filepath'."` via `build_prompt` (`helpers.sh:122-134`)
- The agent is instructed to write to `$CHECKLIST_FILE`
- The prompt also says: `"Append the generated checklist to the end of the content in '$PLAN_FILE'."`
- So the agent writes to **both** `CHECKLIST_FILE` and `PLAN_FILE`

### Downstream consumer
- **Stage 5 (line 154)**: `"Implement the following plan document in '$CHECKLIST_FILE'."`
- If the agent fails to create `04-checklist.md`, Stage 5 will reference a nonexistent file

### Risk in `--print` mode
- `claude --print` still executes file write tool calls. The agent will create `04-checklist.md`.
- Verified by analogy: Stage 5 runs `--no-interactive` and produces code files; Stage 99 runs `--no-interactive` and produces `99-overview.md`.

---

## 6. `append_resume` in Non-interactive Mode

Defined in `helpers.sh:177-211`.

- Only runs for `AGENT_TYPE=claude`
- Finds most recent `.jsonl` in `~/.claude/projects/`
- In `--print` mode, `claude --print` still creates a session file (it's a full agent run, just non-interactive)
- Existing non-interactive stages (5, 99) both call `append_resume` — so keeping it is consistent
- Worst case: silently no-ops if no session found

---

## 7. `build_prompt` Suffix

`helpers.sh:122-134` appends to every prompt:

```
$TERSENESS

Output everything to the file '$filepath'.

Then open the file '$filepath' in the IDE '$IDE' so I can review it.
```

In `--print` mode, the "open file in IDE" instruction still works — the agent calls a tool to open the file. No issue.

---

## 8. Challenges and Edge Cases

### 8.1 Checklist quality without review
- Currently, `cursor` users get a review point. Removing it means malformed checklists go straight to implementation.
- **Mitigation**: Plan was already reviewed. Debug stage (6) exists for corrections.

### 8.2 Agent failure in `--print` mode
- Interactive mode allows retries. `--print` mode fails once → `exit 1`.
- This is identical to how Stages 5 and 99 handle failure. Acceptable.

### 8.3 `build_prompt` adds IDE open instruction
- In `--print` mode, the agent may still attempt to open the file in IDE.
- Not harmful — agent runs tool call, file opens. Consistent with Stages 5 and 99.

### 8.4 Race condition: plan file write
- Checklist prompt tells agent to both write `04-checklist.md` AND append to `03-plan.md`.
- In `--print` mode, tool calls are still sequential within the agent. No race condition.

---

## 9. Relevant Files

| File | Path | Relevance |
|---|---|---|
| build-feature | `bin/build-feature` | Target file. Lines 130-144 contain Stage 4. Lines 149-160 (Stage 5) and 224-228 (Stage 99) are reference patterns. |
| helpers.sh | `bin/helpers.sh` | Defines `run_agent` (40-79), `wait_for_user` (89-118), `new_doc` (81-87), `append_resume` (177-211), `build_prompt` (122-134). |
| clean-room | `bin/clean-room` | Additional prior art for `--no-interactive` pattern (lines 83, 117). |
| gen-doc | `bin/gen-doc` | Single-stage doc generator. Uses `run_agent` without `--no-interactive`. Not directly relevant but shows `append_resume` usage. |
| TODOS.md | `TODOS.md` | Line 18: `[ ] Make checklist stage non-interactive` — this is the tracked item. |
| test-append-prompt.sh | `tests/test-append-prompt.sh` | Tests for `append_prompt` and `append_resume`. May need update if behavior changes. |
| test-overview-stage.sh | `tests/test-overview-stage.sh` | Tests for overview stage (non-interactive). Reference for testing patterns. |
| design doc | `docs/2603161256-NoninteractiveChecklist/01-design.md` | Design specification for this feature. |

---

## 10. Summary of Required Changes

**Single file change**: `bin/build-feature`, lines 142-144.

```diff
-echo "$FINAL_PROMPT" | run_agent "$STAGE" || exit 1
+echo "$FINAL_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1
 append_resume "$PROMPTS_FILE"
-wait_for_user "$CHECKLIST_FILE"
```

Two lines changed. Zero new functions. Zero new files. Follows established pattern from Stages 5, 99, and clean-room.
