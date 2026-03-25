---
name: f-build-epics
description: Generate epic breakdown folders with implementation checklists from a .plan.md file. Use when user says 'f-build-epics', 'build epics', or 'generate epics from plan'.
---

# Build Epics from Plan

Generate a timestamped epics folder from a `.plan.md` file, creating one markdown file per epic with summary, plan reference, Unix Philosophy tenets, and per-phase implementation checklists.

## Gather Requirements

Ask the user which `.plan.md` file to use. Offer suggestions by globbing for `**/*.plan.md`.

- Suggestion A: The most recently modified `.plan.md` file found
- Suggestion B: Let the user specify a path

If the user provided a plan file path as an argument, skip this question.

## Step 1: Parse the Plan

1. Read the specified `.plan.md` file.
2. Extract the **feature name** from the filename: strip any leading date prefix (`YYMMDD-HHMMSS-`) and the `.plan.md` suffix. Example: `260324-000651-mut8.plan.md` becomes `mut8`.
3. Extract all epics from the `## Stage 11a: Epics` section. Each epic starts with `**Epic N: <Title>**` followed by a description paragraph.
4. Extract all phases from the `## Stage 11b: Phases` section. Phases are grouped under `### Epic N: <Title>` headings. Each phase starts with `- **P<epic>.<phase>: <Title>**` followed by a description.
5. Extract all tasks from the `## Stage 11c: Tasks` section. Tasks are grouped under `### Phase P<epic>.<phase>: <Title>` headings. Each task starts with `- **T<epic>.<phase>.<task>: <Title>**` followed by a description.
6. Read the Unix Philosophy tenets from `docs/UNIX_PHILOSOPHY_DISTILLED.md`.

## Step 2: Create the Epics Folder

1. Generate a timestamp in `YYMMDD-HHMMSS` format using the current date/time.
2. Create the directory `epics/<timestamp>-<feature>/`. Example: `epics/260324-143052-mut8/`.

## Step 3: Generate Epic Files

For each epic, create a file named `NN-<kebab-case-title>.md` where `NN` is the zero-padded epic number and the title is the epic title converted to kebab-case (lowercase, spaces to hyphens, strip non-alphanumeric except hyphens). Example: `01-core-audio-engine.md`.

Each epic file MUST follow this exact template:

```markdown
# Epic <N>: <Title>

> Source: `<relative path to .plan.md file>`
> Epic: <N> of <total epics>
> Generated: <YYYY-MM-DD HH:MM:SS>

## Summary

<Epic description from Stage 10a, reproduced verbatim.>

## Guiding Tenets

1. **One thing well** — small, focused programs; new behavior = new program.
2. **Compose through text** — text in, text out; no captive interfaces.
3. **Prototype early, rebuild often** — working in weeks; discard what's clumsy.
4. **Tools, not labor** — automate everything; generate code; reuse before writing.
5. **Simple and transparent** — plain logic, readable code; simplicity > correctness.
6. **Mechanism, not policy** — flexible engines; caller decides behavior.
7. **Portability > efficiency** — flat text, standard interfaces, value dev time.
8. **Fail loud, fail clear** — robust programs; obvious diagnostics.
9. **No flags** — if you're adding a flag, you missed the abstraction.
10. **Functional over imperative** — prefer pure functions over stateful objects as it enables easier testing and composition except when involving state or side effects.
11. **Hexagonal architecture** — separate the domain logic from the infrastructure/IO.
12. **Data driven approach** — use data to drive the decision making process rather than rules or heuristics that are hard coded wherever possible.

## Implementation Phases

<For each phase belonging to this epic, generate a section:>

### Phase <epic>.<phase>: <Phase Title>

<Phase description from Stage 10b, reproduced verbatim.>

#### Checklist

- [ ] <One checklist item per task from Stage 11c belonging to this phase, expanded with concrete deliverables>
- [ ] Write tests
- [ ] Verify integration with dependent phases
```

**Checklist generation rules:**
- Start from the tasks listed in Stage 11c for this phase. Each task becomes one or more checklist items.
- Expand each task into concrete, actionable deliverables. If a task implies multiple components (e.g., "delay, reverb, distortion, chorus"), create a separate checklist item for each.
- If a task is already specific enough, use it as-is for the checklist item.
- Always include `Write tests` and `Verify integration with dependent phases` as the final two items.
- Keep checklist items concise (one line each).

## Step 4: Review Each Epic with User

Process epics **one at a time, sequentially**. For each epic:

1. Generate the epic file content per the template above.
2. Show the user the full rendered content **before writing it to disk**.
3. Ask: "Accept this epic, or request changes?"
4. If the user requests changes, revise and show again. Repeat until accepted.
5. Once accepted, write the file to disk. Confirm: "Wrote `NN-<name>.md`. Moving to Epic <N+1>."
6. Proceed to the next epic.

Do not generate or show the next epic until the current one is accepted.

## Step 5: Generate Index

Create a `README.md` in the epics folder with this template:

```markdown
# Epics: <feature>

> Source: `<relative path to .plan.md file>`
> Generated: <YYYY-MM-DD HH:MM:SS>

| # | Epic | Phases | File |
|---|------|--------|------|
| 1 | <Title> | <phase count> | [01-<kebab>.md](01-<kebab>.md) |
| ... | ... | ... | ... |
```

## Post-creation

1. Confirm the directory was created and list all generated files.
2. Report the total number of epics and phases generated.
3. Tell the user: "Epic breakdown written to `epics/<timestamp>-<feature>/`. Each file contains the epic summary, implementation tenets, and per-phase implementation checklists."
