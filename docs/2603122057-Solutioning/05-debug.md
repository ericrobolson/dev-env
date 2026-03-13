# Debug Log: `bin/research-feature`

## Issue 1: Remove Python dependency — have agent handle HTTP/parsing

**Problem:** The script used a Python co-process (`research-helper.py`) and numerous inline `python3 -c` calls for JSON parsing, URL fetching, keyword parsing, and string sanitization. This added complexity and a dependency on the Python helper architecture.

**Fix:** Removed all Python usage. The agent now handles HTTP calls (curl to HN Algolia API, article fetching) and JSON parsing directly within its `run_agent` invocations. Simple bash functions (`sanitize_slug`, `parse_keywords`) replace Python for the lightweight text processing the bash script still needs.

**Changes:**
- `bin/research-feature` — Full rewrite:
  - Removed coproc PYREPL setup, `py_cmd()`, `py_result()` functions
  - Removed all `python3 -c` inline calls
  - Added bash `sanitize_slug()` function (tr/sed)
  - Added bash `parse_keywords()` function (sed/head)
  - Stage 2 rewritten: single agent call per keyword handles HN API search, article fetching, HTML saving, and markdown summarization
  - Stage 2b (separate HTML→MD summarization pass) eliminated — now part of Stage 2's agent prompt
  - Stages 3 and 4 unchanged (already agent-driven)
- `bin/research-helper.py` — Deleted (no longer needed)

## Issue 2: Stage 2 HTML→MD summarization too shallow

**Problem:** The agent prompt for step 4 (HTML-to-markdown conversion) was generic — "summarize its content into clean, readable markdown" — which would produce shallow overviews and miss critical technical details.

**Fix:** Replaced the brief summarization instructions with detailed deep-analysis requirements, split by file type:

- **Article HTML**: Agent must extract core thesis, all technical details (architectures, algorithms, benchmarks, trade-offs), preserve every code snippet verbatim, capture performance data, failure modes, links/references, and non-obvious insights.
- **Comments HTML**: Agent must extract top technical insights from practitioners, alternative approaches, war stories, disagreements (both sides), gotchas/warnings, recommended resources, and community consensus.

**Changes:**
- `bin/research-feature` Stage 2 prompt, step 4 — expanded from 5 lines to detailed structured requirements for both article and comments analysis

## Issue 3: Research output directory should be per-feature and timestamped

**Problem:** All research output went to a shared `docs/research/` folder. This risks file collisions between features and makes it hard to associate research with a specific run.

**Fix:** Changed `RESEARCH_DIR` from `docs/research` to `docs/${TIME_STAMP}-${FEATURE_SLUG}-research`. Each run now gets its own isolated directory (e.g., `docs/2603131400-my-feature-research/`). Updated all references including the hardcoded paths in the Stage 2 agent prompt.

**Changes:**
- `bin/research-feature`:
  - Moved `sanitize_slug()` and `parse_keywords()` function definitions before `init_globals` (needed for `FEATURE_SLUG` computation)
  - `RESEARCH_DIR` now set to `docs/${TIME_STAMP}-${FEATURE_SLUG}-research`
  - All `docs/research/` references in Stage 2 agent prompt updated to `${RESEARCH_DIR}/`

## Issue 4: `mapfile` not available on macOS default bash (v3)

**Problem:** `mapfile -t KEYWORDS < <(parse_keywords "$KEYWORDS_RAW")` fails with "command not found" — `mapfile` requires bash 4+ but macOS ships bash 3.2.

**Fix:** Instead of parsing agent stdout in bash, the agent now writes keywords directly to `$RESEARCH_DIR/keywords.txt`. The script reads that file with a `while read` loop to populate the `KEYWORDS` array. Removed the now-unused `parse_keywords()` bash function.

**Changes:**
- `bin/research-feature`:
  - Stage 1 prompt updated: agent writes keywords to `$RESEARCH_DIR/keywords.txt` instead of stdout
  - Replaced `mapfile` with `while IFS= read -r` loop over the keywords file
  - Removed `parse_keywords()` function (no longer needed)

## Issue 5: Remove keyword extraction — let agent search HN directly

**Problem:** Keyword extraction was an unnecessary intermediate step. The agent had to extract keywords, bash had to parse them into an array, then loop over them calling the agent once per keyword. This added complexity, fragile parsing, and multiple agent round-trips.

**Fix:** Collapsed the old Stages 1 (keyword extraction), 2 (per-keyword HN search), and 3 (per-keyword summary) into a single agent call. The agent now:
1. Reads the feature description directly
2. Comes up with 3 search queries on its own
3. Searches HN for all of them
4. Picks the top 6 stories across all queries (deduped)
5. Fetches articles + comments HTML
6. Deep-analyzes each into markdown
7. Generates a single SUMMARY.md across all articles

The script is now just 2 stages: Stage 1 (agent does all research) and Stage 2 (agent generates enriched prompt).

**Changes:**
- `bin/research-feature`:
  - Removed Stage 1 (keyword extraction), Stage 2 (per-keyword loop), Stage 3 (per-keyword summary)
  - New Stage 1: single agent call handles query generation, HN search, fetching, analysis, and summary
  - Stage 2 (master summary): now checks for `SUMMARY.md` instead of `*-SUMMARY.md` glob
  - Removed `KEYWORDS` array, `KEYWORDS_FILE`, `keywords.txt`, per-keyword slug logic
  - File naming simplified: `<title-slug>.html/.md` instead of `<kw-slug>--<title-slug>.html/.md`

## Issue 6: Open files/folders in IDE when stages complete

**Problem:** After each stage completed, the user had no way to review output without manually navigating to the files.

**Fix:** Added `$IDE` calls (uses the `IDE` global set by `init_globals`, defaults to `cursor`) to open results after each stage:
- After Stage 1: opens the research directory so user can browse all fetched articles and analyses
- After Stage 2: opens `ENRICHED-PROMPT.md` directly for review before `wait_for_user`

**Changes:**
- `bin/research-feature`:
  - Added `$IDE "$RESEARCH_DIR"` after Stage 1 completes
  - Added `$IDE "$ENRICHED_PROMPT_FILE"` after Stage 2 completes (before wait_for_user)
