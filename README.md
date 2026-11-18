# dev-env

A reproducible development environment with AI-assisted feature building tools.

## WikiWriter

Install the local wiki generator with `make install-wiki-writer`. Start a project with `wikify.sh campaign rulebook.pdf supplement.epub`, or run `wikify.sh --project wikis/campaign` to resume one. See the [WikiWriter guide](tools/wiki-writer/README.md) and [architecture](tools/wiki-writer/ARCHITECTURE.md) for setup, stages, and limits.

## Guided tutorials

`workflows/tutor.sh` turns a plan file or prompt into a resumable tutorial where
you write the code and the agent teaches, hints, and reviews. Start with
`claude "$(workflows/tutor.sh new ideas/plan.md)"` and resume any later session
with `claude "$(workflows/tutor.sh resume)"` (or the `f-tutor` skill). State lives
in `tutor/<id>/` (`TUTOR.md` protocol, `curriculum.md` checkboxes, `lessons/`,
`log.md`). Each step's starting point is a git snapshot under `refs/tutor/`, so
`tutor.sh diff` shows exactly what you changed, including uncommitted files and
mid-step commits. `tutor.sh status` and `tutor.sh list` show progress.

## Narrated explainers

Run `workflows/explainer.sh "Review the architecture of this service"` and give
its printed workflow to Claude, Codex, or OpenCode. It checks real CPU speech
synthesis on every run and automatically installs Kokoro, an isolated Python
runtime, and a mono MP3 encoder under `~/local-models/tts/kokoro-82m` (override
with `EXPLAINER_TTS_DIR`). First setup requires network access and `curl`.

The workflow targets a 3–4 minute offline browser slideshow, with playback and
slide controls, CSS animations, a transcript, and short source notes. Output goes
under the current directory's `explainer/<timestamp>_<name>_<suffix>/`. Budget
15–30 minutes for the agent's work, plus first installation. Audio uses scoped
Git LFS attributes when LFS is available, otherwise a local ignore rule.

Use `workflows/explainer.sh --setup-only` to install/check speech alone, or
`workflows/explainer.sh --render /path/to/deck.json` to regenerate cached narration.
Keep `explainer_tts.py` beside the shell script when copying workflows. The
runtime follows [kokoro-onnx](https://github.com/thewh1teagle/kokoro-onnx), its
[matched model assets](https://github.com/thewh1teagle/kokoro-onnx/releases/tag/model-files-v1.1),
and [lameenc](https://github.com/chrisstaite/lameenc).

## UML architecture diagrams

Run `workflows/uml-diagram.sh src/ "focus on the API"` and give its printed
workflow to Claude, Codex, or OpenCode. It writes
`docs/uml/<timestamp>_<Name>.md` in the current directory, embedding SVG
diagrams rendered from PlantUML sources in the sibling `<timestamp>_<Name>/`:
a context view, a component view (fields, methods/commands, relationships), and
a table citing `file:line` evidence for every arrow. With no path it diagrams
the current directory.

The script installs a pinned, checksum-verified [PlantUML](https://github.com/plantuml/plantuml)
under `~/local-tools/plantuml/<version>` (override with `UML_PLANTUML_DIR`) and
skips the download when it is already there. macOS arm64 and Linux use the native
build, so no Java or Graphviz is needed; elsewhere the jar needs Java 11+. Use
`--setup-only` to install alone, or `--render docs/uml/<file>.md` to re-render
and check a doc's diagrams after editing them.
