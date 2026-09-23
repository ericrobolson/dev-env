---
name: f-code-map
description: Map a folder’s software architecture as a nested list of language-agnostic modules, with one responsibility sentence per entry. Use when the user asks for a folder, codebase, or module map, or invokes /f-code-map.
---

# Code Map

Inspect a target folder and produce a concise, verified map of its meaningful software modules.

## Choose the detail level

If the user has not specified a mode, ask:

> Which detail level do you want?
> - **Architecture overview (recommended):** major subsystems and meaningful architectural boundaries; omit individual utility files and detailed test subfolders.
> - **Comprehensive:** include meaningful submodules, key entry points, test suites, and standalone tools.

If the user specifies the mode in the request, use it without asking.

## Identify the target

Use the path supplied by the user. If no path is supplied, map the current working directory. If the requested path is missing or ambiguous, ask which folder to map.

## Inspect and map

1. Inspect the top-level structure, language and build manifests, imports or module declarations, and relevant documentation.
2. Read representative implementation files to verify what each candidate module owns. Follow the project’s actual language conventions; do not assume that a module is always a directory or that directory names prove responsibility.
3. Identify cohesive modules and useful parent-child relationships. Include a grouping directory only when it clarifies the architecture; avoid repeating its responsibility in child descriptions.
4. Apply the selected detail level:
   - **Architecture overview:** include the folder’s major subsystems and meaningful boundaries, such as independently owned modes or distinct platform adapters. Group supporting tests and tools at subsystem level; omit individual utility files and detailed test subfolders.
   - **Comprehensive:** descend into meaningful submodules, include key entry points, distinct test suites, and standalone tools when they have their own responsibility.
5. Exclude empty or reserved directories, generated output, vendored dependencies, build artifacts, and asset-only collections unless they have a distinct role in the software’s behavior.
6. Do not modify files or run builds and tests just to produce the map.

## Output

Return only a nested Markdown bullet list. Always label the root exactly `folder`, regardless of its actual directory name. Label other entries with paths relative to the target folder; use paths consistently and put parent entries before their children.

Use this format:

```text
- folder — One sentence describing the target folder.
  - src/ — One sentence describing this group, if useful.
    - src/path/to/module — One sentence describing its responsibility.
```

Give every entry exactly one concise, factual sentence about its responsibility. Include only modules whose purpose is supported by inspected contents. Do not add an introduction, conclusion, recommendations, code excerpts, or speculative descriptions.
