1:# Design: `bin/research-feature` — HN-Powered Feature Research Pipeline

## Overview

A new executable `bin/research-feature` that takes a feature prompt, extracts keywords, searches Hacker News for high-engagement posts relevant to those keywords, saves article content as raw text, generates per-keyword summaries, and then kicks off `bin/build-feature` with a research-informed prompt.

## Arguments

```
bin/research-feature <feature-name> <doc-directory> <prompt>
```

Mirrors `build-feature` signature for consistency.

## Pipeline Stages

### Stage 1: Keyword Extraction

- Input: user's feature prompt (free text)
- Output: list of 3–5 keywords/phrases
- Method: `run_agent` with a prompt asking the agent to extract search-worthy keywords
- Output format: newline-delimited keywords written to stdout or a temp file

### Stage 2: HN Search & Article Retrieval

For each keyword:
1. `run_agent` searches Hacker News (via Algolia HN Search API: `hn.algolia.com/api/v1/search?query=KEYWORD&tags=story&numericFilters=num_comments>10`) for stories sorted by relevance/points
2. Select top 3 articles by comment count
3. For each article, fetch the URL content as raw text
4. Write to `docs/research/<keyword>--<TITLE>.md` (title slugified, lowercase, hyphens)

**File naming**: `docs/research/<keyword>--<sanitized-title>.md`

### Stage 3: Per-Keyword Summary

For each keyword:
- `run_agent` reads all 3 article files for that keyword
- Generates `docs/research/<keyword>-SUMMARY.md` containing:
  - Key themes and patterns across articles
  - Relevant technical approaches mentioned
  - Community sentiment / common pain points
  - Links to source article files

### Stage 4: Master Summary + Build-Feature Kickoff

- Join all `*-SUMMARY.md` files into a combined research brief
- Generate an enriched prompt that combines:
  - Original user prompt
  - Research findings (distilled)
  - Recommended approaches based on HN community insights
- Execute `bin/build-feature <feature-name> <doc-directory> <generated-prompt>`

## Happy Path

1. User runs: `bin/research-feature MyFeature docs "I want to build a real-time collaborative editor"`
2. Agent extracts keywords: `collaborative editing`, `CRDT`, `real-time sync`, `operational transform`
3. For each keyword, 3 HN articles fetched → 12 files written to `docs/research/`
4. 4 summary files generated (`collaborative-editing-SUMMARY.md`, etc.)
5. Combined prompt generated and `bin/build-feature` kicks off automatically
6. User enters the normal build-feature flow (design → research → plan → implement → debug)

## Unhappy Paths

| Scenario | Handling |
|----------|----------|
| HN API returns no results for a keyword | Log warning, skip keyword, continue with remaining keywords |
| Article URL unreachable / 404 | Log warning, skip article, continue. If <3 articles found for keyword, proceed with what's available |
| All keywords return zero results | Exit with error: "No relevant HN articles found. Consider broadening your prompt." |
| `run_agent` fails at any stage | Exit with error code 1 and stage name (matches `build-feature` pattern) |
| Article content is paywalled / JS-rendered | Agent gets whatever raw text is available; may be low quality. Log warning if content is very short (<100 chars) |
| Keyword extraction returns 0 keywords | Exit with error: "Could not extract keywords from prompt." |
| `build-feature` fails after kickoff | Normal `build-feature` error handling applies; research artifacts remain in `docs/research/` for reuse |
| Rate limiting from HN API | Retry once after 2s delay; if still failing, skip and continue |

## Open Questions / Unknowns

1. **HN Search method**: Should the agent use the Algolia HN API directly (via `curl`/`wget`) or use the agent's web search capability? The Algolia API is more structured and reliable. **DECISION**: Use the Algolia HN API directly (via `curl`/`wget`)
2. **Article content fetching**: Should we fetch the linked article URL or the HN comments page? Both have value.  **DECISION**: Fetch the linked article URL for content as well as the comments page and store both in markdown files
3. **Keyword count**: Is 3–5 keywords the right range? Too many = slow; too few = narrow research.  **DECISION**: use 3
4. **Top N articles**: Is 3 per keyword correct? With 5 keywords that's 15 articles — could be slow.  **DECISION**:  use 2
5. **`docs/research/` location**: Should this be inside the timestamped feature directory (e.g., `docs/2603122057-Solutioning/research/`) for isolation, or a shared `docs/research/` folder? Shared folder risks collisions between features.  **DECISION**:  use a shared `docs/research/` folder
6. **Cleanup**: Should old research files be cleaned up, or preserved indefinitely?  **DECISION**:  preserve indefinitely. Add a TODOS.md task later on 
7. **Parallelism**: Should keyword searches run in parallel (faster) or sequentially (simpler, less API pressure)?  **DECISION**: use parallel
8. **Build-feature auto-start**: Should `build-feature` start automatically, or should the user review research first and manually trigger it?  **DECISION**: user reviews and provides feedback/selection of what to pass forward

## Testing Plan

### Unit-Level Test Cases

| # | Test Case | Input | Expected Output |
|---|-----------|-------|-----------------|
| 1 | Keyword extraction from clear prompt | "Build a CLI tool for managing Docker containers" | Keywords include: `docker`, `CLI`, `container management` |
| 2 | Keyword extraction from vague prompt | "Make something cool" | Either extracts generic keywords or exits with error |
| 3 | HN search returns results | Keyword: "CRDT" | 3 article files written to `docs/research/` |
| 4 | HN search returns no results | Keyword: "xyzzy12345nonsense" | Warning logged, keyword skipped |
| 5 | Article fetch succeeds | Valid URL | Raw text file written, >100 chars |
| 6 | Article fetch fails (404) | Dead URL | Warning logged, article skipped |
| 7 | Summary generation | 3 article files exist for keyword | `<keyword>-SUMMARY.md` created with themes/patterns |
| 8 | Master prompt generation | All summaries exist | Combined prompt includes original prompt + research |
| 9 | build-feature kickoff | Valid combined prompt | `build-feature` starts successfully |

### Integration Test Cases

| # | Test Case | Description |
|---|-----------|-------------|
| 1 | Full happy path | End-to-end run with a real prompt; verify all files created and build-feature starts |
| 2 | All keywords fail | Verify clean error exit |
| 3 | Partial keyword failure | Verify pipeline continues with successful keywords |
| 4 | Argument validation | Missing args → usage message and exit 1 |
| 5 | Makefile target | Add `test-research-feature` target to Makefile |
