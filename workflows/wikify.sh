#!/usr/bin/env bash
set -euo pipefail

# Initialize a local wiki from source documents, or continue an existing project.
usage() {
    printf '%s\n' \
        'Usage: wikify.sh <project> <sources...>' \
        '       wikify.sh --project DIR' \
        '       wikify.sh  # inside a wiki project' >&2
}

fail() {
    printf 'Error: %s\n' "$1" >&2
    exit 1
}

if ! command -v wiki-writer >/dev/null 2>&1; then
    printf 'Run make install-wiki-writer from %s\n' "$(cd "${BASH_SOURCE[0]%/*}/.." && pwd)" >&2
    exit 1
fi

mode=''
project=''
sources=()
if [[ $# -eq 0 ]]; then
    [[ -f ./wikify.yaml ]] || { usage; exit 2; }
    mode='existing'
    project='.'
elif [[ "$1" == --project ]]; then
    [[ $# -eq 2 ]] || { usage; exit 2; }
    mode='existing'
    project="$2"
elif [[ "$1" == -* || $# -lt 2 ]]; then
    usage
    exit 2
else
    mode='new'
    project="$1"
    shift
    sources=("$@")
fi

if [[ "$mode" == existing ]]; then
    exec wiki-writer run --project "$project"
fi
printf 'Which agent? [claude/codex]: '
read -r agent
case "$agent" in
    claude|codex) ;;
    *) fail "unknown agent: $agent" ;;
esac
exec wiki-writer init "$project" "${sources[@]}" --agent "$agent"
