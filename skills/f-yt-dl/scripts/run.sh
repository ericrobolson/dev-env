#!/usr/bin/env bash
set -euo pipefail

# TODO: restore CoreML (Neural Engine) encoder for faster transcription
#   1. Install nix: curl ... https://install.determinate.systems/nix | sh
#   2. Swap yt-dlp from brew → nix for reproducibility
#   3. Rebuild whisper.cpp with -DWHISPER_COREML=ON
#   4. Generate CoreML model: bash models/generate-coreml-model.sh small.en
#      (requires: pip install torch coremltools)
#   See commented nix section below.

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

LOGDIR="${LOGDIR:-$HOME/.cache/f-yt-dl}"
mkdir -p "$LOGDIR"
TIMESTAMP=$(date +%y%m%d-%H%M%S)
LOGFILE="$LOGDIR/$TIMESTAMP.log"

log()     { printf "${GREEN}[ok]${NC} %s\n" "$*"; }
warn()    { printf "${YELLOW}[--]${NC} %s\n" "$*"; }
fail()    { printf "${RED}[FAIL]${NC} %s\n" "$*"; printf "\nFull log: %s\n" "$LOGFILE"; exit 1; }
step()    { printf "\n${BOLD}>>> %s${NC}\n" "$*"; }
detail()  { printf "    ${BLUE}%s${NC}\n" "$*"; }

run_cmd() {
    local desc="$1"; shift
    detail "$desc"
    printf "\n──── BEGIN: %s ────\n" "$(date +%H:%M:%S)" >> "$LOGFILE"
    printf "  cmd: %s\n" "$*" >> "$LOGFILE"
    "$@" 2>&1 | tee -a "$LOGFILE" || {
        printf "\n──── FAILED: %s ────\n" "$(date +%H:%M:%S)" >> "$LOGFILE"
        return 1
    }
    printf "\n────  DONE: %s ────\n" "$(date +%H:%M:%S)" >> "$LOGFILE"
    return 0
}

URL="${1:-}"
WHISPER_DIR="${WHISPER_DIR:-$HOME/.local/share/whisper.cpp}"
MODEL="${MODEL:-small.en}"
REPO_ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
OUTDIR="${OUTDIR:-$REPO_ROOT/yt-dl}"
mkdir -p "$OUTDIR"

[[ -n "$URL" ]] || fail "Usage: $0 <URL>"

detail "Logging to $LOGFILE"

# # ── nix (disabled: needs sudo for first install) ────────────────────
# # Uncomment once nix is installed on the system:
# #   curl --proto '=https' --tlsv1.2 -sSf -L \
# #     https://install.determinate.systems/nix | sh -s -- install
# #
# source_nix() {
#     for src in \
#         "$HOME/.nix-profile/etc/profile.d/nix.sh" \
#         "/nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh"; do
#         if [[ -f "$src" ]]; then
#             . "$src"
#             return 0
#         fi
#     done
#     return 1
# }
#
# step "nix"
# if command -v nix &>/dev/null; then
#     log "nix $(nix --version 2>&1 | head -1)"
# else
#     if command -v brew &>/dev/null && brew info nix &>/dev/null 2>&1; then
#         run_cmd "Installing nix via Homebrew" brew install nix
#         source_nix
#         command -v nix &>/dev/null && log "nix installed via brew: $(nix --version 2>&1 | head -1)" && :
#     fi
#     if ! command -v nix &>/dev/null; then
#         fail "nix not installed. Run: curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install"
#     fi
# fi

# ── 1. yt-dlp ─────────────────────────────────────────────────────────

step "yt-dlp"
if command -v yt-dlp &>/dev/null; then
    log "yt-dlp $(yt-dlp --version)"
else
    run_cmd "Installing yt-dlp via Homebrew" brew install yt-dlp
    command -v yt-dlp &>/dev/null || fail "yt-dlp install failed — check log: $LOGFILE"
    log "yt-dlp $(yt-dlp --version)"
fi

run_ytdlp() {
    printf "\n──── BEGIN: yt-dlp ────\n" >> "$LOGFILE"
    printf "  cmd: yt-dlp %s\n" "$*" >> "$LOGFILE"
    yt-dlp "$@" 2>&1 | tee -a "$LOGFILE"
    local rc=${PIPESTATUS[0]}
    printf "\n────  DONE: yt-dlp ────\n" >> "$LOGFILE"
    return $rc
}

# ── 2. whisper.cpp + Metal build ─────────────────────────────────────

step "whisper.cpp"
if [[ ! -d "$WHISPER_DIR" ]]; then
    run_cmd "Cloning whisper.cpp" git clone --depth 1 https://github.com/ggerganov/whisper.cpp "$WHISPER_DIR"
fi

WHISPER_BIN="$WHISPER_DIR/build/bin/whisper-cli"
if [[ ! -x "$WHISPER_BIN" ]] || \
   grep -q 'WHISPER_COREML:BOOL=ON' "$WHISPER_DIR/build/CMakeCache.txt" 2>/dev/null; then
    if [[ -d "$WHISPER_DIR/build" ]]; then
        run_cmd "Removing stale CMake cache (CoreML→Metal migration)" \
            rm -rf "$WHISPER_DIR/build"
    fi
    run_cmd "Configuring CMake (Metal GPU)" \
        cmake -B "$WHISPER_DIR/build" -S "$WHISPER_DIR" -DWHISPER_COREML=OFF
    run_cmd "Building whisper.cpp (this may take several minutes)" \
        cmake --build "$WHISPER_DIR/build" -j
    [[ -x "$WHISPER_BIN" ]] || fail "whisper-cli build failed — check log: $LOGFILE"
fi
log "whisper-cli (Metal GPU)"

# ── 3. model ──────────────────────────────────────────────────────────

step "model ($MODEL)"
MODEL_PATH="$WHISPER_DIR/models/ggml-$MODEL.bin"
if [[ ! -f "$MODEL_PATH" ]]; then
    run_cmd "Downloading whisper model: $MODEL" \
        bash "$WHISPER_DIR/models/download-ggml-model.sh" "$MODEL"
    [[ -f "$MODEL_PATH" ]] || fail "Model download failed — check log: $LOGFILE"
fi
log "$MODEL ($(du -h "$MODEL_PATH" | cut -f1))"

# ── 4. metadata ───────────────────────────────────────────────────────

step "metadata"
VIDEO_TITLE=$(yt-dlp --print title "$URL" 2>/dev/null || echo "unknown")
UPLOAD_DATE=$(yt-dlp --print "%(upload_date)s" "$URL" 2>/dev/null || echo "")
if [[ -n "$UPLOAD_DATE" ]] && [[ "$UPLOAD_DATE" =~ ^[0-9]{8}$ ]]; then
    FILE_DATE=$(python3 -c "from datetime import datetime; d=datetime.strptime('$UPLOAD_DATE','%Y%m%d'); print(d.strftime('%y%m%d'))")
else
    FILE_DATE=$(date +%y%m%d)
    warn "upload date unavailable, using today: $FILE_DATE"
fi
SANITIZED=$(echo "$VIDEO_TITLE" \
    | tr -dc 'a-zA-Z0-9 ' \
    | tr '[:upper:]' '[:lower:]' \
    | sed 's/ \+/ /g' \
    | sed 's/^ //;s/ $//' \
    | tr ' ' '-' \
    | head -c 80)
BASENAME="${FILE_DATE}_${SANITIZED}"
AUDIO="$OUTDIR/${BASENAME}.wav"
MD="$OUTDIR/${BASENAME}.md"
log "title: $VIDEO_TITLE"
log "date:  $FILE_DATE"
log "file:  $BASENAME"

# ── 5. audio ──────────────────────────────────────────────────────────

step "audio"
if [[ ! -f "$AUDIO" ]]; then
    TEMP_AUDIO="$OUTDIR/.tmp_${BASENAME}.wav"
    run_ytdlp -f bestaudio --extract-audio --audio-format wav --audio-quality 0 -o "$TEMP_AUDIO" "$URL"
    [[ -f "$TEMP_AUDIO" ]] || fail "Audio download failed — check log: $LOGFILE"
    mv "$TEMP_AUDIO" "$AUDIO"
    log "$AUDIO ($(du -h "$AUDIO" | cut -f1))"
else
    log "cached: $AUDIO ($(du -h "$AUDIO" | cut -f1))"
fi

# ── 6. transcribe ─────────────────────────────────────────────────────

step "transcribe (Metal GPU)"

if [[ -f "$MD" ]]; then
    log "cached: $MD ($(wc -l < "$MD" | tr -d ' ') lines)"
else
    TXT_TMP="${AUDIO%.wav}.txt.tmp"
    SRT_TMP="${AUDIO%.wav}.srt.tmp"
    run_cmd "Running whisper with Metal GPU" \
        "$WHISPER_BIN" -m "$MODEL_PATH" -f "$AUDIO" --output-txt --output-file "${AUDIO%.wav}.tmp"
    TXT_OUT=$(ls "${AUDIO%.wav}"*.txt.tmp 2>/dev/null | head -1)
    [[ -f "$TXT_OUT" ]] || fail "Transcription failed — check log: $LOGFILE"
    {
        printf "# %s\n\n" "$VIDEO_TITLE"
        printf "[Source](%s)\n\n" "$URL"
        printf "---\n\n"
        cat "$TXT_OUT"
    } > "$MD"
    rm -f "$TXT_OUT" 2>/dev/null || true
    log "$MD ($(wc -l < "$MD" | tr -d ' ') lines)"
fi

echo ""
log "Done: $MD"
detail "Audio: $AUDIO"
detail "Full log: $LOGFILE"