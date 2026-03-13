# Research: `bin/research-feature` — HN-Powered Feature Research Pipeline

## References

- Design: [01-design.md](01-design.md)

---

## Stage 1: Keyword Extraction

### How It Works
- `run_agent` receives user prompt, returns 3 newline-delimited keywords
- Non-interactive mode (`--no-interactive`) — agent responds once and exits

### Intricacies
- Agent must return **exactly 3** keywords (design decision #3), not free-form text
- Output parsing: must strip whitespace, empty lines, numbering prefixes (e.g., `1.`, `- `)
- Keywords must be URL-safe for Algolia API query param (spaces → `%20` or `+`)
- Keywords also used in filenames — must sanitize for filesystem (lowercase, hyphens, no special chars)
- Vague prompts ("make something cool") may yield unusable keywords

### Challenges
- Agent output format is unpredictable — may include preamble, markdown, or explanation around keywords
- Need strict output parsing or a structured prompt that forces bare keyword output
- Keyword quality directly impacts all downstream stages

### Prior Art in Repo
- `bin/build-feature` Stage 1 (design) uses `build_prompt | run_agent` pattern — but outputs to a file, not parsed programmatically
- `bin/clean-room` Stage 1 (analysis) uses `run_agent --no-interactive` for unattended execution
- No existing example of parsing agent output as structured data (all current stages write to files)

### Recommendation
- Use `--no-interactive` with a prompt that strictly instructs: "Output ONLY 3 keywords, one per line, no numbering, no explanation"
- Capture stdout, filter blank lines, validate count == 3
- If count != 3, exit with error

---

## Stage 2: HN Search & Article Retrieval

### How It Works
1. For each keyword, call Algolia HN API:
   ```
   https://hn.algolia.com/api/v1/search?query=KEYWORD&tags=story&numericFilters=num_comments>10
   ```
2. Parse JSON response for top 2 stories by points/comments
3. For each article, fetch HTML via `curl -sL`:
   - Article URL → `docs/research/<keyword>--<sanitized-title>.html`
   - HN comments page → `docs/research/<keyword>--<sanitized-title>--comments.html`
4. For each HTML file, `run_agent --no-interactive` summarizes the HTML content into a clean markdown file:
   - `docs/research/<keyword>--<sanitized-title>.md`
   - `docs/research/<keyword>--<sanitized-title>--comments.md`
   - Agent prompt: "Read the HTML file at `<path>`. Summarize its content into clean, readable markdown. Extract the key information, preserve code snippets and links. Output to `<output-path>`."
5. HTML files can be deleted after markdown conversion (or kept for reference)

### Algolia HN Search API Details
- **Base URL**: `https://hn.algolia.com/api/v1/search`
- **Params**: `query`, `tags=story`, `numericFilters=num_comments>10`, `hitsPerPage` (default 20)
- **Response fields**: `.hits[].objectID`, `.hits[].title`, `.hits[].url`, `.hits[].points`, `.hits[].num_comments`
- **Rate limits**: No published rate limit for read-only search; design says retry once after 2s
- **Sorting**: Default is by relevance; use `search_by_date` endpoint for chronological
- **No auth required** — public API

### Intricacies
- **JSON parsing**: Requires `jq` (not currently a documented prerequisite)
- **URL encoding**: Keywords with spaces need encoding for query param
- **Article URL may be null**: Some HN posts are "Ask HN" / "Show HN" with no external URL — `.url` is empty; use HN comments page as fallback
- **HN comments page URL**: `https://news.ycombinator.com/item?id=<objectID>`
- **Content fetching**: `curl -sL` fetches raw HTML into `.html` files; then `run_agent --no-interactive` converts each to a markdown summary
- **File naming**: Title slugification — lowercase, replace spaces/special chars with hyphens, truncate long titles
- **Four files per article**: `.html` + `.md` for article content, `.html` + `.md` for HN comments (design decision #2)
- **Agent summarization**: Each HTML→MD conversion is a separate `run_agent` call; can be parallelized per keyword

### Challenges
- **Paywalled content**: `curl` gets login walls, cookie walls, JS-rendered pages → HTML may be incomplete; agent summarization will extract what it can
- **Large pages**: Some articles produce very long HTML; agent summarization naturally condenses
- **Parallel execution**: Design says parallel (decision #7). Shell options:
  - Background jobs with `&` and `wait`
  - `xargs -P 3` for controlled parallelism
  - `GNU parallel` (not standard on macOS)
- **File collision risk**: Shared `docs/research/` folder (decision #5) — different features may produce same keyword. Mitigate with unique title slugs.

### Prior Art in Repo
- **No existing curl/API usage** in any bin/ script
- **No existing jq dependency** — would be new
- `bin/helpers.sh` has no web-fetching helpers
- `run_agent` with claude has `--dangerously-skip-permissions` which gives the agent shell access to run curl itself

### Recommendation
- Use `curl -s` for HN API calls + `jq` for JSON parsing
- Use `curl -sL` (follow redirects) to fetch article/comments HTML into `.html` files
- Run `run_agent --no-interactive` per HTML file to summarize into `.md`
- Handle null URLs by falling back to HN comments page
- Parallel: use `&` + `wait` (simplest, no extra dependencies)
- Add `jq` to prerequisites in README
- **Only dependency beyond curl is `jq`** — HTML-to-markdown conversion handled by agent

---

## Stage 3: Per-Keyword Summary

### How It Works
- For each keyword, `run_agent` reads all article files for that keyword
- Generates `docs/research/<keyword>-SUMMARY.md` with:
  - Key themes and patterns
  - Technical approaches mentioned
  - Community sentiment / pain points
  - Links to source article files

### Intricacies
- Agent needs file paths passed in prompt — glob `docs/research/<keyword>--*.md` (excludes `.html` files)
- Input is already clean markdown (from Stage 2 agent summarization), not raw HTML
- Summary quality depends on article content quality (garbage in → garbage out)
- With 2 articles per keyword (decision #4), summary may be thin
- Each summary is independent — can run in parallel
- Use `--no-interactive` since no user input needed

### Challenges
- File globbing: keyword may contain characters that interfere with glob patterns
- Agent context window: if article files are very large, agent may truncate
- Consistent summary format across different keywords

### Prior Art in Repo
- `bin/clean-room` Stage 2 collects spec files via `ls "$DIR"/spec-*.md` — same pattern needed here
- `bin/build-feature` Stage 2 (research) generates a research doc referencing the design doc — similar "read inputs, synthesize" pattern

---

## Stage 4: Master Summary + Build-Feature Kickoff

### How It Works
1. Combine all `*-SUMMARY.md` files into a research brief
2. Generate enriched prompt: original prompt + research findings + recommended approaches
3. Present to user for review (decision #8: user reviews before build-feature)
4. User triggers `bin/build-feature <feature-name> <doc-directory> <generated-prompt>`

### Intricacies
- **User review gate**: Design says user reviews and provides feedback/selection (decision #8)
  - Use `wait_for_user` pattern from helpers.sh
  - User may want to edit the enriched prompt before proceeding
- **Prompt size**: Combined research may be very long — need to distill, not concatenate raw
- **Build-feature invocation**: Must pass the enriched prompt as the third argument
  - Shell quoting: prompt with spaces/special chars needs proper quoting

### Challenges
- Enriched prompt could exceed shell argument length limits (ARG_MAX ~262144 on macOS)
- If user declines, should research artifacts still be preserved? (Yes — decision #6)
- `wait_for_user` skips in non-interactive mode (AGENT_TYPE=claude or non-terminal stdin)

### Prior Art in Repo
- `bin/build-feature` Stage 4 (checklist) uses `wait_for_user` — exact pattern to follow
- `bin/clean-room` Stage 3 writes prompt to file instead of executing — alternative approach if prompt is too large

### Recommendation
- Write enriched prompt to a file (e.g., `docs/research/ENRICHED-PROMPT.md`)
- Open for user review with `wait_for_user`
- If user approves, read file content and pass to `build-feature`
- This avoids shell argument length issues and lets user edit

---

## Relevant Files

| File | Path | Relevance |
|------|------|-----------|
| helpers.sh | `bin/helpers.sh` | Core functions: `run_agent`, `validate_args_min`, `init_globals`, `wait_for_user`, `build_prompt`, `new_doc`. Will be sourced by research-feature. |
| build-feature | `bin/build-feature` | Reference implementation for multi-stage pipeline. Final kickoff target for Stage 4. |
| clean-room | `bin/clean-room` | Reference for directory validation, file globbing (`ls "$DIR"/pattern-*.md`), non-interactive stages. |
| gen-doc | `bin/gen-doc` | Minimal single-stage reference. Shows simplest script structure. |
| Makefile | `Makefile` | Add `test-research-feature` target. Current targets: `test-build-feature`, `test-gen-doc`, `test-clean-room`. |
| README.md | `README.md` | Update with research-feature docs (usage, prerequisites, examples). Currently documents 3 tools. |
| TODOS.md | `TODOS.md` | Add cleanup task for research files (decision #6). |
| Design doc | `docs/2603122057-Solutioning/01-design.md` | Source specification with all decisions locked. |

---

## New Dependencies

| Dependency | Purpose | macOS Availability |
|------------|---------|-------------------|
| `curl` | HTTP requests to HN API and article URLs | Preinstalled on macOS |
| `jq` | Parse Algolia HN API JSON responses | `brew install jq` (not preinstalled) |

No HTML-to-text tools needed — raw HTML is stored as-is and the downstream agent interprets it directly.

### Alternative: No New Dependencies
- Use `python3 -c "import json; ..."` instead of `jq` (Python3 is preinstalled on macOS)
- Trade-off: less readable but zero new deps

---

## Parallelism Implementation

### Design Decision
Parallel keyword searches (decision #7).

### Shell Options

| Approach | Pros | Cons |
|----------|------|------|
| `&` + `wait` | No deps, simple | No error collection per job |
| `xargs -P N` | Built-in, controlled concurrency | Awkward for multi-step per-keyword logic |
| `GNU parallel` | Feature-rich | Not preinstalled on macOS |

### Recommended Pattern
```bash
for keyword in "${KEYWORDS[@]}"; do
  process_keyword "$keyword" &
done
wait
# Check for failures via exit status files or a shared error flag
```

Error collection: each background job writes exit status to a temp file; parent checks after `wait`.

---

## Error Handling Summary

| Scenario | Action | Exit Code |
|----------|--------|-----------|
| Missing args | Print usage, exit | 1 |
| init_globals fails | Exit | 1 |
| Keyword extraction returns 0 keywords | Print error, exit | 1 |
| HN API returns no results for a keyword | Log warning, skip keyword | Continue |
| Article URL unreachable | Log warning, skip article | Continue |
| All keywords return zero results | Print error, exit | 1 |
| `run_agent` fails at any stage | Print error with stage name, exit | 1 |
| Rate limiting from HN API | Retry once after 2s; if still failing, skip | Continue |
| Content < 100 chars | Log warning | Continue |

---

## Open Implementation Questions

1. **`jq` vs Python for JSON parsing**: `jq` is idiomatic for shell scripts but is a new dependency. Python3 is preinstalled. Recommend `jq` for readability.
2. **Keyword sanitization function**: Should this be added to `helpers.sh` (reusable) or inline in `research-feature` (simpler)? Recommend inline unless other scripts need it.
3. **Temp file cleanup**: Background jobs need temp files for coordination. Use `mktemp` and `trap` for cleanup on exit.
4. **Max prompt size for build-feature**: If enriched prompt exceeds ~200KB, shell may reject it. Writing to file and reading back is safer.
