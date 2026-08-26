---
name: f-yt-dl
description: Download a YouTube video's audio and transcribe it using whisper.cpp with CoreML (Apple Neural Engine) hardware acceleration. Use when user says 'f-yt-dl', 'transcribe this URL', 'transcribe YouTube', 'get transcript', or 'download and transcribe'.
---

# f-yt-dl

Download audio from a YouTube URL and transcribe it with whisper.cpp using CoreML passthrough to the Apple Neural Engine. The script is idempotent: each step checks whether work is already done before running.

## Script

The pipeline lives at `scripts/run.sh` relative to this skill. It handles:

1. nix installation (brew, then Determinate installer fallback)
2. yt-dlp via Homebrew (lazy, no global install)
3. whisper.cpp clone + Metal GPU build
4. whisper model download (default `small.en`)
5. video metadata extraction (title, upload date)
6. audio extraction (wav, cached, named `YYMMDD_TitleSlug.wav`)
7. transcription as markdown (cached, named `YYMMDD_TitleSlug.md`) with YouTube URL link at top

## Invocation

Run from the skill directory:

```
bash scripts/run.sh <URL>
```

### Output

Audio saved as `yt-dl/<YYMMDD>_<VideoTitleSlug>.wav`. Transcript saved as `yt-dl/<YYMMDD>_<VideoTitleSlug>.md` — a markdown file with the YouTube URL link as its first line, followed by the full transcript. The date is the video's upload date (preferred) or today's date (fallback).

### Environment overrides

| Variable | Default | Description |
|----------|---------|-------------|
| `MODEL` | `small.en` | Whisper model (tiny, base, small, medium, large) |
| `WHISPER_DIR` | `~/.local/share/whisper.cpp` | Build + model cache location |
| `OUTDIR` | `yt-dl/` (repo root) | Output directory for audio + transcript |

### Example

```
MODEL=medium bash scripts/run.sh https://www.youtube.com/watch?v=...
```

## Agent workflow

1. Confirm the URL is valid and non-empty. Ask if the user hasn't provided one.
2. Ensure `scripts/run.sh` is executable: `chmod +x scripts/run.sh`.
3. Run the script via Bash from the skill directory. Show the script's own step-by-step log output to the user.
4. On success, tell the user where the `.md` transcript was saved (the file also contains a link to the source YouTube URL at the top).
5. On failure, relay the error and ask the user for direction (don't retry silently).