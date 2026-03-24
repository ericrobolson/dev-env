---
name: f-new-skill
description: Interactively design and implement a new Claude Code skill (SKILL.md) through structured questioning. Use when user wants to create a new skill, add a slash command, or says "new skill".
---

# New Skill Creator

You are helping the user design and implement a new Claude Code skill. A skill is a SKILL.md file in `.claude/skills/<skill-name>/` that gives Claude Code a reusable prompt triggered by `/skill-name`.

## Phase 1: Core Identity

Grill the user on these questions one at a time. For each, provide 1-3 concrete suggestions for the user to choose from or riff on, based on context. Do not move on until the answer is resolved.

1. **Skill name** — kebab-case identifier, becomes the slash command. (e.g., `deploy-check`, `review-pr`)
2. **One-line description** — what does this skill do? This is used by the system to decide when to invoke it, so it must be precise and include trigger phrases.
3. **Trigger conditions** — when should this skill activate? List explicit phrases or situations. (e.g., "use when user says 'scaffold a new service'")

## Phase 2: Behavior Design

Walk down each branch of the skill's behavior tree. For each question, offer 1-3 concrete suggestions based on what you've learned so far:

4. **Does the skill need user input before it can act?** If yes, what questions must it ask? What are sensible defaults? What can be inferred from context?
5. **What are the discrete steps the skill performs?** List them in order. For each step, clarify:
   - Is it interactive (needs user input) or autonomous?
   - What tools does it use? (Bash, Read, Write, Edit, Glob, Grep, Agent, etc.)
   - What can go wrong, and how should it recover?
6. **Does the skill produce artifacts?** (files, directories, output). What exactly, and where?
7. **Does the skill have a verification/post-creation step?** How does it confirm success?

## Phase 3: Edge Cases & Constraints

For each question below, offer 1-3 concrete suggestions:

8. **What should the skill NOT do?** Explicit anti-patterns or guardrails.
9. **Are there dependencies on the environment?** (installed tools, specific directory structure, etc.)
10. **Should the skill be idempotent?** What happens if run twice on the same input?

## Phase 4: Write the SKILL.md

Once all branches are resolved, synthesize the answers into a SKILL.md file with this structure:

```markdown
---
name: <skill-name>
description: <one-line description with trigger phrases>
---

# <Skill Title>

<One sentence: what you are doing and why.>

## Gather Requirements
<Questions to ask the user. For each question, offer 1-3 concrete suggestions based on context. Include "If the user has already provided these details, skip the questions.">


## <Step 1 Name>

<Precise instructions for step 1.>

## <Step 2 Name>

<Precise instructions for step 2.>

...

## Post-creation

<Verification steps and final user-facing confirmation.>
```

### Writing Guidelines

- Write instructions TO Claude, not about Claude. ("Run `make check`" not "The assistant should run make check".)
- Be precise and imperative. Every instruction should be unambiguous.
- Include literal code/commands where possible — don't leave room for interpretation.
- Keep it minimal. Only include what's needed to produce correct output.
- If the skill creates files, show the exact content or template inline.
- When the generated skill asks the user for input, it MUST provide 1-3 concrete suggestions for each question. Write these suggestions directly into the SKILL.md as examples the skill should offer. The suggestions should be context-aware and specific, not generic placeholders.

## Phase 5: Confirm & Place

- Show the user the full SKILL.md for review.
- Write it to `.claude/skills/<skill-name>/SKILL.md`.
- Verify it appears in the skill list by confirming the file exists.
- Tell the user they can now invoke it with `/<skill-name>`.
