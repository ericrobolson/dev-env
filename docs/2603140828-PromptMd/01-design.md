# Design: Prompt Logging (`00-prompts.md`)

## Overview

Add persistent prompt logging to `build-feature` and `gen-doc` pipelines. Each pipeline run writes all prompts to a `00-prompts.md` file in the output directory, enabling reference and replay.

A shared `append_prompt` function in `helpers.sh` handles file creation and appending.

---

## Components

### 1. `append_prompt` function (helpers.sh)

**Signature:** `append_prompt <filepath> <stage_name> <prompt_text>`

**Behavior:**
- If `<filepath>` does not exist, create it with a top-level header `# Prompts`
- Append a markdown section: `## <stage_name>` followed by the prompt text in a fenced code block
- Append a blank line separator after each entry

**Output format:**
```markdown
# Prompts

## Design

```
<prompt text here>
```

## Research

```
<prompt text here>
```
```

### 2. build-feature integration

- File: `<output_dir>/00-prompts.md`
- Call `append_prompt` before each `run_agent` invocation for all 6 stages: design, research, plan, checklist, implementation, debug
- Stage headers match existing stage names (e.g., "Design", "Research", "Plan", "Checklist", "Implementation", "Debug")

### 3. gen-doc integration

- File: `<output_dir>/00-prompts.md`
- Call `append_prompt` once before the single `run_agent "generate"` call
- Stage header: "Generate"

---

## User Flow

### Happy Path

1. User runs `build-feature MyFeature docs "Build X"`
2. Pipeline creates `docs/YYMMDDHHMM-MyFeature/`
3. Before each stage, `append_prompt` writes the full prompt to `00-prompts.md`
4. After all stages complete, `00-prompts.md` contains all 6 prompts in order
5. User can reference or replay any prompt from the file

### Happy Path (gen-doc)

1. User runs `gen-doc MyDoc docs "Write about X"`
2. Pipeline creates `docs/YYMMDDHHMM-MyDoc/`
3. `append_prompt` writes the single prompt to `00-prompts.md`
4. `00-prompts.md` contains one section: "Generate"

### Unhappy Paths

| Scenario | Expected Behavior |
|---|---|
| Pipeline fails mid-run (e.g., stage 3 errors) | `00-prompts.md` contains prompts for stages 1-3 (all stages up to and including the failed one). File is still valid markdown. |
| Output directory doesn't exist yet | `append_prompt` is called after directory creation (already handled by pipeline). No issue. |
| `00-prompts.md` already exists (re-run or manual creation) | `append_prompt` appends to existing file. Duplicate sections may appear. This is acceptable for replay/reference. |
| Prompt contains markdown special characters (backticks, etc.) | Use a fenced code block with triple-backtick + unique fence (e.g., four backticks) to avoid conflicts. |
| Prompt is empty string | Append the section with empty code block. No special handling needed. |
| Disk full / write permission error | `append_prompt` fails; pipeline should continue (prompt logging is non-critical). The function should not `exit 1`. |
| Debug stage runs multiple iterations | Each debug iteration appends a new "Debug" section. Multiple "Debug" headers are acceptable. |

---

## Unknowns / Questions

1. **Should `append_prompt` log the raw prompt or the prompt after `build_prompt` appends the standard suffix?** Recommendation: log the full final prompt (post-`build_prompt`) since that's what the agent actually receives. This is what matters for replay. **DECISION: do this**
2. **Should `clean-room` also get prompt logging?** It's not mentioned in the TODO but follows the same pattern. Defer for now. **DECISION: do this**
3. **Should prompt logging failure be silent or warn?** Recommendation: print a warning to stderr but do not abort the pipeline. **DECISION: do this**
4. **Should the debug stage's conversational/looping prompts all be logged?** The debug stage is interactive and may have multiple user inputs. Only the initial prompt is feasible to log automatically. **DECISION: initial prompt only for this call as the debug file stores its own record**
5. **Fencing strategy for prompts containing backticks:** Use 4-backtick fences (``````) to wrap prompts, since prompts may contain triple-backtick markdown. **DECISION: use 4-backtick fences**

---

## Testing Plan

### Unit Tests (`append_prompt` function)

| # | Test Case | Input | Expected |
|---|---|---|---|
| 1 | Creates new file | Non-existent path, stage "Design", prompt "hello" | File created with `# Prompts` header + `## Design` section |
| 2 | Appends to existing file | Existing file with one section, new stage "Research" | File has both sections, original content preserved |
| 3 | Handles empty prompt | Stage "Plan", empty string | Section created with empty code block |
| 4 | Handles prompt with backticks | Prompt containing triple backticks | Outer fence (4+ backticks) wraps content correctly |
| 5 | Handles prompt with special chars | Prompt with `$`, `"`, `\`, `!` | Content written verbatim, no shell expansion |
| 6 | Multiple appends same stage name | Two calls with stage "Debug" | Both sections appear in file |

### Integration Tests (build-feature)

| # | Test Case | Expected |
|---|---|---|
| 7 | Full pipeline run | `00-prompts.md` exists with 6 sections in order |
| 8 | Pipeline aborted after stage 2 | `00-prompts.md` has 2 sections (Design, Research) |
| 9 | Prompts match what agent received | Content in `00-prompts.md` matches actual prompt piped to `run_agent` |

### Integration Tests (gen-doc)

| # | Test Case | Expected |
|---|---|---|
| 10 | Full gen-doc run | `00-prompts.md` exists with 1 section ("Generate") |
| 11 | Prompt content matches | Logged prompt matches actual prompt sent to agent |

### Edge Case Tests

| # | Test Case | Expected |
|---|---|---|
| 12 | Read-only directory | `append_prompt` warns to stderr, pipeline continues |
| 13 | Very large prompt (>1MB) | File written successfully, no truncation |
