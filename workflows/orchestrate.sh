#!/usr/bin/env bash
# Multi-model build orchestrator. A strong "brain" model plans and reviews, a
# mid-tier "taskmaster" verifies work and dispatches it, and cheap "workers"
# implement it.
#
#   role         claude              codex
#   brain        claude-fable-5-1    gpt-6-astra
#   taskmaster   claude-opus-5-5     gpt-6.1-sol
#   worker       claude-sonnet-5-5   gpt-6-luna
#
# Flow:
#   1. You and the brain agree on milestones in an interactive session.
#   2. For each milestone, the brain writes tasks.md. The script then fans out
#      workers, and after each batch the taskmaster verifies the results and
#      picks the next batch, until it reports ALL_CLEAR. The script commits the
#      milestone.
#   3. The brain reviews the whole branch. A CHANGES verdict turns its fix tasks
#      into another build loop and a re-review. After MAX_REVIEWS reviews, the
#      last fixes are built and verified but not re-reviewed, and the run ends.
#
# Token budget: every agent call is a fresh, single-purpose session with a
# minimal tool set and no MCP servers or skills. Roles share state only through
# short files. The brain never reads worker transcripts, the taskmaster keeps a
# ledger instead of a growing conversation, and workers read only their own
# task section and the files it lists. Round 1 is dispatched by the script, and
# re-reviews only read the fix diff. Per-call usage is logged to usage.tsv.
#
# State lives in .tmp/orchestrate/<timestamp>_<slug>/, which is added to
# .git/info/exclude when not already ignored. Every step can be resumed. When a
# run stops as BLOCKED, write your decision in HUMAN_NOTES.md in the run
# folder, then resume.
#
# Usage:
#   workflows/orchestrate.sh                 start a new run
#   workflows/orchestrate.sh --resume [dir]  continue a run (default: latest)
#
# Settings are saved in the run's config.env. ORCH_<NAME> env vars override
# them, for example ORCH_WORKER_MODEL, ORCH_PARALLEL, ORCH_MAX_ROUNDS and
# ORCH_MAX_REVIEWS.

set -euo pipefail

SETTINGS='PROVIDER BRAIN_MODEL TASK_MODEL WORKER_MODEL PLAN_EFFORT REVIEW_EFFORT TASK_EFFORT WORKER_EFFORT PARALLEL MAX_ROUNDS MAX_REVIEWS CLAUDE_PERMS CODEX_SANDBOX'
CLAUDE_READONLY='Read,Grep,Glob,Bash(git log *),Bash(git diff *),Bash(git show *),Bash(git status *),Bash(ls *)'

TASK_FORMAT='## T1: <short title>
Files: <paths this task creates or edits>
Read first: <the few paths or symbols a worker needs, nothing more>
Depends on: none | T<n>, ...
Do: <precise instructions, including any names, signatures or contracts that other tasks rely on>
Acceptance:
- <checkable criterion>
Verify: <one fast command scoped to this task, or "manual: <what to inspect>">'

say() { printf '\033[1m[orchestrate]\033[0m %s\n' "$*"; }
die() { printf '[orchestrate] error: %s\n' "$*" >&2; exit 1; }

ask() {
    local answer
    printf '%s' "$1"
    IFS= read -r answer || die 'input closed'
    REPLY=$answer
}

# --- settings and run setup --------------------------------------------------

# Priority: ORCH_* env vars, then config.env (on resume), then provider defaults.
load_settings() {
    local name override
    for name in $SETTINGS; do
        override="ORCH_$name"
        if [ -n "${!override:-}" ]; then printf -v "$name" '%s' "${!override}"; fi
    done
    case "$PROVIDER" in
        claude) : "${BRAIN_MODEL:=claude-fable-5-1}" "${TASK_MODEL:=claude-opus-5-5}" "${WORKER_MODEL:=claude-sonnet-5-5}" ;;
        codex)  : "${BRAIN_MODEL:=gpt-6-astra}" "${TASK_MODEL:=gpt-6.1-sol}" "${WORKER_MODEL:=gpt-6-luna}" ;;
        *) die "unknown provider: $PROVIDER" ;;
    esac
    : "${PLAN_EFFORT:=high}" "${REVIEW_EFFORT:=high}" "${TASK_EFFORT:=medium}" "${WORKER_EFFORT:=medium}"
    : "${PARALLEL:=4}" "${MAX_ROUNDS:=10}" "${MAX_REVIEWS:=3}"
    : "${CLAUDE_PERMS:=--permission-mode auto}" "${CODEX_SANDBOX:=workspace-write}"
}

save_config() {
    local name
    for name in $SETTINGS BASE_SHA BRANCH; do
        printf '%s=%q\n' "$name" "${!name}"
    done > "$RUN_DIR/config.env"
}

exclude_tmp() {
    local exclude
    git check-ignore -q .tmp/orchestrate/probe && return 0
    exclude=$(git rev-parse --git-path info/exclude)
    mkdir -p "$(dirname "$exclude")"
    printf '.tmp/\n' >> "$exclude"
    say "Added .tmp/ to $exclude so run files stay out of commits"
}

new_run() {
    local task slug ts
    PROVIDER=${ORCH_PROVIDER:-}
    while [ "$PROVIDER" != claude ] && [ "$PROVIDER" != codex ]; do
        ask 'Provider [claude/codex]: '
        PROVIDER=$REPLY
    done
    load_settings
    command -v "$PROVIDER" > /dev/null || die "$PROVIDER CLI not found"

    ask 'Task (one line, or a path to a spec file): '
    task=$REPLY
    [ -n "${task// /}" ] || die 'no task given'
    if [ -f "$task" ]; then slug=$(basename "$task"); slug=${slug%.*}; else slug=$task; fi
    slug=$(printf '%s' "$slug" | tr '[:upper:]' '[:lower:]' | tr -cs 'a-z0-9' '-' | cut -c1-40 | sed 's/^-*//; s/-*$//')
    [ -n "$slug" ] || slug=task

    exclude_tmp
    if [ -n "$(git status --porcelain)" ]; then
        ask 'Working tree has uncommitted changes; they will land in the first commit. Continue? [y/N] '
        case "$REPLY" in y|Y) ;; *) exit 1 ;; esac
    fi

    ts=$(date +%Y%m%d-%H%M%S)
    RUN_DIR=.tmp/orchestrate/${ts}_$slug
    mkdir -p "$RUN_DIR"
    if [ -f "$task" ]; then cp "$task" "$RUN_DIR/task.md"; else printf '%s\n' "$task" > "$RUN_DIR/task.md"; fi
    printf 'role\tstep\tmodel\tinput\tcached_input\toutput\tcost_usd\n' > "$RUN_DIR/usage.tsv"

    BASE_SHA=$(git rev-parse HEAD)
    BRANCH=orch/$slug
    if git show-ref --verify --quiet "refs/heads/$BRANCH"; then BRANCH=orch/$slug-$ts; fi
    git switch -q -c "$BRANCH"
    save_config
    say "Run $RUN_DIR on branch $BRANCH ($PROVIDER)"
}

resume_run() {
    RUN_DIR=${1:-}
    if [ -z "$RUN_DIR" ]; then
        RUN_DIR=$(ls -d .tmp/orchestrate/*/ 2> /dev/null | sort | tail -n 1 || true)
    fi
    RUN_DIR=${RUN_DIR%/}
    [ -f "$RUN_DIR/config.env" ] || die "no run found at '$RUN_DIR'"
    . "$RUN_DIR/config.env"
    load_settings
    if [ -n "$(git status --porcelain)" ]; then
        say 'Uncommitted changes, which will be included in the next commit:'
        git status --short
        ask 'Continue? [y/N] '
        case "$REPLY" in y|Y) ;; *) exit 1 ;; esac
    fi
    if [ "$(git branch --show-current)" != "$BRANCH" ]; then
        git switch -q "$BRANCH" || die "could not switch to $BRANCH"
    fi
    say "Resuming $RUN_DIR on branch $BRANCH ($PROVIDER)"
}

# --- agent calls ---------------------------------------------------------------

# run_agent ROLE EFFORT STEP PROMPT OUT LOG
# One headless call. The final message goes to OUT, written atomically, and
# usage goes to usage.tsv. Fails when the CLI exits non-zero or reports an
# error. The brain runs read-only and returns its work as the final message.
run_agent() {
    local role=$1 effort=$2 step=$3 prompt=$4 out=$5 log=$6 model status=0 sandbox
    local -a access
    case "$role" in
        brain) model=$BRAIN_MODEL ;;
        taskmaster) model=$TASK_MODEL ;;
        worker) model=$WORKER_MODEL ;;
    esac

    if [ "$PROVIDER" = claude ]; then
        if [ "$role" = brain ]; then
            access=(--tools Read,Grep,Glob,Bash --permission-mode dontAsk --allowedTools "$CLAUDE_READONLY")
        else
            access=(--tools Read,Grep,Glob,Bash,Edit,Write $CLAUDE_PERMS)
        fi
        claude -p --model "$model" --effort "$effort" --strict-mcp-config --disable-slash-commands \
            --no-session-persistence "${access[@]}" --output-format json -- "$prompt" \
            < /dev/null > "$log.json" 2> "$log" || status=$?
        if [ "$(jq -r '.is_error // false' "$log.json" 2> /dev/null)" = true ]; then status=1; fi
        jq -r '.result // empty' "$log.json" > "$out.part" 2> /dev/null || true
        jq -r --arg r "$role" --arg s "$step" --arg m "$model" \
            '[$r, $s, $m, .usage.input_tokens + .usage.cache_creation_input_tokens,
              .usage.cache_read_input_tokens, .usage.output_tokens, .total_cost_usd] | @tsv' \
            "$log.json" >> "$RUN_DIR/usage.tsv" 2> /dev/null || true
    else
        sandbox=$CODEX_SANDBOX
        if [ "$role" = brain ]; then sandbox=read-only; fi
        codex exec -m "$model" -c "model_reasoning_effort=\"$effort\"" -s "$sandbox" --ephemeral --json \
            -o "$out.part" "$prompt" < /dev/null > "$log.jsonl" 2> "$log" || status=$?
        if [ "$(jq -s 'any(.[]; .type == "turn.failed" or .type == "error")' "$log.jsonl" 2> /dev/null)" = true ]; then status=1; fi
        jq -rs --arg r "$role" --arg s "$step" --arg m "$model" \
            '[.[] | select(.type == "turn.completed") | .usage] as $u
             | [$r, $s, $m, ($u | map(.input_tokens - .cached_input_tokens) | add),
                ($u | map(.cached_input_tokens) | add), ($u | map(.output_tokens) | add), ""] | @tsv' \
            "$log.jsonl" >> "$RUN_DIR/usage.tsv" 2> /dev/null || true
    fi
    if [ -f "$out.part" ]; then mv "$out.part" "$out"; fi
    return "$status"
}

run_brain_interactive() {
    if [ "$PROVIDER" = claude ]; then
        claude --model "$BRAIN_MODEL" --effort "$PLAN_EFFORT" -- "$1" || true
    else
        codex -m "$BRAIN_MODEL" -c "model_reasoning_effort=\"$PLAN_EFFORT\"" "$1" || true
    fi
}

human_notes() {
    if [ -s "$RUN_DIR/HUMAN_NOTES.md" ]; then
        printf '\nHuman notes, which override anything else: %s\n' "$RUN_DIR/HUMAN_NOTES.md"
    fi
}

# --- prompts -------------------------------------------------------------------

planning_prompt() {
    local revise=''
    if [ -s "$RUN_DIR/milestones.md" ]; then
        revise=" $RUN_DIR/milestones.md already exists: revise it with the user instead of starting over."
    fi
    cat << EOF
You are the planning lead for a multi-model build. Cheaper models will carry out the work later, so your job here is the thinking: understand the request, look at only as much of the repo as you need, and agree on milestones with the user.

The request is in $RUN_DIR/task.md.$revise
$(human_notes)
1. Ask the user only the questions that would change the plan (scope, constraints, what "done" means). Keep it brief.
2. Write $RUN_DIR/milestones.md in this shape:

# <Title>
## Goal
<2-4 sentences>
## Constraints and non-goals
- <item>
## M1: <short title>
Outcome: <what exists when it is done>
Done when: <observable checks, ideally commands>
## M2: <short title>
...

Milestones run in order. Each one leaves the repo working and is small enough to split into roughly 3-8 tasks, as many of them parallel as possible. Do not write any code.
3. Once the user agrees, tell them to exit this session (/exit) so the script can take over.
EOF
}

milestone_plan_prompt() {
    local mid=$1 title=$2 built=$3
    cat << EOF
You are the planning lead for one milestone of a multi-model build. Your final message is saved verbatim as the milestone's tasks.md, so output only the task plan in Markdown, with no preamble.

Overall plan: $RUN_DIR/milestones.md
Milestone: $mid: $title
Already built: $built (commits since $BASE_SHA). Look at existing code only where this milestone touches it.
$(human_notes)
Split the milestone into tasks for workers. A worker is a cheaper model that sees only its own task section and the files that section lists, so each section has to stand on its own. Use exactly this format, numbering T1, T2, ...:

$TASK_FORMAT

Rules:
- Let as many tasks as possible run in parallel. Tasks that can run at the same time must not share files.
- Put shared contracts (types, interfaces, schemas) in an early task that the others depend on.
- Keep each task to one focused change, with its tests in the same task.
- Every task must change files. Do not add verification-only tasks: the taskmaster runs each Verify command and the milestone's "Done when" checks.
- Keep "Read first" short: it decides how much each worker reads.
EOF
}

worker_prompt() {
    local unit=$1 rd=$2 id=$3 notes=''
    if [ -s "$rd/notes/$id.md" ]; then
        notes=" Read $rd/notes/$id.md first: it has the taskmaster's feedback on an earlier attempt."
    fi
    cat << EOF
You are a worker implementing exactly one task: the "## $id:" section of $unit/tasks.md. Read only that section, the files it lists under "Read first", and the files you edit. Avoid broad exploration.$notes
$(human_notes)
- Edit only the files listed for your task. If something else must change, keep the change minimal and flag it.
- Other workers are editing other files at the same time. Do not run repo-wide formatters or code generators, and do not touch git (no add, commit, stash, checkout or reset).
- When you are done, run the task's Verify command. If it fails, fix the problem and run it again.

Your final message is your report. Keep it under 15 lines:
STATUS: DONE | FAILED
Changed: <files>
Verify: <command> -> <pass or fail, plus the one key output line>
Notes: <deviations, extra files you touched, open concerns, or "none">
EOF
}

taskmaster_prompt() {
    local unit=$1 label=$2 kind=$3 rd=$4 prev=$5 resumed=$6 rejected=$7 last done_when=''
    if [ -n "$prev" ]; then
        last="$prev ($prev/dispatch.txt lists what ran; $prev/reports/ has one report per task)"
    else
        last='none, this is the first round'
    fi
    if [ "$kind" = milestone ]; then
        done_when=", and the milestone's \"Done when\" checks in $RUN_DIR/milestones.md pass"
    fi
    if [ -n "$resumed" ]; then
        resumed="
The last round was BLOCKED, and a human has since resumed the run. Their decision is in the human notes."
    fi
    if [ -n "$rejected" ]; then
        rejected="
Your previous dispatch was rejected by the script: $rejected. Correct the ledger or the batch and write the dispatch again."
    fi
    cat << EOF
You are the taskmaster for $label. You verify worker output and choose the next batch. You never write product code: fixes go back to workers through notes.

Plan: $unit/tasks.md
Ledger: $unit/ledger.md. This is your only memory between rounds. Create it if it is missing, with one line per task: "<id> | <status> | <short note>", where status is exactly one of pending, dispatched, verified, or rejected x<n>.
Last round: $last$resumed$rejected
$(human_notes)
Keep it cheap. Read the ledger and the latest reports. For the files those tasks own, check git status --short -- <files> and git diff HEAD -- <files>, and read any new untracked files in full. Then run each task's Verify command. Do not re-check tasks the ledger already marks verified.

1. For each task in the last round, check its Acceptance criteria and run its Verify command. Mark it verified, or rejected with the reason. A task rejected 3 times needs a human, so stop with BLOCKED.
2. Choose the next batch: tasks not yet verified whose dependencies are all verified, at most $PARALLEL, with no files shared between them. For every rejected task you send again, write $rd/notes/<id>.md saying what failed and what to change, in 10 lines or fewer.
3. Update the ledger. Then write $rd/dispatch.tmp containing exactly one of:
   - one "RUN <id>" line for each task in the batch
   - ALL_CLEAR, only when every task is verified$done_when, and git status shows no stray files such as build artifacts or caches (delete them, or add them to .gitignore)
   - BLOCKED: <one-line reason>, when a human decision is needed
Write nothing else to that file. The script checks it against the plan and the ledger before running anything. End with a one-line summary of the round.
EOF
}

review_prompt() {
    local n=$1 rv=$2 scope
    if [ "$n" -eq 1 ]; then
        scope="$rv/diff.patch is the whole branch."
    else
        scope="This is re-review $n. $rv/diff.patch covers only the fixes made since $RUN_DIR/review-$((n - 1))/review.md: check that its fix tasks were done properly and introduced no new problems. Use git diff $BASE_SHA..HEAD only if you need wider context."
    fi
    cat << EOF
You are the lead reviewer for a branch that cheaper models built from the plan in $RUN_DIR/milestones.md. A taskmaster has already checked every task against its own acceptance criteria. Your job is what that misses: does the branch as a whole deliver the plan, correctly and coherently?

$scope Start with $rv/diffstat.txt, read the parts of the diff that matter (skip lock files and generated code), and open source files only where the diff is not enough.
$(human_notes)
Look for gaps against the plan, correctness bugs and edge cases, broken contracts between tasks, security or data-safety problems, missing or weak tests, and needless complexity. Ignore style preferences.

Your final message is saved verbatim as review.md. Output only this Markdown:

## Summary
<3-6 lines>
## Findings
- path:line (Blocker | Risk | Suggestion) <issue and fix>
## Fix tasks
<Only when the verdict is CHANGES: tasks for workers, numbered F1, F2, ... instead of T1, T2, in exactly this format. Each must stand on its own.>

$TASK_FORMAT

The last line must be exactly "VERDICT: APPROVED" or "VERDICT: CHANGES". Only Blockers and real Risks justify CHANGES. List Suggestions without blocking on them.
EOF
}

# --- phases --------------------------------------------------------------------

has_milestones() { grep -qE '^## M[0-9]+:' "$RUN_DIR/milestones.md" 2> /dev/null; }

plan_milestones() {
    [ -f "$RUN_DIR/milestones.approved" ] && return 0
    if [ ! -s "$RUN_DIR/milestones.md" ]; then
        say "Opening a planning session with $BRAIN_MODEL. Exit it once the milestones are agreed."
        run_brain_interactive "$(planning_prompt)"
    fi
    while :; do
        if has_milestones; then
            say "Milestones in $RUN_DIR/milestones.md:"
            grep -E '^## M[0-9]+:' "$RUN_DIR/milestones.md" | sed 's/^## /  /'
            ask '[a]pprove and build, [e]dit, [r]eopen planning, [q]uit: '
        else
            ask "No milestones in $RUN_DIR/milestones.md. [r]eopen planning, [q]uit: "
        fi
        case "$REPLY" in
            a|A) if has_milestones; then touch "$RUN_DIR/milestones.approved"; return 0; fi ;;
            e|E) "${EDITOR:-vi}" "$RUN_DIR/milestones.md" ;;
            r|R) run_brain_interactive "$(planning_prompt)" ;;
            q|Q) say "Resume later with: $0 --resume $RUN_DIR"; exit 0 ;;
        esac
    done
}

# task_table TASKS: one "id<TAB>deps<TAB>files" line per task, with deps and
# files comma-separated, or "-" when there are none.
task_table() {
    awk '
        function flush() { if (id != "") print id "\t" (deps == "" ? "-" : deps) "\t" (files == "" ? "-" : files) }
        function field(s) {
            sub(/^[A-Za-z ]+:/, "", s)
            gsub(/`/, "", s)
            gsub(/[[:space:]]*\([^)]*\)/, "", s)
            gsub(/[[:space:]]*,[[:space:]]*/, ",", s)
            sub(/^[[:space:]]+/, "", s)
            sub(/[[:space:].]+$/, "", s)
            return s ~ /^(none|None|-)/ ? "" : s
        }
        /^## [A-Z][0-9]+:/ { flush(); id = $2; sub(/:$/, "", id); deps = ""; files = ""; next }
        id != "" && /^Depends on:/ { deps = field($0) }
        id != "" && /^Files:/ { files = field($0) }
        END { flush() }' "$1"
}

# check_dispatch TASKS LEDGER DISPATCH: print the problem and fail unless
# DISPATCH is exactly ALL_CLEAR (with every task verified in LEDGER), one
# "BLOCKED: <reason>" line, or 1..PARALLEL "RUN <id>" lines naming distinct,
# unverified, known tasks whose dependencies are verified and whose files
# don't overlap.
check_dispatch() {
    task_table "$1" | awk -F'\t' -v max="$PARALLEL" -v ledger="$2" -v dispatch="$3" '
        { order[++tasks] = $1; known[$1] = 1; deps[$1] = $2; files[$1] = $3 }
        END {
            while ((getline line < ledger) > 0) {
                split(line, f, /[[:space:]]*\|[[:space:]]*/)
                sub(/^[[:space:]]+/, "", f[1])
                if (f[2] ~ /^verified/) verified[f[1]] = 1
            }
            while ((getline line < dispatch) > 0) {
                sub(/[[:space:]]+$/, "", line)
                if (line == "") continue
                lines++
                if (line == "ALL_CLEAR" || line ~ /^BLOCKED: ./) { terminal = line; continue }
                if (line !~ /^RUN [A-Z][0-9]+$/) { print "unexpected line \"" line "\""; exit 1 }
                id = substr(line, 5)
                if (!(id in known)) { print "unknown task " id; exit 1 }
                if (id in batch) { print "duplicate task " id; exit 1 }
                if (id in verified) { print id " is already verified"; exit 1 }
                batch[id] = 1
                count++
                n = split(deps[id], d, ",")
                for (i = 1; i <= n; i++)
                    if (d[i] != "-" && !(d[i] in verified)) { print id " depends on unverified " d[i]; exit 1 }
                n = split(files[id], fl, ",")
                for (i = 1; i <= n; i++) {
                    if (fl[i] == "-") continue
                    if (fl[i] in owner) { print id " and " owner[fl[i]] " both edit " fl[i]; exit 1 }
                    owner[fl[i]] = id
                }
            }
            if (lines == 0) { print "no dispatch written"; exit 1 }
            if (terminal != "" && lines > 1) { print terminal " must be the only line"; exit 1 }
            if (count > max) { print count " tasks exceeds the limit of " max; exit 1 }
            if (terminal == "ALL_CLEAR")
                for (i = 1; i <= tasks; i++)
                    if (!(order[i] in verified)) { print "ALL_CLEAR but " order[i] " is not verified in the ledger"; exit 1 }
        }'
}

# Tasks with no dependencies and no shared files, so round 1 doesn't need a
# taskmaster call.
initial_dispatch() {
    task_table "$1" | awk -F'\t' -v max="$PARALLEL" '
        $2 == "-" && picked < max {
            n = split($3, fl, ",")
            for (i = 1; i <= n; i++) if (fl[i] != "-" && (fl[i] in taken)) next
            for (i = 1; i <= n; i++) taken[fl[i]] = 1
            print "RUN " $1
            picked++
        }'
}

# run_taskmaster UNIT LABEL KIND RD PREV RESUMED: publishes RD/dispatch.txt only
# after the call succeeds and the dispatch validates, with one retry.
run_taskmaster() {
    local unit=$1 label=$2 kind=$3 rd=$4 prev=$5 resumed=$6 attempt problem='' status
    for attempt in 1 2; do
        say "taskmaster: $label, round $(basename "$rd" | sed 's/round-//') ($TASK_MODEL)"
        rm -f "$rd/dispatch.tmp"
        status=0
        run_agent taskmaster "$TASK_EFFORT" "$(basename "$unit")-$(basename "$rd")-taskmaster-$attempt" \
            "$(taskmaster_prompt "$unit" "$label" "$kind" "$rd" "$prev" "$resumed" "$problem")" \
            "$rd/taskmaster.md" "$rd/logs/taskmaster-$attempt.log" || status=$?
        [ "$status" -eq 0 ] || die "taskmaster call failed (exit $status); see $rd/logs/taskmaster-$attempt.log, then --resume"
        if problem=$(check_dispatch "$unit/tasks.md" "$unit/ledger.md" "$rd/dispatch.tmp"); then
            mv "$rd/dispatch.tmp" "$rd/dispatch.txt"
            [ -s "$rd/taskmaster.md" ] && say "  $(tail -n 1 "$rd/taskmaster.md")"
            return 0
        fi
        say "  dispatch rejected: $problem"
    done
    stop_blocked "$label: the taskmaster wrote an invalid dispatch twice ($problem). See $rd."
}

run_workers() {
    local unit=$1 rd=$2 step=$3 id report prompt pids=''
    for id in $(sed -n 's/^RUN \([A-Z][0-9]*\)$/\1/p' "$rd/dispatch.txt"); do
        report=$rd/reports/$id.md
        [ -s "$report" ] && continue
        say "  worker $id ($WORKER_MODEL)"
        prompt=$(worker_prompt "$unit" "$rd" "$id")
        (
            status=0
            run_agent worker "$WORKER_EFFORT" "$step-$id" "$prompt" "$report.tmp" "$rd/logs/$id.log" || status=$?
            if [ "$status" -ne 0 ] || [ ! -s "$report.tmp" ]; then
                {
                    printf 'STATUS: FAILED (worker call exited %s; log: %s)\n' "$status" "$rd/logs/$id.log"
                    cat "$report.tmp" 2> /dev/null || true
                } > "$report.failed"
                mv "$report.failed" "$report.tmp"
            fi
            mv "$report.tmp" "$report"
        ) &
        pids="$pids $!"
    done
    for id in $pids; do wait "$id" || true; done
}

stop_blocked() {
    say "Stopped: $1"
    say "Write your decision in $RUN_DIR/HUMAN_NOTES.md, then run: $0 --resume $RUN_DIR"
    print_usage
    exit 2
}

# build_unit DIR LABEL KIND: worker/taskmaster rounds until ALL_CLEAR.
build_unit() {
    local unit=$1 label=$2 kind=$3 round=0 rd prev resumed=''
    while [ -d "$unit/round-$((round + 1))" ]; do round=$((round + 1)); done
    if [ "$round" -eq 0 ]; then
        round=1
    elif grep -q '^BLOCKED: ' "$unit/round-$round/dispatch.txt" 2> /dev/null; then
        resumed=1
        round=$((round + 1))
    fi

    while :; do
        [ "$round" -le "$MAX_ROUNDS" ] || stop_blocked "$label hit MAX_ROUNDS ($MAX_ROUNDS). Raise ORCH_MAX_ROUNDS to continue."
        rd=$unit/round-$round
        prev=''
        if [ "$round" -gt 1 ]; then prev=$unit/round-$((round - 1)); fi
        mkdir -p "$rd/reports" "$rd/notes" "$rd/logs"

        if [ ! -s "$rd/dispatch.txt" ] && [ "$round" -eq 1 ]; then
            initial_dispatch "$unit/tasks.md" > "$rd/dispatch.tmp" || true
            if check_dispatch "$unit/tasks.md" "$unit/ledger.md" "$rd/dispatch.tmp" > /dev/null; then
                mv "$rd/dispatch.tmp" "$rd/dispatch.txt"
            fi
        fi
        if [ ! -s "$rd/dispatch.txt" ]; then
            run_taskmaster "$unit" "$label" "$kind" "$rd" "$prev" "$resumed"
        fi
        resumed=''

        case "$(head -n 1 "$rd/dispatch.txt")" in
            ALL_CLEAR) return 0 ;;
            BLOCKED:*) stop_blocked "$label: $(head -n 1 "$rd/dispatch.txt"). Ledger: $unit/ledger.md" ;;
        esac
        run_workers "$unit" "$rd" "$(basename "$unit")-r$round"
        round=$((round + 1))
    done
}

commit_unit() {
    local unit=$1 message=$2
    git add -A
    if git diff --cached --quiet; then
        say "$message: nothing to commit"
    else
        git commit -q -m "$message" || die "commit failed. Commit by hand, then: touch $unit/DONE && $0 --resume $RUN_DIR"
        say "Committed: $message"
    fi
    touch "$unit/DONE"
}

build_milestones() {
    local i=0 mid title unit line built='nothing yet'
    MIDS=()
    TITLES=()
    while IFS= read -r line; do
        MIDS+=("${line%%|*}")
        TITLES+=("${line#*|}")
    done < <(grep -E '^## M[0-9]+:' "$RUN_DIR/milestones.md" | sed -E 's/^## (M[0-9]+):[[:space:]]*/\1|/')

    while [ "$i" -lt "${#MIDS[@]}" ]; do
        mid=${MIDS[$i]}
        title=${TITLES[$i]}
        unit=$RUN_DIR/$mid
        if [ ! -f "$unit/DONE" ]; then
            mkdir -p "$unit"
            if [ ! -s "$unit/tasks.md" ]; then
                say "brain: planning $mid: $title ($BRAIN_MODEL)"
                run_agent brain "$PLAN_EFFORT" "$mid-plan" "$(milestone_plan_prompt "$mid" "$title" "$built")" \
                    "$unit/tasks.md.tmp" "$unit/plan.log" || die "planning call for $mid failed; see $unit/plan.log, then --resume"
                grep -qE '^## T[0-9]+:' "$unit/tasks.md.tmp" 2> /dev/null || die "no tasks planned for $mid; see $unit/plan.log"
                mv "$unit/tasks.md.tmp" "$unit/tasks.md"
                say "  $(grep -cE '^## T[0-9]+:' "$unit/tasks.md") tasks in $unit/tasks.md"
            fi
            build_unit "$unit" "milestone $mid ($title)" milestone
            commit_unit "$unit" "$mid: $title"
        fi
        if [ "$built" = 'nothing yet' ]; then built=$mid; else built="$built, $mid"; fi
        i=$((i + 1))
    done
}

# Reviews and fix rounds until APPROVED. At MAX_REVIEWS, the last review's fix
# tasks are still built and verified by the taskmaster, and the run finishes
# with a note that the brain has not re-reviewed them.
review_loop() {
    local n=1 rv since verdict unit
    while :; do
        rv=$RUN_DIR/review-$n
        unit=$RUN_DIR/fix-$n
        # A review is stale when the branch moved without its own fix round.
        if [ -s "$rv/review.md" ] && [ ! -f "$unit/DONE" ] && [ "$(cat "$rv/head.txt")" != "$(git rev-parse HEAD)" ]; then
            say "Review $n is stale (the branch has moved since); reviewing again"
            n=$((n + 1))
            continue
        fi
        if [ ! -s "$rv/review.md" ]; then
            [ -z "$(git status --porcelain)" ] || stop_blocked "uncommitted changes before review $n. Commit or discard them first."
            mkdir -p "$rv"
            since=$BASE_SHA
            if [ "$n" -gt 1 ]; then since=$(cat "$RUN_DIR/review-$((n - 1))/head.txt"); fi
            git rev-parse HEAD > "$rv/head.txt"
            git diff --stat "$since"..HEAD > "$rv/diffstat.txt"
            git diff "$since"..HEAD > "$rv/diff.patch"
            say "brain: reviewing the branch, review $n ($BRAIN_MODEL)"
            run_agent brain "$REVIEW_EFFORT" "review-$n" "$(review_prompt "$n" "$rv")" \
                "$rv/review.md.tmp" "$rv/review.log" || die "review $n call failed; see $rv/review.log, then --resume"
            grep -qE '^VERDICT: (APPROVED|CHANGES)$' "$rv/review.md.tmp" || die "review $n has no valid VERDICT line; see $rv/review.md.tmp"
            mv "$rv/review.md.tmp" "$rv/review.md"
        fi

        verdict=$(grep -E '^VERDICT: (APPROVED|CHANGES)$' "$rv/review.md" | tail -n 1 | cut -d' ' -f2 || true)
        say "Review $n: $verdict ($rv/review.md)"
        [ "$verdict" = APPROVED ] && return 0

        if [ ! -f "$unit/DONE" ]; then
            mkdir -p "$unit"
            awk '/^## Fix tasks/ { f = 1; next } /^VERDICT:/ { f = 0 } f' "$rv/review.md" > "$unit/tasks.md"
            grep -qE '^## F[0-9]+:' "$unit/tasks.md" || stop_blocked "review $n wants changes but lists no fix tasks."
            build_unit "$unit" "fixes from review $n" fix
            commit_unit "$unit" "Fixes from review $n"
        fi
        if [ "$n" -ge "$MAX_REVIEWS" ]; then
            FINAL_NOTE="Fixes from review $n were verified by the taskmaster but not re-reviewed by the brain (MAX_REVIEWS=$MAX_REVIEWS)."
            return 0
        fi
        n=$((n + 1))
    done
}

# One orchestrator per repository (shared across worktrees). A lock left by a
# process that no longer exists is taken over.
acquire_lock() {
    local pid
    LOCK=$(cd "$(git rev-parse --git-common-dir)" && pwd)/orchestrate.lock
    if ! mkdir "$LOCK" 2> /dev/null; then
        pid=$(cat "$LOCK/pid" 2> /dev/null || true)
        if [ -n "$pid" ] && kill -0 "$pid" 2> /dev/null; then
            die "another orchestrate run (pid $pid) is active in this repository"
        fi
        rm -rf "$LOCK"
        mkdir "$LOCK" || die "could not take the lock at $LOCK"
    fi
    printf '%s\n' "$$" > "$LOCK/pid"
    trap 'rm -rf "$LOCK"' EXIT
}

print_usage() {
    [ -s "$RUN_DIR/usage.tsv" ] || return 0
    say "Token usage by role (headless calls only):"
    awk -F'\t' -v cost="$([ "$PROVIDER" = claude ] && echo 1 || echo 0)" '
        NR > 1 { n[$1]++; inp[$1] += $4; cached[$1] += $5; out[$1] += $6; usd[$1] += $7 }
        END {
            for (r in n) {
                printf "  %-10s %4d calls %11d input %11d cached %9d output", r, n[r], inp[r], cached[r], out[r]
                if (cost) printf "   $%.4f", usd[r]
                printf "\n"
            }
        }' "$RUN_DIR/usage.tsv"
}

main() {
    local top
    case "${1:-}" in
        -h|--help) awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "$0"; exit 0 ;;
        --resume|'') ;;
        *) die "unknown argument: $1 (see --help)" ;;
    esac
    command -v jq > /dev/null || die 'jq is required'
    top=$(git rev-parse --show-toplevel 2> /dev/null) || die 'run this inside a git repository'
    cd "$top"
    acquire_lock

    if [ "${1:-}" = --resume ]; then resume_run "${2:-}"; else new_run; fi

    FINAL_NOTE=''
    plan_milestones
    build_milestones
    review_loop

    say "Done. Branch $BRANCH:"
    git log --oneline "$BASE_SHA"..HEAD
    if [ -n "$FINAL_NOTE" ]; then say "Note: $FINAL_NOTE"; fi
    print_usage
}

main "$@"
