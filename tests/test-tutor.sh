#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TUTOR="$ROOT/workflows/tutor.sh"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
PASS=0
FAIL=0

check() {
    if "$@"; then PASS=$((PASS+1)); else printf 'FAIL: %s\n' "$*" >&2; FAIL=$((FAIL+1)); fi
}
contains() { [[ "$1" == *"$2"* ]]; }
lacks() { [[ "$1" != *"$2"* ]]; }

export GIT_CONFIG_GLOBAL=/dev/null GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t

# Usage errors.
bash "$TUTOR" >/dev/null 2>&1 && code=0 || code=$?
check test "$code" -eq 2
bash "$TUTOR" bogus >/dev/null 2>&1 && code=0 || code=$?
check test "$code" -eq 2

# Not a git repo, decline init.
mkdir -p "$TMP/nogit"
(cd "$TMP/nogit" && printf 'n\n' | bash "$TUTOR" new build a thing >/dev/null 2>&1) && code=0 || code=$?
check test "$code" -eq 1
check test ! -d "$TMP/nogit/tutor"

# Not a git repo, accept init.
(cd "$TMP/nogit" && printf 'y\n\n' | bash "$TUTOR" new build a thing >/dev/null 2>&1)
check test -d "$TMP/nogit/.git"
check test -d "$TMP/nogit/tutor"

# New tutorial from a plan file, in a repo with a commit and an ignored file.
P="$TMP/proj"
mkdir -p "$P/src"
git -C "$P" init -q
printf 'fn main() {}\n' > "$P/src/main.rs"
printf 'target/\n' > "$P/.gitignore"
git -C "$P" add -A && git -C "$P" commit -qm init
printf '# Idea\nBuild a parser.\n' > "$TMP/261015-101714_GainEater.md"

prompt=$(cd "$P" && printf '\n' | bash "$TUTOR" new "$TMP/261015-101714_GainEater.md" only the parser 2>/dev/null)
tdir=$(ls -d "$P"/tutor/*_GainEater)
check test -f "$tdir/TUTOR.md"
check test -f "$tdir/log.md"
check test -d "$tdir/lessons"
check grep -q "^Project: $P$" "$tdir/curriculum.md"
check grep -q 'notes: only the parser' "$tdir/curriculum.md"
check grep -q "$TUTOR diff $tdir" "$tdir/TUTOR.md"
check contains "$prompt" "$tdir/TUTOR.md"
check contains "$prompt" 'only the parser'

# Custom name from a prompt.
(cd "$P" && printf 'My Lexer\n' | bash "$TUTOR" new write a lexer >/dev/null 2>&1)
check test -d "$(ls -d "$P"/tutor/*_My_Lexer)"

# Status before the curriculum exists.
out=$(cd "$P" && bash "$TUTOR" status "$tdir")
check contains "$out" 'curriculum not written yet'

# Write a curriculum as the agent would.
cat >> "$tdir/curriculum.md" <<'EOF'

## Steps
- [ ] Step 1 — Tokens
  **Done when:** tests pass
- [ ] Step 2 — Parser
EOF

out=$(bash "$TUTOR" status "$tdir")
check contains "$out" 'Current:  Step 1 — Tokens'
check contains "$out" 'not begun'
check contains "$out" '0/2'

bash "$TUTOR" diff "$tdir" >/dev/null 2>&1 && code=0 || code=$?
check test "$code" -eq 1

# Begin snapshots without touching the real index, HEAD or stash.
head_before=$(git -C "$P" rev-parse HEAD)
status_before=$(git -C "$P" status --porcelain)
out=$(bash "$TUTOR" begin "$tdir")
check contains "$out" 'Step 1 began'
has_ref() { git -C "$P" rev-parse -q --verify "$1" >/dev/null; }
check has_ref "refs/tutor/$(basename "$tdir")/step-1"
check test "$(git -C "$P" rev-parse HEAD)" = "$head_before"
check test "$(git -C "$P" status --porcelain)" = "$status_before"
out=$(bash "$TUTOR" begin "$tdir")
check contains "$out" 'already began'

# The human edits a tracked file, adds an untracked one, writes an ignored one,
# and the tutor writes a lesson. Only the first two appear in the diff.
printf 'fn main() { lex(); }\n' > "$P/src/main.rs"
printf 'pub fn lex() {}\n' > "$P/src/lexer.rs"
mkdir -p "$P/target" && printf 'junk\n' > "$P/target/out"
printf '# Lesson\n' > "$tdir/lessons/01-tokens.md"
out=$(bash "$TUTOR" diff "$tdir")
check contains "$out" 'lex();'
check contains "$out" 'src/lexer.rs'
lacks_out() { lacks "$out" "$1"; }
check lacks_out 'target/out'
check lacks_out '01-tokens.md'
check test -z "$(git -C "$P" status --porcelain -- src/lexer.rs | grep -v '^??' || true)"

# Committing mid-step keeps the diff against the step's baseline.
git -C "$P" add src && git -C "$P" commit -qm wip
out=$(bash "$TUTOR" diff "$tdir" --stat)
check contains "$out" 'src/lexer.rs'
check contains "$out" 'src/main.rs'

# Hints are counted from the log; status works from inside the tutorial folder.
printf '### Hint (step 1, #1) — x\nlook at chars\n### Hint (step 1, #2) — x\nmore\n### Hint (step 2, #1) — x\n' >> "$tdir/log.md"
out=$(cd "$tdir/lessons" && bash "$TUTOR" status)
check contains "$out" 'Hints:    2 given'
check contains "$out" 'in progress'
check contains "$out" '2 files changed'

# Passing step 1 moves to step 2, which has not begun.
sed -i 's/^- \[ \] Step 1/- [x] Step 1/' "$tdir/curriculum.md"
out=$(bash "$TUTOR" status "$tdir")
check contains "$out" 'Step 2 — Parser'
check contains "$out" 'not begun'
check contains "$out" 'Hints:    1 given'
check contains "$out" '1/2'

# Resume prompt names the protocol and embeds the status.
out=$(bash "$TUTOR" resume "$tdir")
check contains "$out" "$tdir/TUTOR.md"
check contains "$out" 'Step 2 — Parser'

# Default tutorial is the newest under ./tutor.
newest=$(ls -d "$P"/tutor/* | sort | tail -n 1)
out=$(cd "$P" && bash "$TUTOR" resume)
check contains "$out" "$newest/TUTOR.md"
out=$(cd "$P" && bash "$TUTOR" list)
check contains "$out" 'GainEater  (1/2)'

# Completion.
sed -i 's/^- \[ \] Step 2/- [x] Step 2/' "$tdir/curriculum.md"
out=$(bash "$TUTOR" status "$tdir")
check contains "$out" 'complete'
bash "$TUTOR" begin "$tdir" >/dev/null 2>&1 && code=0 || code=$?
check test "$code" -eq 1

printf 'Results: %s passed, %s failed\n' "$PASS" "$FAIL"
test "$FAIL" -eq 0
