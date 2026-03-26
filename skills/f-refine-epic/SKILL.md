---
name: f-refine-epic
description: Interactively refine an existing epic file by grilling the user on design decisions, resolving ambiguities, and updating the epic with concrete decisions. Use when user says 'refine epic', 'f-refine-epic', 'grill me on this epic', 'improve epic', or 'review epic'.
---

# Refine Epic

Interactively grill the user on every aspect of an epic file until all design branches are resolved, then update the epic with concrete decisions and an expanded checklist.

## Coding Philosophy

Apply these principles when making recommendations throughout the session:

1. **Do one thing well.** Build small, focused programs. When you need new behavior, write a new program or library rather than bloating an existing one with options.
2. **Compose through text.** Every program is a filter: it takes text in and produces text out. Avoid captive interfaces, rigid formats, and extraneous output so programs can chain freely. The power of a system lives in the connections, not the components.
3. **Prototype early, rebuild often.** Get something working in weeks. Throw away the clumsy parts without hesitation. Favor polishing what survives over planning what might.
4. **Build tools, not labor.** Automate with programs and shell scripts, even throwaway ones. Leverage existing software before writing new code. Generate code rather than writing it by hand.
5. **Keep it simple and transparent.** Write readable, modular code. Make data structures rich when needed but keep the program logic plain. Simplicity of interface and implementation beats correctness, consistency, and completeness.
6. **Separate mechanism from policy.** Build flexible, extensible engines and let the caller decide behavior. Build on what users already know.
7. **Portability over efficiency.** Flat text files, standard interfaces, developer time valued over machine time.
8. **Fail loud, fail clear.** Programs should be robust and when they do fail, they should make the problem easy to diagnose.
9. **If you're adding a flag, you missed the design.** Feature accretion and monolithic subsystems are symptoms of not hitting the right abstraction.
10. **Functional over imperative.** Prefer pure functions over stateful objects as it enables easier testing and composition except when involving state or side effects.
11. **Hexagonal architecture.** Separate the domain logic from the infrastructure/IO.
12. **Data driven approach.** Use data to drive the decision making process rather than rules or heuristics that are hard coded wherever possible.

All recommendations, design decision suggestions, and checklist expansions MUST align with these principles. When a principle is relevant to a decision, cite it by quoting its bold title and key phrase (e.g., "per *Do one thing well* — write a new program rather than bloating an existing one").

## Gather Requirements

Ask the user which epic file to refine. Offer suggestions by globbing `epics/**/*.md` (excluding README.md files).

- Suggestion A: The most recently modified epic file found
- Suggestion B: Let the user specify a path

If the user provided an epic file path as an argument, skip this question.

Then ask: "Is there a source plan file for this epic?" Check the epic's header for a `> Source:` line. If found, attempt to read that plan file. If not found or the file doesn't exist, ask the user if there is a plan file. If no plan exists, proceed without it.

## Step 1: Load Context

1. Read the target epic file in full.
2. If a source `.plan.md` was identified, read it to understand the broader project context.
3. Read all sibling epic files in the same `epics/<folder>/` directory (just filenames and `## Summary` sections) to understand the full project scope. Hold these for the cross-reference check in Step 3.

## Step 2: Grill the User

Walk down every branch of the epic's design tree, one question at a time. For each unresolved or underspecified area, ask a focused question and provide your recommended answer grounded in the Coding Philosophy above. Do not move on until the branch is resolved.

Cover these areas (skip any that are already well-resolved in the epic):

- **Scope & boundaries** — What's in this epic, what's explicitly out? Are there responsibilities that could bleed in from adjacent epics?
- **Tech choices** — What technologies, libraries, patterns? What alternatives were considered and why were they rejected?
- **Interface design** — APIs, data formats, protocols, file formats. How do consumers interact with what this epic produces?
- **Dependencies** — What must exist before this epic can start? What does this epic produce that others depend on?
- **Edge cases & failure modes** — What happens when things go wrong? What are the boundary conditions?
- **Vague checklist items** — Any checklist item that is hand-wavy or could mean multiple things gets interrogated and expanded into concrete deliverables.

Rules for the grill session:
- One question at a time. Wait for the user's response before moving on.
- Always provide a recommended answer with your question. Frame it as: "My recommendation: [X], because *[principle title]* — [principle essence]. What do you think?"
- If the user's answer contradicts a Coding Philosophy principle, flag it respectfully and ask if they want to proceed anyway.
- Track resolved decisions as you go — you will need them for the rewrite.

## Step 3: Cross-Reference Sibling Epics

Before rewriting, review the sibling epic summaries loaded in Step 1. Check for:

- **Conflicts** — Does a decision made in this session contradict something stated in another epic?
- **Dependency changes** — Does this epic now require something from another epic that wasn't previously expected, or vice versa?
- **Scope overlap** — Did the grill session pull in responsibilities that belong to another epic?

If any conflicts or changes are found, present them to the user one at a time with a recommended resolution. Do not proceed to the rewrite until all cross-epic concerns are resolved.

## Step 4: Rewrite the Epic

Once all branches are resolved and cross-references checked, update the epic file:

1. Add or update a `## Design Decisions` section after `## Summary` (or after `## Guiding Tenets` if present). Each decision gets a `### Decision Title` subsection with:
   - A short paragraph explaining the decision and rationale.
   - A `**Related epics:**` line listing any sibling epics affected by or related to this decision (by number and title, e.g., "Epic 7: FX Processing, Epic 12: MIDI"). Omit this line if the decision is self-contained within the epic.
2. Update `## Implementation Phases` — expand, split, reword, or add checklist items based on what was learned. Remove items that are no longer relevant.
3. Update any other section the user directed changes to during the session.
4. Add a `> Revised: <YYYY-MM-DD> (refine-epic session)` line to the epic header if not already present, or update the date if it is.

Show the user the full updated epic content before writing to disk. Ask: "Accept this revision, or request changes?" Revise and show again until accepted. Once accepted, write the file.

## Post-creation

1. Confirm the file was written successfully.
2. Summarize what changed:
   - Number of design decisions added or updated
   - Number of checklist items added, modified, or removed
   - Any cross-epic concerns that were flagged and resolved
3. Note any open questions that were not fully resolved during the session (if any).
