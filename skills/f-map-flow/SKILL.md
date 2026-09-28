---
name: f-map-flow
description: Trace requested code behavior through a repository and write a Markdown waterfall with actual symbols, branch paths, and pseudocode. Use for PRs, files, file sets, or directed code-flow investigations.
metadata:
  short-description: Map code flows to Markdown
---

# Map Code Flows

Trace the requested behavior through the repository and create a readable Markdown document under `docs/flows/`. The document should let a reader follow execution in order and identify the real functions, methods, modules, and classes responsible for it.

## Identify and trace the target

- Accept a PR, one or more file paths, or a behavior, symbol, or other code directive. If no target is supplied, use the current repository and ask what behavior to trace.
- For a PR, inspect its diff to find changed behaviors, then follow each relevant behavior into surrounding code. For files or directives, start at the named symbol or find the most plausible entry point.
- Read implementation and call sites; do not infer behavior from names alone. Follow relevant data and calls to the meaningful result, output, or error handling. State where the trace stops when it reaches an external or unavailable boundary.
- Use actual function and method names and identify their containing module, class, or file. Cite repository-relative paths and line numbers beside the relevant steps. Do not invent names for anonymous or external code; identify the closest real symbol or boundary.
- Include conditions, values passed between steps, transformations, returns, and failures that materially affect the requested behavior. Distinguish observed behavior from assumptions when the source is unclear.

## Make linear waterfalls

- Organize each distinct entry behavior as its own flow in the document.
- Show a shared trunk once. At every behavior-changing conditional, match, or relevant error outcome, create a separate linear waterfall for each outcome. Trace each branch to its meaningful result or to where execution rejoins; show any common suffix after the rejoin once.
- Include error and early-return paths when they change the requested behavior. Omit trivial guards only when they do not materially affect that behavior. For nested decisions, repeat this split-and-trace structure.
- Use concise numbered Markdown steps for waterfalls and fenced pseudocode to express execution order. Keep real symbol names in the pseudocode and annotate each step with its source reference. A diagram is optional; use one only when it clarifies the paths better than the text.
- Keep the trace scoped to the request. Do not map unrelated code or add unsupported behavior.

## Write and report the document

- Create `docs/flows/` if it does not exist. Write one Markdown file per request as `docs/flows/<YYMMDD-HHMMSS>_<RelevantName>.md`, using the current local time and a concise PascalCase name derived from the target behavior.
- Include a descriptive title and a brief scope statement, then organize flows by entry behavior. For each flow, present the shared trunk, branch waterfalls, any shared continuation, and concise notes on important data or boundaries.
- Do not generate HTML or open a browser. After writing the file, report its path.
