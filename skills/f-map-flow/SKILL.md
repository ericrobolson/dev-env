---
name: f-map-flow
description: Trace a code path through a repository and create a readable standalone HTML page with a diagram and pseudocode. Use for mapping entry points, features, requests, data flow, and branching behavior.
metadata:
  short-description: Map code flows to HTML
---

# Map a Code Flow

Trace the requested behavior through the repository and produce a single, self-contained HTML page in `/tmp/`. The page should let a reader understand both the path through the code and the functions, methods, modules, and classes responsible for it.

## Trace the behavior

- Start from the named entry point or find the most plausible one. Follow calls and relevant data until the requested behavior reaches its meaningful result, output, or error handling.
- Read the implementation rather than inferring behavior from names. Include only code that helps explain this flow; note any important boundary where the trace stops.
- Record actual function and method names, their containing module or class, and file path plus line number. Distinguish observed behavior from assumptions when the source leaves something unclear.
- Show the conditions that select branches, where branches rejoin, and the important values passed between steps. Include error and return paths when they materially change the flow.

## Build the page

Create one HTML file under `/tmp/` containing:

1. A descriptive title and a short scope statement.
2. A flow diagram, preferably arranged from top to bottom, with branches splitting horizontally and rejoining where the code converges. Choose a simple format that makes the actual flow easiest to follow. Mermaid or UML is optional; use it only when it makes the diagram significantly clearer. If using a library or external resource would make the page non-self-contained, draw the diagram with inline HTML/CSS/SVG instead.
3. Text pseudocode in execution order. Preserve actual function/method names and identify their modules or classes. Make branching, calls, and returned data explicit.
4. Brief notes for important data transformations, conditions, and failure behavior.
5. Source references next to the relevant diagram steps and pseudocode, using repository-relative paths and line numbers.

Keep the page legible at normal browser sizes. Use inline styles and any scripts needed for rendering; do not rely on network resources. Avoid adding detail that is not supported by the code.

## Open and report

After writing the page, open it in the user's browser using an available browser or operating-system mechanism. Then report the page path. If opening the browser is unavailable or blocked, say so clearly and provide the path so the user can open it manually.
