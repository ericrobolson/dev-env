#!/bin/bash
# Shared helper functions for bin/ scripts

# Exit if sourced from non-bash shell
if [ -z "$BASH_VERSION" ]; then
    echo "Error: helpers.sh must be sourced from bash" >&2
    return 1
fi

# validate_args_min: Exit if $# < count
# Usage: validate_args_min <count> <usage_message>
validate_args_min() {
    local min_count="$1"
    local usage_msg="$2"
    shift 2

    if [[ $# -lt $min_count ]]; then
        echo "Usage: $usage_msg" >&2
        return 1
    fi
}

# init_globals: Set shared global variables
# Sets: TIME_STAMP, IDE, AGENT_TYPE, CURSOR_MODEL, TERSENESS
init_globals() {
    TIME_STAMP=$(date +%y%m%d%H%M)
    IDE="${IDE:-cursor}"
    AGENT_TYPE="${AGENT_TYPE:-claude}"
    CURSOR_MODEL="${CURSOR_MODEL:-kimi-k2.5}"
    TERSENESS="Be concise and to the point. Stick to facts. Be succinct and terse. Don't be verbose."
    

    # Validate AGENT_TYPE
    if [[ "$AGENT_TYPE" != "cursor" && "$AGENT_TYPE" != "claude" ]]; then
        echo "Error: AGENT_TYPE must be 'cursor' or 'claude', got: $AGENT_TYPE" >&2
        return 1
    fi
}

# run_agent: Execute cursor or claude agent based on AGENT_TYPE
# Usage: echo "prompt" | run_agent <stage_name> [--no-interactive]
run_agent() {
    local stage="$1"
    local interactive=true

    if [[ "$2" == "--no-interactive" ]]; then
        interactive=false
    fi

    local prompt
    prompt=$(cat)

    if [[ -z "$prompt" ]]; then
        echo "Error: No prompt provided to run_agent" >&2
        return 1
    fi

    if [[ "$AGENT_TYPE" == "claude" ]]; then
        local cmd=(claude --dangerously-skip-permissions)
        if [[ "$interactive" == false ]]; then
            cmd+=(--print)
        fi
        if ! echo "$prompt" | "${cmd[@]}"; then
            echo "Error: claude agent failed at stage '$stage'" >&2
            return 1
        fi
    else
        local cmd=(cursor-agent --model "$CURSOR_MODEL")
        if [[ "$interactive" == false ]]; then
            cmd+=(--print)
        fi
        if ! echo "$prompt" | "${cmd[@]}"; then
            echo "Error: cursor-agent failed at stage '$stage'" >&2
            return 1
        fi
    fi

    echo "✓ Stage '$stage' complete"
}

# new_doc: Return filepath for new document
# Usage: filepath=$(new_doc <directory> <filename>)
new_doc() {
    local dir="${1%/}"  # Remove trailing slash
    local filename="$2"
    echo "$dir/$filename"
}

# wait_for_user: Loop until user confirms (skip if AGENT_TYPE=claude or non-interactive)
# Usage: wait_for_user <filepath>
wait_for_user() {
    local filepath="$1"

    # Skip for claude agent or non-interactive shells
    if [[ "$AGENT_TYPE" == "claude" ]] || [[ ! -t 0 ]]; then
        return 0
    fi

    echo ""
    echo "Review: $filepath"
    echo "Open file in IDE to review. Continue? (y/n)"

    while true; do
        read -r response
        case "$response" in
            [yY]|[yY][eE][sS])
                return 0
                ;;
            [nN]|[nN][oO])
                echo "Continuing without confirmation..."
                return 0
                ;;
            *)
                echo "Please enter 'y' or 'n':"
                ;;
        esac
    done
}

# build_prompt: Append standard suffix to prompt
# Usage: prompt=$(build_prompt <filepath> <instructions>...)
build_prompt() {
    local filepath="$1"
    shift
    local instructions="$@"

    echo "$instructions

$TERSENESS

Output everything to the file '$filepath'.

Then open the file '$filepath' in the IDE '$IDE' so I can review it.

If possible, play an audio notification to alert me that the file is ready to review."
}

# append_prompt: Log a prompt to a markdown file
# Usage: append_prompt <filepath> <stage_name> <prompt_text>
# Non-critical — warns on failure, never exits
append_prompt() {
    local filepath="$1"
    local stage_name="$2"
    local prompt_text="$3"

    # Create file with header if it doesn't exist
    if [[ ! -f "$filepath" ]]; then
        printf '# Prompts\n\n' > "$filepath" 2>/dev/null || {
            echo "Warning: Could not create prompt log: $filepath" >&2
            return 0
        }
    fi

    # Append stage section with 4-backtick fence
    {
        printf '## %s\n\n' "$stage_name"
        printf '````\n'
        printf '%s\n' "$prompt_text"
        printf '````\n\n'
    } >> "$filepath" 2>/dev/null || {
        echo "Warning: Could not write to prompt log: $filepath" >&2
    }

    return 0
}

# append_resume: Log the claude resume command to the prompts file
# Usage: append_resume <prompts_file>
# Finds the most recent claude session for the current project and appends the resume command.
# Non-critical — silently does nothing if session not found.
append_resume() {
    local prompts_file="$1"

    # Only applicable for claude agent
    if [[ "$AGENT_TYPE" != "claude" ]]; then
        return 0
    fi

    # Derive claude project directory from cwd
    local claude_project_dir="$HOME/.claude/projects/$(pwd | sed 's|/|-|g')"

    # Find most recent .jsonl session file
    local latest
    latest=$(ls -t "$claude_project_dir"/*.jsonl 2>/dev/null | head -1)

    if [[ -z "$latest" ]]; then
        return 0
    fi

    # Extract session ID from filename
    local session_id
    session_id=$(basename "$latest" .jsonl)

    # Append resume command to prompts file
    {
        printf '> To resume this session, run:\n'
        printf '> `claude --resume %s`\n\n' "$session_id"
    } >> "$prompts_file" 2>/dev/null

    return 0
}
