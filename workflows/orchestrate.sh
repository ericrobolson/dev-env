#!/usr/bin/env bash
set -euo pipefail

# Print an orchestration prompt for the chosen agent CLI. A strong "brain"
# model plans and reviews, and a mid-tier "taskmaster" runs and verifies
# subagents. Edit the models for each provider below.

printf 'Which agent? [claude/codex/opencode]: '
read -r agent

case "$agent" in
    claude)
        brain=claude-fable-5-1
        taskmaster=claude-opus-5-5
        ;;
    codex)
        brain=gpt-6-astra
        taskmaster=gpt-6.1-sol
        ;;
    opencode)
        brain=anthropic/claude-fable-5-1
        taskmaster=anthropic/claude-opus-5-5
        ;;
    *)
        printf 'Error: unknown agent: %s\n' "$agent" >&2
        exit 1
        ;;
esac

cat <<PROMPT

Use an orchestrator model. $brain writes the plan, spins up $taskmaster orchestrator, $taskmaster spins up subagents and verifies their work as it comes in, $brain does full-branch review after $taskmaster gives the all-clear. $brain directs $taskmaster on any changes if applicable. Especially on larger builds, separating out the brain-work ($brain) from the taskmaster work ($taskmaster) gets even better token optimization.
PROMPT
