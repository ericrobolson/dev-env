# Implementation Plan: `bin/research-feature`

## References

- Design: [01-design.md](01-design.md)
- Research: [02-research.md](02-research.md)

---

## Prerequisites

Python 3 (preinstalled on macOS). No new dependencies.

---

## Architecture: Python REPL Approach

The bash script drives a **long-running Python 3 REPL** (`python3 -i` or a helper script) for all data processing: JSON parsing, URL encoding, string sanitization, HTML-to-text extraction. The agent never executes Python — it only receives processed data via bash variables.

**Flow:**

```
bash script  →  pipes commands to python3  →  reads stdout back into bash vars
bash script  →  pipes prompts to run_agent  →  agent reads/writes files only
```

The Python REPL is a **co-process** that persists for the script's lifetime. Bash sends it commands via stdin and reads results from stdout.

### Python Helper Script: `bin/research-helper.py`

A small Python script that reads JSON commands from stdin (one per line) and writes JSON responses to stdout. Acts as a stateless processing service.

```python
#!/usr/bin/env python3
"""Research-feature helper. Reads JSON commands from stdin, writes JSON responses to stdout."""

import json
import sys
import re
import urllib.parse
import urllib.request

def sanitize(text):
    """Lowercase, replace non-alphanumeric with hyphens, collapse, strip."""
    s = text.lower()
    s = re.sub(r'[^a-z0-9]', '-', s)
    s = re.sub(r'-+', '-', s)
    return s.strip('-')[:60]

def urlencode(text):
    return urllib.parse.quote(text)

def search_hn(keyword):
    """Search HN Algolia API. Returns list of {id, title, url, points}."""
    encoded = urllib.parse.quote(keyword)
    api_url = f"https://hn.algolia.com/api/v1/search?query={encoded}&tags=story&numericFilters=num_comments%3E10&hitsPerPage=5"
    try:
        with urllib.request.urlopen(api_url, timeout=10) as resp:
            data = json.loads(resp.read())
    except Exception:
        return []
    hits = data.get("hits", [])
    hits.sort(key=lambda h: h.get("points", 0), reverse=True)
    results = []
    for h in hits[:2]:
        results.append({
            "id": h.get("objectID", ""),
            "title": h.get("title", ""),
            "url": h.get("url", ""),
            "points": h.get("points", 0),
        })
    return results

def fetch_url(url):
    """Fetch URL content. Returns raw text."""
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
        with urllib.request.urlopen(req, timeout=15) as resp:
            return resp.read().decode("utf-8", errors="replace")
    except Exception as e:
        return f"FETCH_ERROR: {e}"

def parse_keywords(raw_text):
    """Parse agent output into clean keyword list."""
    lines = raw_text.strip().splitlines()
    keywords = []
    for line in lines:
        line = line.strip()
        line = re.sub(r'^[0-9]+[.)]\s*', '', line)  # strip numbering
        line = re.sub(r'^[-*]\s*', '', line)          # strip bullets
        line = line.strip('` ')                        # strip backticks
        if line and not line.startswith('✓') and len(line) < 80:
            keywords.append(line)
    return keywords[:3]

# Main loop: read JSON commands, dispatch, write JSON responses
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        cmd = json.loads(line)
        action = cmd.get("action")

        if action == "sanitize":
            result = {"result": sanitize(cmd["text"])}
        elif action == "urlencode":
            result = {"result": urlencode(cmd["text"])}
        elif action == "search_hn":
            result = {"result": search_hn(cmd["keyword"])}
        elif action == "fetch_url":
            result = {"result": fetch_url(cmd["url"])}
        elif action == "parse_keywords":
            result = {"result": parse_keywords(cmd["text"])}
        else:
            result = {"error": f"unknown action: {action}"}

        print(json.dumps(result), flush=True)
    except Exception as e:
        print(json.dumps({"error": str(e)}), flush=True)
```

---

## File Structure

```
bin/research-feature         # Main bash executable
bin/research-helper.py       # Python REPL helper (data processing only)
docs/research/               # Shared research output directory
```

---

## Implementation

### Bash Script: `bin/research-feature`

#### Script Skeleton + REPL Setup

```bash
#!/bin/bash

# HN-Powered Feature Research Pipeline
# Uses Python REPL co-process for data processing. Agent handles only file I/O and summarization.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$SCRIPT_DIR/helpers.sh" ]]; then
    source "$SCRIPT_DIR/helpers.sh"
else
    echo "Error: helpers.sh not found at $SCRIPT_DIR/helpers.sh" >&2
    exit 1
fi

validate_args_min 3 "research-feature <feature-name> <doc-directory> <prompt>" "$@" || exit 1

FEATURE_NAME="$1"
DOC_DIRECTORY="${2%/}"
shift 2
PROMPT="$@"

init_globals || exit 1

RESEARCH_DIR="docs/research"
mkdir -p "$RESEARCH_DIR"

# Start Python REPL co-process
coproc PYREPL { python3 "$SCRIPT_DIR/research-helper.py"; }
trap "kill $PYREPL_PID 2>/dev/null; wait $PYREPL_PID 2>/dev/null" EXIT

# Send command to Python REPL, read JSON response
py_cmd() {
    echo "$1" >&${PYREPL[1]}
    local response
    read -r response <&${PYREPL[0]}
    echo "$response"
}

# Extract .result from JSON response (simple string)
py_result() {
    local resp=$(py_cmd "$1")
    python3 -c "import json,sys; print(json.loads(sys.stdin.read()).get('result',''))" <<< "$resp"
}

echo "==> Researching feature: $FEATURE_NAME"
```

#### Stage 1: Keyword Extraction

```bash
STAGE="keyword-extraction"
KEYWORD_PROMPT="You are a keyword extraction tool.
Given this feature description, output ONLY 3 search keywords, one per line.
No numbering, no explanation, no markdown, no extra text.

Feature description:
$PROMPT"

KEYWORDS_RAW=$(echo "$KEYWORD_PROMPT" | run_agent "$STAGE" --no-interactive) || exit 1

# Parse keywords via Python helper
KEYWORDS_JSON=$(py_cmd "{\"action\":\"parse_keywords\",\"text\":$(python3 -c "import json; print(json.dumps('''$KEYWORDS_RAW'''))")}")
mapfile -t KEYWORDS < <(python3 -c "
import json, sys
data = json.loads(sys.stdin.read())
for kw in data.get('result', []):
    print(kw)
" <<< "$KEYWORDS_JSON")

if [[ ${#KEYWORDS[@]} -eq 0 ]]; then
    echo "Error: Could not extract keywords from prompt." >&2
    exit 1
fi

echo "==> Keywords: ${KEYWORDS[*]}"
```

#### Stage 2: HN Search & Article Retrieval

All HTTP requests and JSON parsing go through the Python REPL. Agent only summarizes fetched HTML files.

```bash
process_keyword() {
    local keyword="$1"
    local kw_slug=$(py_result "{\"action\":\"sanitize\",\"text\":\"$keyword\"}")

    echo "  -> Searching HN for: $keyword"

    # Search HN via Python helper
    local search_result=$(py_cmd "{\"action\":\"search_hn\",\"keyword\":\"$keyword\"}")
    local articles=$(python3 -c "
import json, sys
data = json.loads(sys.stdin.read())
articles = data.get('result', [])
print(json.dumps(articles))
" <<< "$search_result")

    local count=$(python3 -c "import json,sys; print(len(json.loads(sys.stdin.read())))" <<< "$articles")

    if [[ "$count" -eq 0 ]]; then
        echo "  [WARN] No HN results for: $keyword" >&2
        return 0
    fi

    for ((i=0; i<count; i++)); do
        local title url object_id
        title=$(python3 -c "import json,sys; print(json.loads(sys.stdin.read())[$i]['title'])" <<< "$articles")
        url=$(python3 -c "import json,sys; print(json.loads(sys.stdin.read())[$i].get('url','') or '')" <<< "$articles")
        object_id=$(python3 -c "import json,sys; print(json.loads(sys.stdin.read())[$i]['id'])" <<< "$articles")

        local title_slug=$(py_result "{\"action\":\"sanitize\",\"text\":\"$title\"}")
        local hn_url="https://news.ycombinator.com/item?id=${object_id}"

        # Fetch article HTML via Python helper
        local article_html="$RESEARCH_DIR/${kw_slug}--${title_slug}.html"
        local article_md="$RESEARCH_DIR/${kw_slug}--${title_slug}.md"

        if [[ -n "$url" && "$url" != "None" ]]; then
            local fetch_result=$(py_cmd "{\"action\":\"fetch_url\",\"url\":\"$url\"}")
            python3 -c "
import json, sys
data = json.loads(sys.stdin.read())
content = data.get('result', '')
with open('$article_html', 'w') as f:
    f.write(content)
" <<< "$fetch_result"

            local size=$(wc -c < "$article_html" 2>/dev/null || echo 0)
            if [[ $size -lt 100 ]]; then
                echo "  [WARN] Article content very short (<100 chars): $url" >&2
            fi
        else
            # Ask HN / Show HN — no external URL, fetch comments page
            local fetch_result=$(py_cmd "{\"action\":\"fetch_url\",\"url\":\"$hn_url\"}")
            python3 -c "
import json, sys
data = json.loads(sys.stdin.read())
with open('$article_html', 'w') as f:
    f.write(data.get('result', ''))
" <<< "$fetch_result"
        fi

        # Fetch HN comments HTML via Python helper
        local comments_html="$RESEARCH_DIR/${kw_slug}--${title_slug}--comments.html"
        local comments_md="$RESEARCH_DIR/${kw_slug}--${title_slug}--comments.md"

        local fetch_result=$(py_cmd "{\"action\":\"fetch_url\",\"url\":\"$hn_url\"}")
        python3 -c "
import json, sys
data = json.loads(sys.stdin.read())
with open('$comments_html', 'w') as f:
    f.write(data.get('result', ''))
" <<< "$fetch_result"

        # Agent summarizes article HTML → markdown (agent reads file, writes file)
        if [[ -f "$article_html" && $(wc -c < "$article_html") -gt 0 ]]; then
            echo "Read the HTML file at '$article_html'. Summarize its content into clean, readable markdown. Extract key information, preserve code snippets and links. Write output to '$article_md'." \
                | run_agent "summarize-article" --no-interactive || true
        fi

        # Agent summarizes comments HTML → markdown
        if [[ -f "$comments_html" && $(wc -c < "$comments_html") -gt 0 ]]; then
            echo "Read the HTML file at '$comments_html'. Summarize the HN discussion into clean, readable markdown. Focus on technical insights, recommendations, and community sentiment. Write output to '$comments_md'." \
                | run_agent "summarize-comments" --no-interactive || true
        fi
    done

    echo "  -> Done: $keyword"
}

STAGE="hn-search"
for keyword in "${KEYWORDS[@]}"; do
    process_keyword "$keyword" &
done
wait
```

**Note on parallelism:** The coproc REPL is single-threaded, so parallel `process_keyword` calls cannot share it. Two options:

1. **Run keyword processing sequentially** (simpler, use shared REPL)
2. **Each background job spawns its own `python3 -c`** one-shot calls instead of the coproc (parallel but more process overhead)

Recommended: **sequential for REPL calls, parallel for agent summarization only.** Restructure as:

```bash
# Sequential: fetch all articles via REPL
for keyword in "${KEYWORDS[@]}"; do
    fetch_keyword_articles "$keyword"   # uses py_cmd / REPL
done

# Parallel: agent summarizes all HTML files
for html_file in "$RESEARCH_DIR"/*.html; do
    md_file="${html_file%.html}.md"
    echo "Read '$html_file'. Summarize into clean markdown. Write to '$md_file'." \
        | run_agent "summarize" --no-interactive &
done
wait
```

#### Stage 3: Per-Keyword Summary

```bash
STAGE="per-keyword-summary"
for keyword in "${KEYWORDS[@]}"; do
    kw_slug=$(py_result "{\"action\":\"sanitize\",\"text\":\"$keyword\"}")
    article_files=$(ls "$RESEARCH_DIR"/${kw_slug}--*.md 2>/dev/null | grep -v SUMMARY)

    if [[ -z "$article_files" ]]; then
        echo "  [WARN] No articles for keyword: $keyword, skipping summary" >&2
        continue
    fi

    summary_file="$RESEARCH_DIR/${kw_slug}-SUMMARY.md"
    summary_prompt="You are a research analyst.
Read the following markdown files:
$article_files

Generate a summary in '$summary_file' containing:
- Key themes and patterns across articles
- Relevant technical approaches mentioned
- Community sentiment and common pain points
- Links to source article files

$TERSENESS"

    echo "$summary_prompt" | run_agent "summary-$kw_slug" --no-interactive || true
done
```

#### Stage 4: Master Summary + User Review

```bash
STAGE="master-summary"
ENRICHED_PROMPT_FILE="$RESEARCH_DIR/ENRICHED-PROMPT.md"

SUMMARY_FILES=$(ls "$RESEARCH_DIR"/*-SUMMARY.md 2>/dev/null)
if [[ -z "$SUMMARY_FILES" ]]; then
    echo "Error: No relevant HN articles found. Consider broadening your prompt." >&2
    exit 1
fi

MASTER_PROMPT="You are a research synthesizer.
Read all summary files:
$SUMMARY_FILES

Generate an enriched feature prompt in '$ENRICHED_PROMPT_FILE' that combines:
1. The original user prompt: $PROMPT
2. Key research findings distilled from the summaries
3. Recommended approaches based on HN community insights

Format it as a ready-to-use prompt for build-feature. Keep it focused and actionable.

$TERSENESS"

echo "$MASTER_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1

echo ""
echo "==> Research complete. Review enriched prompt:"
echo "    $ENRICHED_PROMPT_FILE"
echo ""
echo "To proceed: bin/build-feature $FEATURE_NAME $DOC_DIRECTORY \"\$(cat $ENRICHED_PROMPT_FILE)\""

wait_for_user "$ENRICHED_PROMPT_FILE"
```

---

## Error Handling

| Scenario | Code |
|---|---|
| Missing args | `validate_args_min` → exit 1 |
| `init_globals` fails | exit 1 |
| 0 keywords extracted | exit 1 |
| HN API fails for keyword | `search_hn` returns `[]`, keyword skipped |
| Article URL unreachable | `fetch_url` returns `FETCH_ERROR:...`, logged, skipped |
| All keywords return 0 results | exit 1 (no SUMMARY files) |
| `run_agent` fails (critical stage) | exit 1 |
| `run_agent` fails (summarization) | `|| true` — skip, continue |
| Content < 100 chars | Log warning, continue |
| Python REPL dies | trap cleans up; script errors on next `py_cmd` |

---

## Changes to Existing Files

### `README.md`

```markdown
### research-feature

HN-powered feature research pipeline. Searches Hacker News for relevant articles,
summarizes findings, and generates an enriched prompt for `build-feature`.

    bin/research-feature <feature-name> <doc-directory> <prompt>

**Prerequisites:** Python 3 (preinstalled on macOS)
```

### `Makefile`

```makefile
test-research-feature:
	bin/research-feature TestFeature docs "Build a simple key-value store"
```

### `TODOS.md`

```markdown
- [ ] Add cleanup mechanism for old `docs/research/` files
```

---

## Output Files

```
docs/research/
├── crdt--some-article-title.html
├── crdt--some-article-title.md
├── crdt--some-article-title--comments.html
├── crdt--some-article-title--comments.md
├── crdt-SUMMARY.md
├── real-time-sync--...
├── real-time-sync-SUMMARY.md
├── collaborative-editing--...
├── collaborative-editing-SUMMARY.md
└── ENRICHED-PROMPT.md
```

---

## Checklist

- [ ] Create `bin/research-helper.py` (Python REPL helper)
- [ ] Create `bin/research-feature` with executable permissions
- [ ] Implement coproc REPL setup + `py_cmd`/`py_result` helpers
- [ ] Implement Stage 1: keyword extraction + `parse_keywords` via REPL
- [ ] Implement Stage 2: HN search + article fetch via REPL
- [ ] Implement Stage 2b: agent-driven HTML→markdown summarization
- [ ] Implement Stage 3: per-keyword summary generation
- [ ] Implement Stage 4: master summary + enriched prompt generation
- [ ] Add `wait_for_user` gate before build-feature handoff
- [ ] Handle parallelism: sequential REPL calls, parallel agent summarization
- [ ] Add all error handling per table above
- [ ] Update `README.md` (no new deps beyond Python 3)
- [ ] Add `test-research-feature` target to `Makefile`
- [ ] Add cleanup TODO to `TODOS.md`
- [ ] Test: happy path end-to-end
- [ ] Test: keyword extraction with vague prompt
- [ ] Test: HN API returns no results
- [ ] Test: article URL returns 404
- [ ] Test: missing arguments → usage message

---

## Detailed Checklist

See [04-checklist.md](04-checklist.md) for the full, detailed implementation checklist broken down by component and stage.
