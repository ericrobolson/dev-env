#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
PASS=0
FAIL=0

check() {
    if "$@"; then PASS=$((PASS+1)); else printf 'FAIL: %s\n' "$*" >&2; FAIL=$((FAIL+1)); fi
}

mkdir -p "$TMP/bin"
output=$(PATH="$TMP/bin" /bin/bash "$ROOT/workflows/wikify.sh" demo source.md 2>&1) && code=0 || code=$?
check test "$code" -eq 1
check test "$output" = "Run make install-wiki-writer from $ROOT"

cat > "$TMP/bin/wiki-writer" <<'MOCK'
#!/bin/bash
printf '%s\n' "$@" > "$CAPTURE"
MOCK
chmod +x "$TMP/bin/wiki-writer"
export CAPTURE="$TMP/args"
printf 'claude\n' | PATH="$TMP/bin" /bin/bash "$ROOT/workflows/wikify.sh" demo "$TMP/source one.md" "$TMP/source two.md" >/dev/null
args=(); while IFS= read -r line; do args+=("$line"); done < "$TMP/args"
check test "${args[0]}" = init
check test "${args[1]}" = demo
check test "${args[2]}" = "$TMP/source one.md"
check test "${args[3]}" = "$TMP/source two.md"
check test "${args[4]}" = --agent
check test "${args[5]}" = claude

mkdir -p "$TMP/wiki"
touch "$TMP/wiki/wikify.yaml"
output=$(PATH="$TMP/bin" /bin/bash "$ROOT/workflows/wikify.sh" --project "$TMP/wiki" </dev/null)
args=(); while IFS= read -r line; do args+=("$line"); done < "$TMP/args"
check test "${args[*]}" = "run --project $TMP/wiki"
check test "$output" = ''

output=$(cd "$TMP/wiki" && PATH="$TMP/bin" /bin/bash "$ROOT/workflows/wikify.sh" </dev/null)
args=(); while IFS= read -r line; do args+=("$line"); done < "$TMP/args"
check test "${args[*]}" = "run --project ."

printf 'opencode\n' | PATH="$TMP/bin" /bin/bash "$ROOT/workflows/wikify.sh" demo source.md >/dev/null 2>&1 && code=0 || code=$?
check test "$code" -eq 1
PATH="$TMP/bin" /bin/bash "$ROOT/workflows/wikify.sh" demo >/dev/null 2>&1 && code=0 || code=$?
check test "$code" -eq 2

dry=$(cd "$ROOT" && make -n install-wiki-writer)
check test "${dry#*tools/wiki-writer}" != "$dry"
dry=$(cd "$ROOT" && make -C tools/wiki-writer -n install)
check test "${dry#*cargo build --release}" != "$dry"

printf 'Results: %s passed, %s failed\n' "$PASS" "$FAIL"
test "$FAIL" -eq 0
