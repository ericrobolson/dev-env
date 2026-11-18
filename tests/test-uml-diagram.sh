#!/usr/bin/env bash
# Offline tests for workflows/uml-diagram.sh: a fake PlantUML install and a fake
# curl stand in for the real download, so nothing touches the network.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="$ROOT/workflows/uml-diagram.sh"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
PASS=0
FAIL=0

check() {
    if "$@"; then PASS=$((PASS+1)); else printf 'FAIL: %s\n' "$*" >&2; FAIL=$((FAIL+1)); fi
}
contains() { [[ "$1" == *"$2"* ]]; }

version=$(sed -n 's/^version=//p' "$SCRIPT")
export UML_PLANTUML_DIR="$TMP/tools"
mkdir -p "$UML_PLANTUML_DIR/$version" "$TMP/bin"
# Fake PlantUML: -version exits 16 like the real one; -pipe emits one <svg per
# @startuml and fails (200) when the source contains BAD.
cat > "$UML_PLANTUML_DIR/$version/plantuml" <<FAKE
#!/bin/bash
if [[ "\$1" == -version ]]; then echo "PlantUML version \${FAKE_VERSION:-$version}"; exit 16; fi
src=\$(cat)
[[ "\$src" != *BAD* ]] || { echo '<svg error/>'; echo 'Syntax Error?' >&2; exit 200; }
printf '%s\n' "\$src" | grep '^@startuml' | sed 's/.*/<svg ok\/>/'
FAKE
chmod +x "$UML_PLANTUML_DIR/$version/plantuml"
cat > "$TMP/bin/curl" <<'FAKE'
#!/bin/bash
echo called >> "$CURL_LOG"
exit 22
FAKE
chmod +x "$TMP/bin/curl"
export CURL_LOG="$TMP/curl.log"
export PATH="$TMP/bin:$PATH"

/bin/bash "$SCRIPT" --help >/dev/null && code=0 || code=$?
check test "$code" -eq 0
/bin/bash "$SCRIPT" --bogus >/dev/null 2>&1 && code=0 || code=$?
check test "$code" -eq 1

# An installed, matching version is reused without any download.
output=$(/bin/bash "$SCRIPT" --setup-only)
check test "$output" = "PlantUML version $version"
check test ! -e "$CURL_LOG"

# A mismatched install triggers a (failing, fake) download and a clear error.
FAKE_VERSION=0.0 /bin/bash "$SCRIPT" --setup-only >/dev/null 2>&1 && code=0 || code=$?
check test "$code" -eq 1
check test -s "$CURL_LOG"
check test ! -e "$UML_PLANTUML_DIR/.setup-lock"
check test -x "$UML_PLANTUML_DIR/$version/plantuml"
rm -f "$CURL_LOG"

# Prompt mode names output after the first target and writes under ./docs/uml.
mkdir -p "$TMP/work/my-service"
prompt=$(cd "$TMP/work" && /bin/bash "$SCRIPT" my-service "only the API")
doc=$(printf '%s\n' "$prompt" | sed -n 's/^DOC: //p')
check contains "$doc" "$TMP/work/docs/uml/"
check contains "$doc" "_MyService.md"
check test -d "${doc%.md}"
check contains "$prompt" "NOTES: only the API"
check contains "$prompt" "  $TMP/work/my-service"
check contains "$prompt" "--render"
mkdir -p "$TMP/dev-env"
prompt=$(cd "$TMP/dev-env" && /bin/bash "$SCRIPT")
check contains "$prompt" "_DevEnv.md"
check contains "$prompt" "  $TMP/dev-env"

# Render: a clean pass writes SVGs named after the source files.
assets="${doc%.md}"
stem=$(basename -- "$assets")
printf '@startuml named\nclass A\n@enduml\n' > "$assets/01_context.puml"
printf '# X\n\n![Context](%s/01_context.svg)\n' "$stem" > "$doc"
/bin/bash "$SCRIPT" --render "$doc" >/dev/null 2>&1 && code=0 || code=$?
check test "$code" -eq 0
check test -s "$assets/01_context.svg"
check test ! -e "$assets/named.svg"

# A syntax error leaves no SVG behind, even a stale one.
printf '@startuml\nBAD\n@enduml\n' > "$assets/01_context.puml"
/bin/bash "$SCRIPT" --render "$doc" >/dev/null 2>&1 && code=0 || code=$?
check test "$code" -eq 1
check test ! -e "$assets/01_context.svg"
check test ! -e "$assets/01_context.svg.part"

# Two diagrams in one file, an unlinked SVG, and a broken link all fail.
printf '@startuml\nclass A\n@enduml\n' > "$assets/01_context.puml"
printf '@startuml\nclass A\n@enduml\n@startuml\nclass B\n@enduml\n' > "$assets/02_two.puml"
printf '# X\n\n![Context](%s/01_context.svg)\n' "$stem" > "$doc"
output=$(/bin/bash "$SCRIPT" --render "$doc" 2>&1) && code=0 || code=$?
check test "$code" -eq 1
check contains "$output" "exactly one"
rm "$assets/02_two.puml"
printf '@startuml\nclass B\n@enduml\n' > "$assets/02_components.puml"
printf '# X\n\n![Context](%s/01_context.svg)\n![Gone](%s/99_gone.svg)\n' "$stem" "$stem" > "$doc"
output=$(/bin/bash "$SCRIPT" --render "$doc" 2>&1) && code=0 || code=$?
check test "$code" -eq 1
check contains "$output" "UNLINKED"
check contains "$output" "BROKEN LINK: $stem/99_gone.svg"

printf 'Results: %s passed, %s failed\n' "$PASS" "$FAIL"
test "$FAIL" -eq 0
