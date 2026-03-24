---
name: f-plan
description: Interactively define a software product — from idea through use cases, interfaces, data model, tech stack, and task breakdown — outputting a timestamped plan file. Use when user says 'f-plan' or 'create a f plan'.
---

# F Plan

Walk the user through the following stages to produce a comprehensive planning document. No source code is written — this is planning only. Interview them relentlessly about every aspect of this plan until we reach a shared understanding. Walk down each branch of the design tree, resolving dependencies between decisions one-by-one. For each question, provide your recommended answer.

## Gather Requirements

Ask the user two questions:

1. **Feature name** — a short kebab-case identifier for the filename (e.g., `invoice-generator`, `chat-app`, `log-viewer`).
2. **One-liner description** — a sentence describing what they want to build (e.g., "A CLI tool that generates invoices from time-tracking CSV exports").

If the user has already provided these details (e.g., as arguments to `/f-plan`), skip the questions.

## Setup

If user does not specify a file to write the plan to, generate a timestamp in `YYMMDD-HHMMSS` format using the current date/time. Create the plan file at `<cwd>/plans/YYMMDD-HHMMSS-<FEATURE>.plan.md` otherwise use their specified file. Write the plan to the file with an initial header:

```markdown
# Plan: <Feature Name>
<one-liner description>
```

Write this to disk immediately. All subsequent stages append to this file.

## Stage Loop

For each stage below, follow this process:

1. **Present** the stage goal to the user. For stages that need user input, ask the relevant questions (with suggestions).
2. **Draft** the stage content using the user's input and all prior stage content as context.
3. **Show** the draft to the user.
4. **Iterate** — if the user provides feedback, redraft incorporating it. Repeat until the user approves (empty input or explicit approval).
5. **Append** the approved content under the stage header to the plan file on disk.
6. Move to the next stage.

Never skip the user review loop on any non-skipped stage. Every draft must be shown and approved.

## Stage 0: Product Description

Append a `## Stage 0: Product Description` header to the plan file. Use the one-liner description from the gather step as the seed. Ask the user to elaborate on what they want to build — who is it for, what problem does it solve, any constraints. Draft a concise product description and iterate until approved.

## Stage 1: Use Cases

Append `## Stage 1: Use Cases`. Using the product description as context, draft a set of use cases. Each use case should have a name, actor, and flow. Iterate with the user until approved.

## Stage 2: User-Facing Interfaces

Append `## Stage 2: User-Facing Interfaces`. Based on stages 0–1, determine the interface type:
- **For UIs**: generate markdown mockups of key screens.
- **For CLIs**: describe commands, flags, and show usage examples.
- **For APIs**: describe endpoints, payloads, and show request/response examples.

Ask the user which interface type applies (suggest based on context). Draft and iterate until approved.

## Stage 3: Actions

Append `## Stage 3: Actions`. Based on stages 0–2, determine the actions the system must perform. List each action with a name, trigger, and what it does. Draft and iterate until approved.

## Stage 4: Inputs and Outputs

Append `## Stage 4: Inputs and Outputs`. Based on stages 0–3, determine all inputs (e.g., keyboard, files, API requests) and outputs (e.g., screen, files, audio, API responses). Draft and iterate until approved.

## Stage 5: Data Model

Append `## Stage 5: Data Model`. Based on stages 0–4, draft the data model — entities, their attributes, and relationships. Draft and iterate until approved.

## Stage 6: Error Handling

Append `## Stage 6: Error Handling`. Based on stages 0 and 3–5, propose an error handling strategy. Ask the user for their desired robustness level. Suggestions:
- (A) Quick-and-dirty — panic/crash on unexpected errors, basic validation
- (B) Production-grade — structured error types, recovery strategies, user-facing error messages
- (C) Somewhere in between — specify

Draft the strategy and iterate until approved.

## Stage 7: Replayability (Skippable)

Ask the user if replayability is needed. Present options:
- (A) Idempotent operations
- (B) Event sourcing
- (C) Undo/redo
- (D) Deterministic builds
- (E) Skip — not needed

If the user selects (E), append `## Stage 7: Replayability` with "N/A — not needed" and move on.

Otherwise, append `## Stage 7: Replayability`. Based on stages 0 and 3–5, draft a replayability solution using the selected approach(es). Iterate until approved.

## Stage 8: Persistence (Skippable)

Ask the user if persistence is needed. Present options:
- (A) File system
- (B) SQLite
- (C) Postgres
- (D) Cloud storage
- (E) Other — specify
- (F) Skip — not needed

If the user selects (F), append `## Stage 8: Persistence` with "N/A — not needed" and move on.

Otherwise, append `## Stage 8: Persistence`. Based on stages 0 and 5, draft a persistence solution using the selected approach(es). Iterate until approved.

## Stage 9: Tech Stack

Append `## Stage 9: Tech Stack`. Based on all prior stages, propose a tech stack — language(s), frameworks, libraries, build tools. Include a `### Build & Test Commands` subsection with:

```
build: <cmd>
test: <cmd>
```

Iterate until approved.

## Stage 10: Monitoring & Observability

Append `## Stage 10: Monitoring & Observability`. Based on stages 0–9, draft a monitoring and observability plan. Ask the user what level of monitoring they need. Suggestions:
- (A) Minimal — basic health checks and stdout logging
- (B) Standard — structured logging, key metrics, alerting on errors
- (C) Comprehensive — distributed tracing, dashboards, SLOs/SLIs, on-call alerting
- (D) Skip — not needed

If the user selects (D), append with "N/A — not needed" and move on.

Otherwise, draft a monitoring plan covering relevant aspects: logging strategy, key metrics to track, health checks, alerting rules, and dashboards. Iterate until approved.

## Stage 11a: Epics

Append `## Stage 11a: Epics`. Based on all prior stages, generate high-level epics that cover the full implementation scope. Each epic should have a name and brief description. Draft and iterate until approved.

## Stage 11b: Phases

Append `## Stage 11b: Phases`. Break each epic into phases. Each phase should have a name, the parent epic, and a description of what it accomplishes. Draft and iterate until approved.

## Stage 11c: Tasks

Append `## Stage 11c: Tasks`. Break each phase into concrete implementation tasks. Each task should be specific enough that a developer (or LLM) could execute it without ambiguity. Draft and iterate until approved.

## Post-creation

After all stages are complete, append `## Status: complete` to the plan file.

Print:
- The full path to the plan file
- A summary listing each stage and whether it was completed or skipped (N/A)

## Constraints

- Do not write source code or create project directories — this is planning only.
- Do not modify existing plan files — always create a new one.
- Write to the plan file incrementally after each stage is approved.
- Always show drafts to the user and wait for approval before proceeding.
