#!/usr/bin/env bash
set -euo pipefail

# Start a guided multi-model build. A strong "brain" model plans and reviews, a
# mid-tier "taskmaster" dispatches and verifies tasks, and cheaper "worker"
# subagents write the code, each in its own git worktree made by worktree.sh.
# Finished tasks are merged into the branch that was checked out when this
# script ran. This script records the feature set in
# planner/<timestamp>_<feature>.md and prints a prompt that has the host agent
# fill in and check off that file as it works. Edit the models for each
# provider below.

fail() {
    printf 'Error: %s\n' "$1" >&2
    exit 1
}

wt="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/worktree.sh"
root=$(git rev-parse --show-toplevel 2> /dev/null) || fail 'run this from inside a git repository'
# worktree.sh always merges into the main checkout, so the plan must live there too.
main=$(git worktree list --porcelain | sed -n '1s/^worktree //p')
[[ "$root" == "$main" ]] || fail "run this from the main checkout ($main), not a linked worktree"
base_branch=$(git symbolic-ref --quiet --short HEAD) || fail 'check out a branch first; HEAD is detached'
base_commit=$(git rev-parse --short HEAD)
# Workers start from the last commit, so uncommitted work would be invisible to them.
[[ -z "$(git -C "$root" status --porcelain --untracked-files=all -- . ':!planner')" ]] ||
    fail 'commit or stash your changes first; workers only see committed code'

printf 'Which agent? [claude/codex/opencode]: '
read -r agent

case "$agent" in
    claude)
        brain=claude-opus-5-5
        taskmaster=claude-sonnet-5-5
        worker=claude-haiku-5-5
        ;;
    codex)
        brain=gpt-6-astra
        taskmaster=gpt-6.1-sol
        worker=gpt-6-luna
        ;;
    opencode)
        brain=anthropic/claude-fable-5-1
        taskmaster=anthropic/claude-opus-5-5
        worker=anthropic/claude-sonnet-5-5
        ;;
    *)
        fail "unknown agent: $agent"
        ;;
esac

printf 'Feature name: '
read -r feature_name
safe_name=$(printf '%s' "$feature_name" | LC_ALL=C tr -cs '[:alnum:]_.-' '_' | sed -E 's/^[-_.]+//; s/[-_.]+$//')
[[ -n "$safe_name" ]] || fail 'feature name must contain at least one letter or number'
git check-ref-format "refs/heads/orchestrate/$safe_name/T1" ||
    fail "feature name does not make a valid branch name: $safe_name"

printf 'Feature set, one per line (end with an empty line):\n'
features=''
while IFS= read -r line || [[ -n "$line" ]]; do
    [[ -n "$line" ]] || break
    features+="- $line"$'\n'
done
[[ -n "$features" ]] || fail 'feature set must not be empty'

mkdir -p "$root/planner"
timestamp=$(date '+%y%m%d-%H%M%S')
plan_file="planner/${timestamp}_${safe_name}.md"
[[ ! -e "$root/$plan_file" ]] || fail "plan file already exists: $plan_file"

cat > "$root/$plan_file" <<PLAN
# $feature_name

Status: breakdown
Base: $base_branch @ $base_commit
Models: brain $brain, taskmaster $taskmaster, worker $worker

## Features

$features
## Tasks

_Not written yet._

## Groups

_Not written yet._

## Log
PLAN

cat <<PROMPT

Build the features in $plan_file with an orchestrator model. $brain plans and reviews, $taskmaster dispatches and verifies, and $worker subagents write the code. Keeping the brain-work in one session and the taskmaster work in another keeps both contexts small and token use down.

$plan_file is the source of truth. Keep its Status line current, check off tasks as they are merged, and add one line to its Log for every dispatch, verification, merge, review, and escalation.

Stages:
1. Breakdown ($brain). Split each feature into tasks that each fit in one $worker context window. Give each task an id (T1, T2, ...), the files it may touch, and the tests it must add first. Review the breakdown for gaps, errors, and security issues, write it under Tasks, and stop for my approval.
2. Grouping ($brain). Put tasks that touch separate files and do not depend on each other into the same parallel group. Order the groups so each one builds on the merged result of the ones before it, and mark any group that must run one task at a time as sequential. Write this under Groups and stop for my approval.
3. Build ($brain spins up one $taskmaster orchestrator). Work from the main checkout on $base_branch, and use $wt for every worktree step. For each group in order:
   - Run \`$wt add $safe_name <task id>\` for each task. It prints the worktree path.
   - Spin up one $worker subagent per task. Tell it to work and run tests only in its worktree path, and to commit its work there when done.
   - Verify each result as it comes in: the tests pass and only the task's files changed. Send work back if not.
   - Merge verified tasks one at a time with \`$wt merge $safe_name <task id>\`, and run the tests on $base_branch after each merge.
   - If a merge conflicts, the script aborts it. Do not resolve it by hand. Send it to $brain, because the grouping was wrong.
   - After each merge, run \`$wt remove $safe_name <task id>\`. Then start the next group.
4. Review ($brain). Once $taskmaster gives the all-clear, review the full \`git diff $base_commit..HEAD\` on $base_branch. Findings become a new group of tasks that goes back through stage 3. When the review is clean, set Status to done and stop.

Rules:
- Set the model explicitly on every subagent you spin up, because default subagents inherit the caller's model. The orchestrator runs on $taskmaster, and workers never run on $brain.
- $taskmaster only dispatches, verifies, merges, and updates $plan_file. It does not write code itself.
- $taskmaster picks the smallest model and effort that can handle each task, defaulting to $worker.
- Run a security review whenever a change touches auth, secrets, or API keys. Its findings become new tasks.
- If a task fails twice, $taskmaster narrows it and retries. If it is still stuck, $taskmaster sends $brain a short summary. $brain gives direction but never takes over the work.
PROMPT

printf '\nCreated %s\n' "$plan_file" >&2
if [[ "$agent" == codex ]]; then
    printf 'Note: the Codex CLI may need an agent file for %s before it can spin up subagents on a different model.\n' "$worker" >&2
fi
