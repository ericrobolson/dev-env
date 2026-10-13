#!/usr/bin/env bash
set -euo pipefail

# Distill a book into one large Markdown doc. Prints a prompt for the host agent
# (like orchestrate.sh) that has it split the book into chunks, hand each chunk to
# a cheaper subagent for a summary, and stitch the results into
# docs/<timestamp>_<BookName>.md. If you give a focus, it is folded into every
# chunk summary and the final doc; otherwise the doc is a summary plus key facts.
# Edit the models for each provider below.

usage() {
    cat >&2 <<'USAGE'
Usage: distill-book.sh <book-path> [focus...]

Example: distill-book.sh ~/books/Deep_Work.pdf "focus on actionable habits"
USAGE
}

fail() {
    printf 'Error: %s\n' "$1" >&2
    exit 1
}

if [[ $# -lt 1 ]]; then
    usage
    exit 2
fi

book_path="$1"
shift
focus="$*"

[[ -f "$book_path" && -r "$book_path" ]] || fail "book does not exist or is not readable: $book_path"
# Make relative paths absolute so the agent can find the file from anywhere.
[[ "$book_path" == /* ]] || book_path="$PWD/$book_path"

printf 'Which agent? [claude/codex]: '
read -r agent

case "$agent" in
    claude)
        orchestrator=claude-opus-5-5
        summarizer=claude-sonnet-5-5
        ;;
    codex)
        orchestrator=gpt-6-astra
        summarizer=gpt-6-luna
        ;;
    *)
        fail "unknown agent: $agent"
        ;;
esac

file_name=$(basename "$book_path")
book_name=$(printf '%s' "${file_name%.*}" | LC_ALL=C tr -cs '[:alnum:]' '_' | sed -E 's/^_+//; s/_+$//')
[[ -n "$book_name" ]] || fail 'book file name must contain at least one letter or number'

mkdir -p docs
timestamp=$(date '+%y%m%d-%H%M%S')
out_file="$PWD/docs/${timestamp}_${book_name}.md"
[[ ! -e "$out_file" ]] || fail "output file already exists: $out_file"

if [[ -n "${focus//[[:space:]]/}" ]]; then
    goal="Fold in this request from me, and treat it as the main lens for every summary: $focus"
    sections="Organize the doc around that request. Keep a short overall summary at the top, then whatever sections serve the request best."
else
    goal='I gave no particular focus, so default to a summary plus key facts.'
    sections='Use these sections: Overview (what the book is, who wrote it, and its thesis), Summary (chapter by chapter or part by part), Key Facts (concrete claims, figures, names, dates, definitions, and frameworks), and Takeaways.'
fi

cat <<PROMPT

Distill the book at $book_path into one large Markdown doc at $out_file.

$goal

Steps:
1. Read just enough of the book to find its structure (contents, chapters, parts). If it is a PDF or EPUB, extract the text first into the scratchpad or .tmp, never next to the book.
2. Split the book into chunks along chapter or part boundaries. Size each chunk to fit comfortably in one $summarizer context window, and number them in order.
3. Spin up one $summarizer subagent per chunk, in parallel batches. Set the model explicitly on every subagent, because default subagents inherit the caller's model ($orchestrator runs this session). Tell each one which chunk to read (file and range), the goal above, and to return a summary in Markdown with specifics rather than generalities: keep names, numbers, quotes worth keeping, and definitions. It must not invent anything the chunk does not say.
4. Collect the chunk summaries in order. If any came back thin or off-target, send that chunk back once.
5. Write $out_file. $sections Keep every chunk's substance. The doc is meant to be long and thorough, so do not compress it further than the summaries already are. Start it with a title and a line naming the source file.
6. Reply with the path to the doc and a three-line description of what is in it, not the doc itself.

Rules:
- Run without stopping to ask me anything. When a choice is unclear, make the call and note it at the end of the doc.
- $orchestrator only splits, dispatches, and assembles. $summarizer subagents do the reading.
- Only write to $out_file and scratch files. Do not modify the book.
PROMPT

printf '\nWill write %s\n' "$out_file" >&2
