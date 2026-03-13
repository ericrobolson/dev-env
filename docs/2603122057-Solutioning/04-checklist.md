# Implementation Checklist: `bin/research-feature`

## References

- Design: [01-design.md](01-design.md)
- Research: [02-research.md](02-research.md)
- Plan: [03-plan.md](03-plan.md)

---

## 1. Python Helper Script (`bin/research-helper.py`)

- [ ] Create `bin/research-helper.py`
- [ ] Implement `sanitize()` — lowercase, replace non-alphanumeric with hyphens, collapse, strip, truncate to 60 chars
- [ ] Implement `urlencode()` — URL-encode text via `urllib.parse.quote`
- [ ] Implement `search_hn()` — query Algolia HN API, filter `num_comments>10`, return top 2 by points
- [ ] Implement `fetch_url()` — fetch URL content with User-Agent header, 15s timeout, return text
- [ ] Implement `parse_keywords()` — strip numbering/bullets/backticks, return up to 3 clean keywords
- [ ] Implement JSON stdin/stdout main loop — read JSON commands, dispatch to functions, write JSON responses
- [ ] Make file executable (`chmod +x`)

## 2. Main Bash Script (`bin/research-feature`)

### 2a. Script Setup

- [ ] Create `bin/research-feature`
- [ ] Add shebang, source `helpers.sh`
- [ ] Validate args: `validate_args_min 3`
- [ ] Parse args: `FEATURE_NAME`, `DOC_DIRECTORY`, `PROMPT`
- [ ] Call `init_globals`
- [ ] Create `docs/research/` directory
- [ ] Start Python REPL co-process (`coproc PYREPL`)
- [ ] Add trap to kill co-process on exit
- [ ] Implement `py_cmd()` — send JSON to REPL stdin, read JSON from stdout
- [ ] Implement `py_result()` — extract `.result` string from `py_cmd` response
- [ ] Make file executable (`chmod +x`)

### 2b. Stage 1: Keyword Extraction

- [ ] Build keyword extraction prompt (strict: "Output ONLY 3 keywords, one per line")
- [ ] Run `run_agent` with `--no-interactive`
- [ ] Parse raw output via `py_cmd` with `parse_keywords` action
- [ ] Map JSON array into bash `KEYWORDS` array via `mapfile`
- [ ] Validate keyword count > 0, exit 1 if empty

### 2c. Stage 2: HN Search & Article Retrieval (Sequential REPL)

- [ ] Implement `fetch_keyword_articles()` function
  - [ ] Slugify keyword via `py_result` with `sanitize` action
  - [ ] Search HN via `py_cmd` with `search_hn` action
  - [ ] Parse article count from search results
  - [ ] Log warning and skip if 0 results
  - [ ] For each article: extract `title`, `url`, `object_id` from JSON
  - [ ] Slugify title via `py_result`
  - [ ] Fetch article HTML via `py_cmd` with `fetch_url` action
  - [ ] Write article HTML to `docs/research/<kw_slug>--<title_slug>.html`
  - [ ] Handle null/empty URL: fall back to HN comments page
  - [ ] Log warning if content < 100 chars
  - [ ] Fetch HN comments HTML via `py_cmd` with `fetch_url` action
  - [ ] Write comments HTML to `docs/research/<kw_slug>--<title_slug>--comments.html`
- [ ] Loop over all keywords sequentially (REPL is single-threaded)

### 2d. Stage 2b: Agent HTML→Markdown Summarization (Parallel)

- [ ] For each `.html` file in `docs/research/`:
  - [ ] Derive `.md` output path
  - [ ] Run `run_agent "summarize" --no-interactive` in background (`&`)
  - [ ] Prompt: "Read HTML file at X. Summarize into clean markdown. Write to Y."
- [ ] `wait` for all background agent jobs
- [ ] Use `|| true` so failed summarizations don't abort pipeline

### 2e. Stage 3: Per-Keyword Summary

- [ ] For each keyword:
  - [ ] Slugify keyword
  - [ ] Glob `docs/research/<kw_slug>--*.md` (exclude SUMMARY files)
  - [ ] Skip if no article markdown files found
  - [ ] Build summary prompt: read article files, generate themes/patterns/sentiment/links
  - [ ] Run `run_agent "summary-<kw_slug>" --no-interactive`
  - [ ] Output to `docs/research/<kw_slug>-SUMMARY.md`

### 2f. Stage 4: Master Summary + User Review

- [ ] Glob all `*-SUMMARY.md` files
- [ ] Exit 1 if no summary files found
- [ ] Build master prompt: synthesize summaries + original prompt into enriched prompt
- [ ] Run `run_agent "master-summary" --no-interactive`
- [ ] Output to `docs/research/ENRICHED-PROMPT.md`
- [ ] Print path to enriched prompt file
- [ ] Print `bin/build-feature` invocation command for user
- [ ] Call `wait_for_user` for user review gate

## 3. Existing File Updates

- [ ] **README.md** — Add `research-feature` section (usage, prerequisites: Python 3)
- [ ] **Makefile** — Add `test-research-feature` target: `bin/research-feature TestFeature docs "Build a simple key-value store"`
- [ ] **TODOS.md** — Add: `- [ ] Add cleanup mechanism for old docs/research/ files`

## 4. Error Handling

- [ ] Missing args → `validate_args_min` exits 1
- [ ] `init_globals` failure → exit 1
- [ ] 0 keywords extracted → exit 1 with message
- [ ] HN API failure per keyword → `search_hn` returns `[]`, keyword skipped
- [ ] Article URL unreachable → `fetch_url` returns `FETCH_ERROR:...`, logged, skipped
- [ ] All keywords return 0 results → exit 1 (no SUMMARY files)
- [ ] `run_agent` failure on critical stage → exit 1
- [ ] `run_agent` failure on summarization → `|| true`, skip and continue
- [ ] Content < 100 chars → log warning, continue
- [ ] Python REPL dies → trap cleans up; next `py_cmd` errors out

## 5. Testing

- [ ] Happy path end-to-end: real prompt, verify all output files created
- [ ] Keyword extraction with vague prompt → handles gracefully
- [ ] HN API returns no results for keyword → warning logged, keyword skipped
- [ ] Article URL returns 404 → warning logged, article skipped
- [ ] Missing arguments → usage message and exit 1
- [ ] Partial keyword failure → pipeline continues with remaining keywords
- [ ] All keywords fail → clean error exit
- [ ] Makefile `test-research-feature` target runs successfully
