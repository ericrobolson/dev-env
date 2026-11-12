#!/usr/bin/env bash
set -euo pipefail

# A resumable, agent-guided tutorial where the human writes the code. `new`
# scaffolds tutor/<timestamp>_<Name>/ in the current directory and prints a
# prompt for the host agent (Claude, Codex, OpenCode) that has it design a
# step-by-step curriculum from a plan file or prompt. Each step is assigned,
# the human implements it with hints on request, then the agent reviews the
# diff since the step began and decides whether to move on.
#
# Durability: everything lives on disk. The curriculum's checkboxes are the
# progress (the current step is the first unchecked one), lessons/ holds the
# written tutorial, log.md is the session journal, and each step's starting
# point is a git tree snapshot kept under refs/tutor/<id>/step-<n>, so a review
# sees exactly what the human changed, including new untracked files, even
# across sessions and commits. Prompts print to stdout so they can be passed
# straight to an agent: claude "$(tutor.sh resume)"

usage() {
    cat >&2 <<'USAGE'
Usage: tutor.sh new <document-path> [notes...]   start a tutorial from a plan file
       tutor.sh new <prompt...>                  start a tutorial from a prompt
       tutor.sh resume [dir]                     print the prompt to continue a tutorial
       tutor.sh status [dir]                     show the current step and its changes
       tutor.sh begin [dir] [--force]            snapshot the start of the current step
       tutor.sh diff [dir] [--stat]              show what changed since the step began
       tutor.sh list                             list tutorials under ./tutor

[dir] defaults to the newest tutorial under ./tutor (or the one you are in).

Example: claude "$(tutor.sh new ../ideas/261015-101714_GainEater.md 'just the parser')"
Example: claude "$(tutor.sh resume)"
USAGE
}

fail() {
    printf 'Error: %s\n' "$1" >&2
    exit 1
}

script_path=$(readlink -f "${BASH_SOURCE[0]}")

# --- locating a tutorial -----------------------------------------------------

# Prints the tutorial directory: the argument if given, the tutorial containing
# the current directory, or the newest one under ./tutor.
find_tutorial() {
    local dir="${1:-}"
    if [[ -n "$dir" ]]; then
        [[ -f "$dir/curriculum.md" ]] || fail "not a tutorial directory: $dir"
        (cd "$dir" && pwd)
        return
    fi
    local here=$PWD
    while [[ "$here" != / ]]; do
        if [[ -f "$here/curriculum.md" && -f "$here/TUTOR.md" ]]; then
            printf '%s\n' "$here"
            return
        fi
        here=$(dirname "$here")
    done
    local newest
    newest=$(find tutor -mindepth 2 -maxdepth 2 -name curriculum.md 2>/dev/null | sort | tail -n 1)
    [[ -n "$newest" ]] || fail 'no tutorial found under ./tutor; start one with: tutor.sh new <plan-or-prompt>'
    (cd "$(dirname "$newest")" && pwd)
}

# Reads "Key: value" from the header of curriculum.md.
meta() {
    sed -n "s/^$2: //p" "$1/curriculum.md" | head -n 1
}

# --- derived progress --------------------------------------------------------

# Steps are "- [ ] Step N — Title" lines in curriculum.md. Prints the number
# of the first unchecked step, or nothing when all are done.
current_step() {
    sed -nE 's/^- \[ \] Step ([0-9]+).*/\1/p' "$1/curriculum.md" | head -n 1
}

step_title() {
    sed -nE "s/^- \[[ x]\] Step $2 [—-]+ (.*)/\1/p" "$1/curriculum.md" | head -n 1
}

count_steps() {
    local total done
    total=$(grep -cE '^- \[[ x]\] Step [0-9]+' "$1/curriculum.md" || true)
    done=$(grep -cE '^- \[x\] Step [0-9]+' "$1/curriculum.md" || true)
    printf '%s/%s' "$done" "$total"
}

step_ref() {
    printf 'refs/tutor/%s/step-%s' "$(meta "$1" Id)" "$2"
}

# Hints given for a step, counted from the log's "### Hint (step N" entries.
hint_count() {
    grep -cE "^### Hint \(step $2[,)]" "$1/log.md" || true
}

# --- git snapshots -----------------------------------------------------------

# Writes a tree of the whole working tree (tracked + untracked, minus ignored
# files) without touching the real index, HEAD, or stash. Prints the tree id.
snapshot_tree() {
    local root=$1 idx tree
    idx=$(mktemp)
    local real_index
    real_index=$(git -C "$root" rev-parse --path-format=absolute --git-path index)
    if [[ -f "$real_index" ]]; then cp "$real_index" "$idx"; else rm -f "$idx"; fi
    GIT_INDEX_FILE=$idx git -C "$root" add -A -- . >/dev/null 2>&1
    tree=$(GIT_INDEX_FILE=$idx git -C "$root" write-tree)
    rm -f "$idx"
    printf '%s\n' "$tree"
}

# Pathspec that hides the tutorial folder itself from diffs.
exclude_spec() {
    local tdir=$1 root=$2
    printf ':(top,exclude)%s' "$(realpath --relative-to="$root" "$(dirname "$tdir")")"
}

cmd_begin() {
    local dir='' force=false
    for arg in "$@"; do
        case "$arg" in
            --force) force=true ;;
            *) dir=$arg ;;
        esac
    done
    local tdir root step ref
    tdir=$(find_tutorial "$dir")
    root=$(meta "$tdir" Project)
    step=$(current_step "$tdir")
    [[ -n "$step" ]] || fail 'every step is done; nothing to begin'
    ref=$(step_ref "$tdir" "$step")
    if git -C "$root" rev-parse -q --verify "$ref" >/dev/null && ! $force; then
        printf 'Step %s already began at %s (use --force to reset the baseline).\n' \
            "$step" "$(git -C "$root" rev-parse --short "$ref")"
        return
    fi
    git -C "$root" update-ref "$ref" "$(snapshot_tree "$root")"
    printf 'Step %s began. Baseline %s saved at %s.\n' \
        "$step" "$(git -C "$root" rev-parse --short "$ref")" "$ref"
}

cmd_diff() {
    local dir='' stat=false
    for arg in "$@"; do
        case "$arg" in
            --stat) stat=true ;;
            *) dir=$arg ;;
        esac
    done
    local tdir root step ref now
    tdir=$(find_tutorial "$dir")
    root=$(meta "$tdir" Project)
    step=$(current_step "$tdir")
    [[ -n "$step" ]] || fail 'every step is done; nothing to diff'
    ref=$(step_ref "$tdir" "$step")
    git -C "$root" rev-parse -q --verify "$ref" >/dev/null ||
        fail "step $step has not begun; run: tutor.sh begin"
    now=$(snapshot_tree "$root")
    if $stat; then
        git -C "$root" --no-pager diff --stat "$ref" "$now" -- . "$(exclude_spec "$tdir" "$root")"
    else
        git -C "$root" --no-pager diff "$ref" "$now" -- . "$(exclude_spec "$tdir" "$root")"
    fi
}

cmd_status() {
    local tdir root step ref
    tdir=$(find_tutorial "${1:-}")
    root=$(meta "$tdir" Project)
    step=$(current_step "$tdir")
    printf 'Tutorial: %s\n' "$(meta "$tdir" Title)"
    printf 'Folder:   %s\n' "$tdir"
    printf 'Project:  %s\n' "$root"
    printf 'Progress: %s steps done\n' "$(count_steps "$tdir")"
    if [[ -z "$step" ]]; then
        if grep -qE '^- \[[ x]\] Step [0-9]+' "$tdir/curriculum.md"; then
            printf 'State:    complete\n'
        else
            printf 'State:    curriculum not written yet\n'
        fi
        return
    fi
    printf 'Current:  Step %s — %s\n' "$step" "$(step_title "$tdir" "$step")"
    printf 'Hints:    %s given for this step\n' "$(hint_count "$tdir" "$step")"
    ref=$(step_ref "$tdir" "$step")
    if git -C "$root" rev-parse -q --verify "$ref" >/dev/null; then
        local changes
        changes=$(cmd_diff "$tdir" --stat | tail -n 1)
        printf 'State:    in progress since %s; %s\n' \
            "$(git -C "$root" rev-parse --short "$ref")" "${changes:-no changes yet}"
    else
        printf 'State:    not begun\n'
    fi
}

cmd_list() {
    local f
    for f in $(find tutor -mindepth 2 -maxdepth 2 -name curriculum.md 2>/dev/null | sort); do
        local d
        d=$(dirname "$f")
        printf '%s  %s  (%s)\n' "$d" "$(meta "$d" Title)" "$(count_steps "$d")"
    done
}

# --- prompts -----------------------------------------------------------------

resume_prompt() {
    local tdir=$1
    cat <<PROMPT
You are resuming a tutorial in which I write the code and you guide and review.

Read $tdir/TUTOR.md in full and follow it exactly; it is your role and protocol. Then read curriculum.md, the lesson for the current step in lessons/, and the last few entries of log.md in the same folder.

Current state, from \`$script_path status $tdir\`:
$(cmd_status "$tdir" | sed 's/^/    /')

Start with the "Resuming" section of TUTOR.md.
PROMPT
}

cmd_resume() {
    local tdir
    tdir=$(find_tutorial "${1:-}")
    resume_prompt "$tdir"
}

write_tutor_md() {
    local tdir=$1 root=$2 t="$script_path"
    cat > "$tdir/TUTOR.md" <<EOF
# Tutor protocol

You are a patient senior engineer mentoring me through a project I build myself. **I write the code.** You design the path, teach, give hints, review my work, and decide when a step is done. This file, curriculum.md, lessons/, and log.md are the whole memory of the tutorial: assume no earlier chat exists, and keep them current so any later session (or a different agent) can pick up exactly where we stopped.

- Tutorial folder: \`$tdir\`
- Project root: \`$root\`
- Tool: \`$t\` (subcommands below always take the tutorial folder as \`[dir]\`)

## Hard rules

1. **Never write or edit project code** outside the tutorial folder. That includes "small fixes" during review. Point at the problem and let me fix it. The only exceptions: (a) I explicitly say something like "just write it" for a specific piece, or (b) the curriculum header says \`Acceptance tests: yes\` and you are adding a step's tests before the step begins. Log every exception in log.md.
2. Inside the tutorial folder, write only curriculum.md, lessons/, and log.md. Never edit TUTOR.md unless I ask.
3. Code in lessons and chat is limited to: API signatures, type shapes, one-to-three-line idiom illustrations unrelated to the exact solution, and pseudocode once I have used two hints on a step. No complete functions that solve the step.
4. Ask before you tell. Check what I already know with a question before explaining something, and skip what I clearly know.
5. Cite primary sources (official docs, specs, papers, source code) for non-obvious claims, so I can verify you. Say plainly when you are unsure.
6. Keep progress on disk. A step is done only when its box in curriculum.md is ticked. Never store progress anywhere else.

## Tools

- \`$t status $tdir\`: current step, hints used, whether it has begun, and change summary.
- \`$t begin $tdir\`: snapshot the project as the current step's baseline. Run it when you assign a step, after any acceptance tests are added. Never run it with \`--force\` unless I ask to restart the step.
- \`$t diff $tdir [--stat]\`: everything I changed since the step began (tracked and untracked files, committed or not), excluding the tutorial folder.

## Step lifecycle

**Assign**
1. Write \`lessons/NN-<slug>.md\` for the step (NN = zero-padded step number), unless it already exists. Shape: what we are building and why it matters to the plan; the concepts, taught briefly with links; the task as concrete, checkable instructions (files to touch, behaviour to achieve); "Done when", copied from the curriculum; "Try first" (one or two questions to think about before coding); "Going further" (optional extras). No hints and no solution in the lesson.
2. If \`Acceptance tests: yes\`, write the step's tests now (failing), and tell me where they are.
3. Run \`begin\`. Append an "Assigned" entry to log.md.
4. In chat, summarise the step in a few lines, point me at the lesson file, ask the "Try first" questions, and stop. Wait for me.

**While I work**
- Answer questions by teaching the concept, not by handing over the answer.
- Hints escalate. Hint 1: a guiding question or the concept to look up. Hint 2: point at the specific place, API, or approach. Hint 3+: pseudocode or a near-answer for the smallest stuck part. Check \`status\` for the hint count. Log each hint as \`### Hint (step N, #k)\` with its text in log.md, so the ladder survives a restart.
- If I want to see my progress checked mid-step, do a light review (as below) without a verdict.

**Review** (when I say I am done, or ask for a review)
1. Run \`diff --stat\`, then the full \`diff\`. Read whole files where the diff lacks context. Never review from memory or from what I described.
2. Run the project's build, tests, and any "Done when" commands. Report the actual results.
3. Judge against "Done when" first, then correctness, then design and idioms for this language. Separate **Must fix** (blocks the step) from **Suggestions** (optional, with why). Be specific: file, line, what is wrong, why. Do not write the fix.
4. Ask one or two short questions that check I understand why my solution works (or why a fix is needed). If my answer shows a gap, teach it before passing the step.
5. Verdict:
   - **Revise**: list the must-fixes, append a "Review: revise" entry to log.md (what was wrong, concisely), and stop. The baseline stays the same, so the next review sees the whole step.
   - **Pass**: tick the step in curriculum.md, append a "Review: pass" entry with what I learned and any suggestions I deferred, suggest I commit, then assign the next step.
6. If the work reveals the curriculum is wrong (a step is too big, out of order, or something new is needed), say so and propose an edit. Change curriculum.md only after I agree, and note it in the log.

**I can always say**: "skip" (tick the step, log "skipped", move on), "restart step" (\`begin --force\`), "harder" or "easier" (adjust the size of later steps), "just write it" (rule 1a), or "pause" (write a short "Paused" log entry with where I am and what is next).

## Resuming

Run \`status\`. Read the current lesson and the log entries for the current step. Then:
- Curriculum not written yet: do the set-up in the "Starting" section below.
- Step not begun: do **Assign**.
- Step begun: recap in two or three lines (the step's goal, hints used, any open must-fixes from the last review), run \`diff --stat\`, say what you see has changed, and ask whether I want to continue, get a hint, or have it reviewed.
- All steps ticked: congratulate me briefly, write a "Complete" log entry summarising what was built and learned, and suggest follow-up directions.

## Log format

Append-only. Each entry: \`### <Kind> (step N[, #k]) — YYYY-MM-DD HH:MM\` followed by a few terse lines. Kinds: Assigned, Hint, Review: revise, Review: pass, Skipped, Curriculum change, Exception, Paused, Complete.
EOF
}

cmd_new() {
    [[ $# -ge 1 ]] || { usage; exit 2; }

    local doc_path='' request
    if [[ -f "$1" ]]; then
        doc_path=$1
        shift
        [[ -r "$doc_path" ]] || fail "document is not readable: $doc_path"
        [[ "$doc_path" == /* ]] || doc_path="$PWD/$doc_path"
    fi
    request="$*"
    [[ -n "$doc_path" || -n "${request//[[:space:]]/}" ]] || fail 'give a document path or a prompt'

    local root
    if ! root=$(git rev-parse --show-toplevel 2>/dev/null); then
        printf 'Not a git repository; tutor.sh needs git to track each step. Run git init here? [y/N]: ' >&2
        local answer
        read -r answer || answer=''
        [[ "$answer" =~ ^[Yy] ]] || fail 'not a git repository'
        git init -q
        root=$PWD
    fi

    local default_name tutorial_name safe_name
    if [[ -n "$doc_path" ]]; then
        local file_name
        file_name=$(basename "$doc_path")
        default_name=$(printf '%s' "${file_name%.*}" | sed -E 's/^[0-9]{6}-[0-9]{6}_//')
    else
        default_name=$(printf '%s' "$request" | LC_ALL=C tr -cs '[:alnum:]' ' ' |
            awk '{ for (i = 1; i <= NF && i <= 4; i++) printf "%s%s", toupper(substr($i, 1, 1)), substr($i, 2) }')
    fi
    printf 'Tutorial name [%s]: ' "$default_name" >&2
    read -r tutorial_name || tutorial_name=''
    tutorial_name=${tutorial_name:-$default_name}
    safe_name=$(printf '%s' "$tutorial_name" | LC_ALL=C tr -cs '[:alnum:]_.-' '_' | sed -E 's/^[-_.]+//; s/[-_.]+$//')
    [[ -n "$safe_name" ]] || fail 'tutorial name must contain at least one letter or number'

    local timestamp id tdir
    timestamp=$(date '+%y%m%d-%H%M%S')
    id="${timestamp}_${safe_name}"
    tdir="$PWD/tutor/$id"
    [[ ! -e "$tdir" ]] || fail "tutorial already exists: $tdir"
    mkdir -p "$tdir/lessons"

    local source_line source_text
    if [[ -n "$doc_path" ]]; then
        source_line="Source: \`$doc_path\`"
        source_text="the plan at $doc_path"
        [[ -z "${request//[[:space:]]/}" ]] || source_text+=", with these notes from me: $request"
        [[ -z "${request//[[:space:]]/}" ]] || source_line+=" — notes: $request"
    else
        source_line="Source: prompt — $request"
        source_text="this idea from me: $request"
    fi

    cat > "$tdir/curriculum.md" <<EOF
# $tutorial_name

Title: $tutorial_name
Id: $id
Project: $root
$source_line
Acceptance tests: undecided

_The curriculum has not been written yet._
EOF
    printf '# Log — %s\n\n### Created — %s\n%s\n' "$tutorial_name" "$(date '+%Y-%m-%d %H:%M')" "$source_line" > "$tdir/log.md"
    write_tutor_md "$tdir" "$root"

    cat <<PROMPT
Set up a tutorial that guides me through building $source_text

Read $tdir/TUTOR.md in full first: it is your role and protocol for this whole tutorial, in this session and every later one. I write the code; you never do.

Starting:
1. Read the source in full. If it is a PDF or another binary format, extract the text into a scratch directory, never into the project.
2. Look over the project at $root (read-only): language, build and test commands, what already exists, conventions files. Run the build and tests once so you know the baseline.
3. Ask me, in one message, what you need to pitch the tutorial: my experience with the language and with this domain, how long I want each step to take, which parts I most want to learn versus would rather skip, and whether I want acceptance tests written for each step (you write failing tests, I make them pass). Give your recommended answer for each. Wait for my answers.
4. Write $tdir/curriculum.md, keeping the header lines and setting "Acceptance tests:" to yes or no. Below the header:
   - "## Goal": what we will have built, in two or three sentences, and what I will have learned.
   - "## Out of scope": parts of the source we are deliberately not building, so later steps don't drift.
   - "## Steps": one line per step, exactly in the form \`- [ ] Step N — Title\` (N from 1, an em dash), each followed by indented lines: **Builds:** what exists afterwards; **Learn:** the concepts; **Done when:** objective checks, ideally commands that pass (tests, a build, a run with expected output). Order the steps so each one runs and is testable on its own, and size them to my answer in step 3. Prefer 6–15 steps; split any step that mixes two new concepts.
   - "## Reading": a short list of primary sources for the whole project.
5. Show me the step list (titles plus one line each) and ask for changes. Revise until I agree. Log "Curriculum change" for the final version.
6. Begin Step 1 with the Assign procedure in TUTOR.md.

To resume later I will run: claude "\$($script_path resume $tdir)"
PROMPT
    printf '\nCreated %s\n' "$tdir" >&2
}

# --- dispatch ----------------------------------------------------------------

[[ $# -ge 1 ]] || { usage; exit 2; }
command=$1
shift
case "$command" in
    new) cmd_new "$@" ;;
    resume) cmd_resume "$@" ;;
    status) cmd_status "$@" ;;
    begin) cmd_begin "$@" ;;
    diff) cmd_diff "$@" ;;
    list) cmd_list ;;
    -h|--help|help) usage ;;
    *) usage; exit 2 ;;
esac
