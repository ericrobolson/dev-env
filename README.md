# dev-env

A reproducible development environment with AI-assisted feature building tools.

## WikiWriter

Install the local wiki generator with `make install-wiki-writer`. Start a project with `wikify.sh campaign rulebook.pdf supplement.epub`, or run `wikify.sh --project wikis/campaign` to resume one. See the [WikiWriter guide](tools/wiki-writer/README.md) and [architecture](tools/wiki-writer/ARCHITECTURE.md) for setup, stages, and limits.

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
