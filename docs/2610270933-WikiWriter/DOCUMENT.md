# WikiWriter — turn a PDF/EPUB into a DokuWiki

Status: design agreed, ready for planning
Date: 2026-10-27

## Goal

Take one or more source documents (PDF, EPUB, plain text/Markdown) about a
subject — a TTRPG rulebook, a videogame guide, a trading-strategy book — and
build a browsable, cross-linked DokuWiki for it under `wikis/{project}/`, with a
Makefile that installs, builds and serves it locally.

The pipeline must be idempotent and resumable: it can be killed at any point and
re-run, and it only redoes work whose inputs changed.

It leans on the cheapest model as much as possible. Every work item starts on the
small model (Claude Haiku / Codex luna) and only escalates to the mid model
(Sonnet / sol) and then the large model (Opus / astra) when deterministic checks
fail. The large model is used only for the few global, judgment-heavy steps.

## Pieces

1. `tools/wiki-writer/` — a Rust crate (edition 2024) producing the
   `wiki-writer` binary. It is the driver: it does all file I/O, hashing,
   validation, scheduling and rendering, and calls agent CLIs headlessly.
2. `tools/wiki-writer/Makefile` — `make deps` checks for and installs (via brew)
   `poppler` and `php`; `make install` runs `cargo build --release` and copies the
   binary to `~/bin/wiki-writer`; `make test` runs `cargo test`.
3. `workflows/wikify.sh` — thin launcher in the style of the other workflow
   scripts: asks `Which agent? [claude/codex]:`, checks `wiki-writer` is on PATH
   (tells the user to run `make -C tools/wiki-writer install` if not), then runs
   `wiki-writer init` for a new project or `wiki-writer run` for an existing one.
4. Top-level `Makefile` gains `install-wiki-writer` that runs
   `make -C tools/wiki-writer deps install`.
5. `wikis/{project}/` — per-wiki output, created by `wiki-writer init`, relative
   to the current directory (like `docs/` and `planner/` in the other workflows).

## Agent invocation (copy forgeflow's pattern, write it here)

Reference: `~/dev/forgeflow/src/agent.rs`. It has a `Provider` enum
(Opencode/Codex/Claude) whose `command()` builds a `std::process::Command`, and
`Agent::ask(prompt)` spawns the CLI, reads stdout line by line as JSON events,
and extracts text per provider:

- Claude: `claude -p --output-format stream-json --verbose <prompt>`; text is the
  `result` field of an event.
- Codex: `codex exec --json <prompt>`; text is `item.text` where
  `item.type == "agent_message"`.
- Opencode: `opencode run --format json <prompt>`; text is `part.text` where
  `type == "text"`.
- Non-zero exit → `io::Error::other("<provider> exited with <status>")`.

wiki-writer writes its own version of this in `src/agent.rs` with these changes:

- Providers: Claude, Codex, and Fake (for tests). Opencode is out of scope.
- `ask(model, prompt) -> Result<Response>` where
  `Response { text, model, input_tokens, output_tokens, cost_usd: Option<f64> }`.
  Claude's final `result` event carries `total_cost_usd` and `usage`; Codex
  reports token usage but not cost.
- Pass `--model <id>` (Claude) / `-m <id>` (Codex).
- Agents get no tools: the driver embeds all input text in the prompt and the
  agent returns JSON or Markdown. Claude is run with tools disabled; Codex with
  `--sandbox read-only`. (Verify the exact flags against the installed CLIs.)
- A per-call timeout that kills the child.
- Errors are classified: rate-limit/overload (retry same model with backoff),
  bad output (escalate), other failure (retry once, then escalate).
- Every call appends one line to `state/ledger.jsonl`: stage, item, model,
  attempt, tokens, cost, outcome, duration.
- The Fake provider returns canned responses from fixture files keyed by stage +
  item + model, so escalation, validators and resume are testable with
  `cargo test` and no network.

No async runtime. Concurrency is `std::thread::scope` + channels, matching
forgeflow.

## Models

| Tier | Claude | Codex |
|---|---|---|
| Small | claude-haiku-5-5 | gpt-6-luna |
| Mid | claude-sonnet-5-5 | gpt-6.1-sol |
| Large | claude-opus-5-5 | gpt-6-astra |

`wikify.yaml` can override any of them per wiki. `--ceiling mid` caps escalation
(useful for small books).

## Escalation ladder

```
small ──validate──▶ ok ▶ done
  │ fail
  └▶ small again, with the validator errors in the prompt ──▶ ok ▶ done
        │ fail
        └▶ mid (same prompt + errors) ──▶ fail ▶ large ──▶ fail ▶ blocked
```

Rate-limit errors never escalate; they back off (1s, 2s, 4s, … capped) and
retry the same model. Repeated rate limits lower the worker pool size for the
rest of the run.

Blocked items are reported by `status` and do not stop other items. Downstream
stages proceed without them and note the gap.

## Deterministic validators (no model)

- Grounding: every extracted fact has a `quote`; after whitespace/case
  normalization the quote must be a substring of the chunk text. Every number in
  the fact must appear in its quote. Page refs must lie inside the chunk's page
  range.
- Schema: output deserializes into the stage's Rust struct; entity types exist in
  `schema.yaml`.
- Coverage: a page writer must cite every fact assigned to the page using a
  marker `[^fNNNN]`; each assigned fact id must appear at least once.
- Links: every wiki link in written Markdown targets a page in the outline.

## Pipeline

| # | Stage | Who | Output |
|---|---|---|---|
| 0 | ingest | Rust (poppler shell-out for PDF; zip + XML for EPUB) | `state/ingest/pages/NNNN.txt`, `toc.json`, `chunks.json` |
| 1 | survey | large | `state/schema.yaml` (entity types + fields, namespaces) |
| 2 | extract | small → escalate | `state/notes/cNNNN.json` |
| 3 | audit | mid on a sample + all items that escalated + all table/stat-block items | `state/audit/cNNNN.json` (corrections, applied deterministically) |
| 4 | resolve | Rust candidate pairs (strsim) → small yes/no with evidence → mid for ambiguous clusters | `state/entities.json` |
| 5 | organize | large, on a compact entity index (never raw text) | `state/outline.yaml` |
| 6 | write | small for leaf pages, mid for hub/index/overview pages; escalate on validator failure | `content/<ns>/<page>.md` |
| 7 | link/lint | Rust | auto-link alias mentions, report broken links and orphans |
| 8 | review | large, samples pages | `state/review.json`; findings become redo items |

Details:

- Ingest: `pdfinfo` for page count; `pdftotext -layout -f N -l N` per page; the
  PDF outline if present for chunk boundaries, else fixed page windows with one
  page of overlap. Target chunk size ~4–6k tokens (estimate 4 chars/token).
  EPUB: unzip, read the OPF spine, convert each XHTML file to text. `.txt`/`.md`
  pass through, split on headings.
- Pages with little or no extractable text (scanned) are detected, listed in
  `status`, and skipped in v1.
- Survey reads the TOC plus ~5 sampled chunks and a seed profile, and writes the
  book's schema. Profiles shipped in v1: `generic`, `ttrpg`.
- Audit sample rate defaults to 10%. If a stage's audit failure rate exceeds a
  threshold (default 20%), the sample rate is raised for that stage and a
  warning is printed.
- Fact ids `fNNNN` are stable and global across the project.

## Idempotency / resume

- Every work item has an input hash: sha256 of (prompt template name + its
  explicit `version: N` header + the item's input content + tier ceiling).
  Prompts are versioned by hand, so fixing a typo does not invalidate everything;
  bumping `version` does.
- An item is done if its output file exists and its stored input hash matches.
- All writes go to a temp file in the same directory and are renamed into place.
  Killing the process at any moment leaves only complete files or no file.
- Downstream items hash their upstream outputs, so a redo cascades only where
  content actually changed.

## CLI

```
wiki-writer init <project> <sources...> --agent claude|codex [--profile generic|ttrpg]
wiki-writer run [--project DIR] [-j 4] [--until STAGE] [--only STAGE] [--ceiling small|mid|large] [--yes] [--budget-tokens N]
wiki-writer status [--project DIR]
wiki-writer redo <stage> [--item ID] [--project DIR]
wiki-writer render [--project DIR]
wiki-writer lint [--project DIR]
wiki-writer doctor
```

- `-j` defaults to 4.
- After ingest, `run` prints a token estimate per stage and stops unless `--yes`
  is passed (or the estimate has already been accepted, recorded in state).
- `--budget-tokens` stops scheduling new items once spent tokens reach it.
- `--until organize` lets the user review the outline before paying for writing.
- `status` shows per stage: done / pending / blocked counts, escalation rate,
  tokens and dollars (where reported).
- `doctor` checks `pdftotext`, `pdfinfo`, `php`, and the chosen agent CLI.
- `--project` defaults to the current directory if it contains `wikify.yaml`.

## Rendering (our own, no pandoc)

`src/render.rs` converts `content/**/*.md` to DokuWiki syntax using
pulldown-cmark, writing `dokuwiki/data/pages/<ns>/<page>.txt`:

- Headings: `#` → `====== x ======`, `##` → `===== x =====`, … (6 `=` for level 1
  down to 2 for level 5).
- `**b**` → `**b**`, `*i*` → `//i//`, `` `c` `` → `''c''`, code blocks →
  `<code>…</code>`.
- Lists: two-space indent per level, `  * ` unordered, `  - ` ordered.
- Tables: header row `^ a ^ b ^`, body rows `| a | b |`.
- Links `[label](ns:page)` → `[[ns:page|label]]`; external links kept.
- Citation markers `[^fNNNN]` → footnote `((p. 42))` from the fact's page ref;
  identical refs on a page are merged.
- Page ids are lowercased with non-alphanumerics → `_`, as DokuWiki expects.
- The renderer records the hash of every page it writes in
  `state/render.json`. If a page on disk no longer matches its recorded hash (the
  user edited it in the wiki), it is skipped with a warning, not overwritten.
- After rendering, `make build` runs DokuWiki's `bin/indexer.php -c` to refresh
  search.

## Per-wiki layout

```
wikis/{project}/
  Makefile        # generated from a template in the crate
  wikify.yaml     # agent, models, profile, sources, jobs
  .gitignore      # sources/, dokuwiki/, state/ingest/
  sources/        # copies of the input documents
  state/          # pipeline state (not a dotdir: the repo .gitignore ignores .*)
    ingest/ schema.yaml notes/ audit/ entities.json outline.yaml
    render.json ledger.jsonl review.json
  content/        # Markdown source of truth, committed
  dokuwiki/       # disposable install, gitignored
```

Per-wiki Makefile targets:

- `install` — download a pinned DokuWiki release tarball, verify its sha256,
  unpack into `dokuwiki/`, write `conf/local.php` (title from project, nice URLs
  off, `useacl` on) and `conf/acl.auth.php` granting read to everyone and nothing
  else; run `wiki-writer doctor`.
- `serve` — `php -S 127.0.0.1:8080 -t dokuwiki` (localhost only; sources are
  often copyrighted).
- `wikify` — `wiki-writer run --project .`
- `build` — `wiki-writer render --project .` then the indexer.
- `status`, `lint`, `clean` (removes `dokuwiki/` only).

## Decisions

1. Rust driver in `tools/wiki-writer`, headless agent CLIs, no async runtime.
2. Our own Markdown → DokuWiki renderer in Rust. No pandoc.
3. Default concurrency 4; adaptive backoff on rate limits.
4. Binary installed to `~/bin` by `make install`; `make deps` installs poppler
   and php via brew.
5. Multiple sources per wiki are accepted by `init` from v1. Adding a source to
   an existing wiki (incremental organize) is out of scope for v1.
6. Default run is straight through; `--until` for checkpoints.
7. Token estimate printed after ingest; stops unless `--yes`.
8. Prompts carry an explicit `version: N`.
9. Scanned pages detected and skipped in v1; vision extraction and image/map
   extraction are later.
10. Profiles in v1: `generic`, `ttrpg`.
11. User edits in DokuWiki are protected (skip + warn); pulling them back into
    `content/` is later.
12. Committed: `content/`, `wikify.yaml`, `state/` except `state/ingest/`.
    Ignored: `sources/`, `dokuwiki/`, `state/ingest/`.
13. A Fake provider drives all tests.

## Out of scope for v1

Opencode provider; vision/OCR for scanned pages; image and map extraction;
incremental addition of sources to an existing wiki; pulling wiki edits back into
`content/`; profiles other than `generic` and `ttrpg`; any public hosting.
