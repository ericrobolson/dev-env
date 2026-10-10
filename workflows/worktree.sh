#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat >&2 <<'EOF'
Usage: worktree.sh add <feature> <task>
       worktree.sh merge <feature> <task>
       worktree.sh remove [--force] <feature> <task>

Manage one worktree per orchestrated task. Each task gets the branch
orchestrate/<feature>/<task> in .worktrees/<feature>-<task>/ of the main
checkout.

  add     Create the worktree from the main checkout's current commit and
          print its path.
  merge   Merge the task branch into the main checkout's current branch with
          --no-ff. On a conflict the merge is aborted and the exit status is 1.
  remove  Delete the worktree and its branch. Refuses an unmerged branch
          unless --force is given.
EOF
}

fail() {
    printf 'Error: %s\n' "$1" >&2
    exit 1
}

check_name() {
    [[ "$2" =~ ^[A-Za-z0-9][A-Za-z0-9_.-]*$ ]] || fail "invalid $1 name: $2"
}

force=false
if [[ $# -eq 4 && "$1" == remove && "$2" == --force ]]; then
    force=true
    set -- "$1" "$3" "$4"
fi
if [[ $# -ne 3 ]]; then
    usage
    exit 2
fi

command="$1"
feature="$2"
task="$3"
check_name feature "$feature"
check_name task "$task"

# Work from the main checkout even when called from inside a task worktree.
main=$(git worktree list --porcelain 2> /dev/null | sed -n '1s/^worktree //p')
[[ -n "$main" ]] || fail 'not inside a git repository'
path="$main/.worktrees/$feature-$task"
branch="orchestrate/$feature/$task"
git check-ref-format "refs/heads/$branch" || fail "not a valid branch name: $branch"

case "$command" in
    add)
        [[ ! -e "$path" ]] || fail "worktree already exists: $path"
        exclude="$(git -C "$main" rev-parse --path-format=absolute --git-common-dir)/info/exclude"
        if ! git -C "$main" check-ignore -q .worktrees/probe; then
            mkdir -p "$(dirname "$exclude")"
            printf '.worktrees/\n' >> "$exclude"
        fi
        git -C "$main" worktree add --quiet -b "$branch" "$path" HEAD
        printf '%s\n' "$path"
        ;;
    merge)
        [[ -d "$path" ]] || fail "no worktree for $feature $task: $path"
        [[ -z "$(git -C "$path" status --porcelain)" ]] || fail "uncommitted changes in $path; commit them first"
        if ! git -C "$main" merge --quiet --no-ff -m "Merge $feature task $task" "$branch"; then
            git -C "$main" merge --abort 2> /dev/null || true
            fail "merge of $branch conflicted and was aborted"
        fi
        ;;
    remove)
        if [[ "$force" != true ]] && git -C "$main" show-ref --verify --quiet "refs/heads/$branch" &&
            ! git -C "$main" merge-base --is-ancestor "$branch" HEAD; then
            fail "$branch is not merged; use --force to discard it"
        fi
        if [[ -d "$path" ]]; then
            if [[ "$force" == true ]]; then
                git -C "$main" worktree remove --force "$path"
            else
                git -C "$main" worktree remove "$path"
            fi
        fi
        if git -C "$main" show-ref --verify --quiet "refs/heads/$branch"; then
            if [[ "$force" == true ]]; then
                git -C "$main" branch --quiet -D "$branch"
            else
                git -C "$main" branch --quiet -d "$branch" || fail "$branch is not merged; use --force to discard it"
            fi
        fi
        ;;
    *)
        usage
        exit 2
        ;;
esac
