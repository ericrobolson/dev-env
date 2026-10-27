#!/usr/bin/env bash
set -euo pipefail

# Turn a design doc or a short prompt into an implementation checklist that a
# cheaper agent can follow literally. Prints a prompt for the host agent (like
# orchestrate.sh) that has it scout the codebase with cheap subagents, make the
# open decisions, and write planner/<timestamp>_<Name>.md in the current
# directory: scope, frozen decisions, and small TDD phases with exact paths,
# line ranges, data, and build/test gates. A cold read by the worker model
# checks the plan before it is handed over. The planner writes no code. Edit
# the models for each provider below.

usage() {
    cat >&2 <<'USAGE'
Usage: plan.sh <document-path> [notes...]
       plan.sh <prompt...>

Example: plan.sh ../../ideas/261015-101714_BeatQueueNinja.md
Example: plan.sh ~/TechnicalDocument.pdf "only the parser for now"
Example: plan.sh Make a game about tic tac toe
USAGE
}

fail() {
    printf 'Error: %s\n' "$1" >&2
    exit 1
}

if [[ $# -lt 1 ]]; then
    usage
    exit 2
fi

doc_path=''
if [[ -f "$1" ]]; then
    doc_path="$1"
    shift
    [[ -r "$doc_path" ]] || fail "document is not readable: $doc_path"
    # Make relative paths absolute so the agent can find the file from anywhere.
    [[ "$doc_path" == /* ]] || doc_path="$PWD/$doc_path"
fi
request="$*"
[[ -n "$doc_path" || -n "${request//[[:space:]]/}" ]] || fail 'give a document path or a prompt'

printf 'Which agent? [claude/codex/opencode]: '
read -r agent

case "$agent" in
    claude)
        planner=claude-opus-5-5
        worker=claude-haiku-5-5
        ;;
    codex)
        planner=gpt-6-astra
        worker=gpt-6-luna
        ;;
    opencode)
        planner=anthropic/claude-opus-5-5
        worker=anthropic/claude-sonnet-5-5
        ;;
    *)
        fail "unknown agent: $agent"
        ;;
esac

# Default name: the document's name without its timestamp prefix, or the first
# few words of the prompt in CamelCase.
if [[ -n "$doc_path" ]]; then
    file_name=$(basename "$doc_path")
    default_name=$(printf '%s' "${file_name%.*}" | sed -E 's/^[0-9]{6}-[0-9]{6}_//')
else
    default_name=$(printf '%s' "$request" | LC_ALL=C tr -cs '[:alnum:]' ' ' |
        awk '{ for (i = 1; i <= NF && i <= 4; i++) printf "%s%s", toupper(substr($i, 1, 1)), substr($i, 2) }')
fi

printf 'Plan name [%s]: ' "$default_name"
read -r plan_name
plan_name=${plan_name:-$default_name}
safe_name=$(printf '%s' "$plan_name" | LC_ALL=C tr -cs '[:alnum:]_.-' '_' | sed -E 's/^[-_.]+//; s/[-_.]+$//')
[[ -n "$safe_name" ]] || fail 'plan name must contain at least one letter or number'

mkdir -p planner
timestamp=$(date '+%y%m%d-%H%M%S')
plan_file="$PWD/planner/${timestamp}_${safe_name}.md"
[[ ! -e "$plan_file" ]] || fail "plan file already exists: $plan_file"

if [[ -n "$doc_path" ]]; then
    source_line="Design source: \`$doc_path\`"
    source_text="the design doc at $doc_path"
    [[ -z "${request//[[:space:]]/}" ]] || source_text+=", with these notes from me: $request"
else
    source_line="Design source: prompt — $request"
    source_text="this request from me: $request"
fi

cat > "$plan_file" <<PLAN
# $plan_name

Status: drafting
$source_line

_The plan has not been written yet._
PLAN

cat <<PROMPT

Write an implementation checklist for $source_text

Write it to $plan_file, replacing the stub that is there. Work from $PWD. The plan is for $worker, a cheaper agent that will follow it literally, with no context but this file and the code. You ($planner) only plan: do not write or change any project code.

Steps:
1. Read the source in full. If it is a PDF or another binary format, extract the text into the scratchpad or .tmp, never next to the source. Pull out every rule, number, and behaviour it asks for, and list its open questions.
2. Scout the codebase with $worker subagents, in parallel and read-only. Set the model explicitly on every subagent, because default subagents inherit the caller's model. Ask each one a narrow question and have it answer with paths, line ranges, and signatures, not file dumps. Find out:
   - the project root and the exact build and test commands;
   - the conventions file (AGENTS.md, CLAUDE.md, or similar) and the rules in it that matter here;
   - existing code to copy or extend, with the exact line ranges worth reading and the ones to skip;
   - every registry, enum, list, test, and doc that must change when something new is added;
   - files and directories the worker should never read because the plan already covers them.
   Read the key ranges yourself only where a decision depends on them.
3. Run the build and test commands once. Record whether they pass, so the plan's baseline is known to work (or note existing failures).
4. Decide every open question yourself. Prefer the smallest choice that keeps the design intact. Prefer derived state: when a value can be computed from state that already exists (counts, totals, flags, statuses, lookups), compute it where it is used instead of storing, caching, or syncing a copy. Store it only when it can't be derived (user input, external data) or computing it is measurably too slow, and record why as a decision. Write each decision as one line under "Decisions (do not ask)". Do not stop to ask me unless the source contradicts itself about scope; if it does, ask once, with all your questions together.
5. Write the plan in the shape below.
6. Check the plan:
   - Confirm every path, line range, symbol, and command in it exists, with grep or by running it.
   - Spin up one fresh $worker subagent, give it only $plan_file, and ask it to list every step where it would have to guess, read more than the plan names, or ask a question. Fix each one, then run the cold read once more.
7. Set Status to ready. Reply with the plan path, the phase list in one line each, and the decisions you made. Do not paste the plan itself.

Shape of the plan:
- Title line, then: Project (the root, with "all paths are relative to it"), the Design source line as it is now, and one line naming what is being built and what it copies or extends. Then: "You do not need to read the design source. Every rule you need is restated here. If the plan and the source disagree, the plan wins."
- "## How to work". Include:
  - Do the phases in order. Tick each box (\`- [x]\`) in this file as you finish it.
  - Write each phase's tests first and watch them fail, then implement.
  - Run the build and tests only at the **Gate** lines. If a gate fails, fix it before you go on.
  - Make at most 3 attempts at any failing step. After the third, stop, write what you tried and what failed under "Open questions", and report back. Do not guess past it.
  - Never edit or delete an existing test to make it pass.
  - Derive, don't store: compute values from existing state instead of adding fields, counters, caches, or flags that must be kept in sync. Add stored state only where the plan says to.
  - The files and directories not to read, and to read only the line ranges the plan names.
  - The conventions-file rules that matter, restated in a few words.
  - Completeness: everything in scope must work and be tested. No stubs, no TODOs, no enum values for out-of-scope features.
  - Do not commit unless asked.
- "## Scope": **In scope (must all work)** as a list, and **Out of scope** as one line that says not to stub them.
- "## Decisions (do not ask)": the decisions from step 4.
- "## Phase 0 — Baseline": the exact build and test commands, which must pass before anything changes.
- Phases 1..n. Size each so $worker can finish it in one context window, and give each one checkbox per small action. Be concrete:
  - exact file paths, and shell commands (cp, mv, sed) for mechanical copies;
  - code blocks for new types, enums, constants, and public API signatures, with derived values written as functions or computed properties, not fields;
  - tables with exact numbers for data;
  - numbered rules, in order, for logic and edge cases;
  - "Read only lines X–Y" or line-range tables (keep, rewrite, delete) when the worker edits a copied file;
  - one checkbox per test case, written as the behaviour it proves, listed before the code it tests;
  - a **Gate:** line with the exact commands at the end of each phase.
- A manual check phase, if anything can only be checked by running the program. Say: "If you can't run it, say so in the final report; don't claim it was checked."
- A last phase for docs (README, architecture docs, and the like) and a final Gate that reports the pass and fail counts.
- "## Open questions": empty, for the worker.
- "## Implementation notes": empty, for the worker's report of fixes, deviations, and gate results.

Rules:
- Restate everything the worker needs. Never write "see the design doc" or "as appropriate".
- Prefer copying and editing existing code over writing new code, and say exactly what to copy.
- Use plain words and short sentences. One action per checkbox.
- Only write to $plan_file and scratch files.
PROMPT

printf '\nCreated %s\n' "$plan_file" >&2
