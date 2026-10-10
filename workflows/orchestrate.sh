#!/usr/bin/env bash
set -euo pipefail

# Print an orchestration prompt for the chosen agent CLI. A strong "brain"
# model plans and reviews, a mid-tier "taskmaster" plans and verifies the
# workers' tasks, and cheaper "worker" subagents write the code. Edit the models
# for each provider below.

printf 'Which agent? [claude/codex/opencode]: '
read -r agent

case "$agent" in
    claude)
        brain=claude-fable-5-1
        taskmaster=claude-opus-5-5
        worker=claude-sonnet-5-5
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
        printf 'Error: unknown agent: %s\n' "$agent" >&2
        exit 1
        ;;
esac

cat <<PROMPT

Use an orchestrator model. $brain writes the plan, spins up $taskmaster orchestrator, $taskmaster plans out tasks for $worker subagents, spins them up to write the code, and verifies their work as it comes in, $brain does full-branch review after $taskmaster gives the all-clear. $brain directs $taskmaster on any changes if applicable. Especially on larger builds, separating out the brain-work ($brain) from the taskmaster work ($taskmaster) gets even better token optimization.

Rules:
- Set the model explicitly on every subagent you spin up. Default subagents inherit the caller's model, so never spin up a $brain subagent.
- Before showing me the plan, $brain reviews it for gaps, errors, and security issues.
- $brain writes the plan as tasks in <plan>_tasks.md. Each task fits in one $worker context window, lists the files it may touch, and is built test-first. Split anything larger.
- Order tasks by dependency. Run tasks in parallel only when no two agents touch the same or interacting files.
- $taskmaster only dispatches tasks, verifies results, and marks tasks done in the tasks file. It does not write code itself.
- $taskmaster picks the smallest model and effort that can handle each task, defaulting to $worker.
- Run a review after every few tasks, and a security review whenever a change touches auth, secrets, or API keys. Review findings become new tasks in the tasks file.
- If a task fails twice, $taskmaster narrows it and retries. If it is still stuck, $taskmaster sends $brain a short summary. $brain gives direction but never takes over the work.
PROMPT

if [[ "$agent" == codex ]]; then
    printf '\nNote: the Codex CLI may need an agent file for %s before it can spin up subagents on a different model.\n' "$worker" >&2
fi
