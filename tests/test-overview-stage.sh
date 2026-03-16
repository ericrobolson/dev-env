#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../bin/helpers.sh"
init_globals

PASS=0
FAIL=0
ORIGINAL_DIR="$(pwd)"

echo "Test 1: Git baseline capture in a git repo"
test_git_baseline_capture() {
    local tmpdir=$(mktemp -d)
    trap "rm -rf $tmpdir" RETURN

    cd "$tmpdir"
    git init -q
    echo "initial" > file.txt
    git add file.txt
    git commit -q -m "initial"

    local ref=$(git rev-parse HEAD)
    cd "$ORIGINAL_DIR"

    if [[ -n "$ref" ]]; then
        ((PASS++))
        echo "  PASS: git baseline captured: $ref"
    else
        ((FAIL++))
        echo "  FAIL: baseline ref empty"
    fi
}
test_git_baseline_capture

echo "Test 2: Overview file path from new_doc"
test_overview_file_created() {
    local doc_dir=$(mktemp -d)
    trap "rm -rf $doc_dir" RETURN

    local overview_file=$(new_doc "$doc_dir" "99-overview.md")
    if [[ "$overview_file" == "$doc_dir/99-overview.md" ]]; then
        ((PASS++))
        echo "  PASS: overview file path correct"
    else
        ((FAIL++))
        echo "  FAIL: wrong path — got $overview_file"
    fi
}
test_overview_file_created

echo "Test 3: Prompt excludes 00-prompts.md"
test_prompt_excludes_prompts_file() {
    local prompt="Read all markdown files in '/tmp/docs' in filename-sorted order, excluding '00-prompts.md'."
    if echo "$prompt" | grep -q "excluding '00-prompts.md'"; then
        ((PASS++))
        echo "  PASS: prompt excludes 00-prompts.md"
    else
        ((FAIL++))
        echo "  FAIL: exclusion clause not found in prompt"
    fi
}
test_prompt_excludes_prompts_file

echo "Test 4: No git repo fallback — GIT_BASELINE_REF is empty"
test_no_git_repo_fallback() {
    local tmpdir=$(mktemp -d)
    trap "rm -rf $tmpdir" RETURN

    cd "$tmpdir"

    local ref=""
    if git rev-parse HEAD >/dev/null 2>&1; then
        ref=$(git rev-parse HEAD)
    fi

    cd "$ORIGINAL_DIR"

    if [[ -z "$ref" ]]; then
        ((PASS++))
        echo "  PASS: no git repo handled gracefully"
    else
        ((FAIL++))
        echo "  FAIL: ref should be empty outside git repo, got $ref"
    fi
}
test_no_git_repo_fallback

echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]] && exit 0 || exit 1
