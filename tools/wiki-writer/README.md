# WikiWriter

WikiWriter turns local PDF, EPUB, UTF-8 text, and Markdown sources into an editable Markdown wiki, then renders it for a local DokuWiki installation. The Rust crate uses edition 2024 and requires Rust 1.96 or newer, `pdfinfo` and `pdftotext` from Poppler, PHP for DokuWiki, and an authenticated `claude` or `codex` CLI for model stages. On macOS, install local dependencies with `brew install poppler php`. On Linux, install equivalent Poppler and PHP packages with your package manager.

From this repository, run `make install-wiki-writer`. The crate installer places `wiki-writer` in `~/bin`; use `export PATH="$HOME/bin:$PATH"` in your shell if needed. `make install-workflows path=/destination` copies `wikify.sh` with the other workflows.

## Start a project

```sh
wikify.sh campaign rulebook.pdf supplement.epub notes.md
# Or directly:
wiki-writer init campaign rulebook.pdf supplement.epub notes.md --agent codex --profile ttrpg
cd wikis/campaign
wiki-writer run
wiki-writer run --yes --ceiling small --budget-tokens 20000
```

Initialization copies each source into `wikis/campaign/sources/` and does not run a model. `run` ingests sources and prints token estimates first. A changed ingest/config/template/ceiling fingerprint needs `--yes` before any model call. The budget counts completed input and output tokens, including retries; calls already running can exceed it. `--ceiling` limits the model tier to `small`, `mid`, or `large`. `-j` sets concurrent jobs for that invocation.
`run` exits 4 when acceptance is required, 3 when work remains pending or blocked, and 0 when the selected work completes.

The `wikify.yaml` file supports `generic` and `ttrpg` profiles and model IDs for the selected provider. For example:

```yaml
agent: codex
profile: generic
models: {small: gpt-6-luna, mid: gpt-6.1-sol, large: gpt-6-astra}
```

The saved file also contains `version`, `project`, copied source metadata, jobs, audit rates, and timeout. Do not add or reorder sources in the file after initialization. Changes to copied source bytes are detected at ingest.

## Run and inspect

The stage order is `ingest`, `survey`, `extract`, `audit`, `resolve`, `organize`, `write`, `link`, `review`. A normal run stops after review. `--until STAGE` runs through a stage; `--only STAGE` runs a stage using existing prerequisites. Use `wiki-writer status`, `wiki-writer redo extract --item c0001`, or `wiki-writer redo write --item items:copper` to inspect or request focused work. `wiki-writer lint` reports broken links and missing citations; `wiki-writer render` explicitly converts the Markdown source to DokuWiki pages. `wiki-writer doctor` checks local executables. Small PDF/EPUB/text pages with fewer than 40 non-whitespace characters are listed as skipped and omitted from model chunks.

Each generated wiki has a Makefile: `make install` downloads the pinned, checksum-verified DokuWiki release, `make wikify` runs the pipeline, `make build` renders then indexes, `make status` and `make lint` inspect it, `make serve` binds PHP to `127.0.0.1:8080`, and `make clean` removes only the DokuWiki installation. Install may run before or after rendering; it preserves rendered pages. The installer initializes an empty local authentication database and grants anonymous read access only. Edit `content/**/*.md` as the source of truth. When writer inputs change, WikiWriter updates the draft and preserves a hand-edited content page. Automatic links and review redos also leave hand-edited pages alone. DokuWiki page edits are protected from renderer overwrites and are not pulled back into Markdown.

OCR/vision, images/maps, adding sources after initialization, syncing browser edits back to Markdown, profiles beyond `generic` and `ttrpg`, public hosting, and Opencode are outside this tool's scope. The Codex adapter limits tool access with read-only sandboxing and feature flags, and rejects tool-call events, but a live run is still needed to verify the installed CLI's behavior.
