#!/usr/bin/env bash
set -euo pipefail

# Config load order: explicit VOXPIPE_CONFIG, user config, repo config.
CONFIG_PATHS=(
  "${VOXPIPE_CONFIG:-}"
  "${XDG_CONFIG_HOME:-$HOME/.config}/voxpipe/voxpipe.env"
  "$HOME/.config/voxpipe/voxpipe.env"
  "$PWD/config/voxpipe.env"
)

for cfg in "${CONFIG_PATHS[@]}"; do
  if [[ -n "${cfg}" && -f "${cfg}" ]]; then
    set +u
    # shellcheck source=/dev/null
    . "${cfg}"
    set -u
    break
  fi
done

WHISPER_REPO="${WHISPER_REPO:-$HOME/whisper.cpp}"
WHISPER_BIN="${WHISPER_BIN:-$WHISPER_REPO/build/bin/whisper-cli}"
MODEL_NAME="${MODEL_NAME:-base.en-q5_1}"
MODEL="${MODEL:-$WHISPER_REPO/models/ggml-${MODEL_NAME}.bin}"

BT_CARD="bluez_card.CC_98_8B_F3_16_31"
BT_SRC="bluez_input.CC:98:8B:F3:16:31"

# We will discover the BT sink name dynamically (bluez_output....)
BT_SINK_PREFIX="bluez_output.CC:98:8B:F3:16:31"

HFP_PROFILE_PRIMARY="headset-head-unit"        # mSBC
HFP_PROFILE_FALLBACK="headset-head-unit-cvsd"  # CVSD

# A2DP profiles to try (order: best first)
A2DP_PROFILES=(
  "a2dp-sink"
  "a2dp-sink-aptx"
  "a2dp-sink-aac"
  "a2dp-sink-sbc_xq"
  "a2dp-sink-sbc"
)

THREADS="${THREADS:-8}"
export OPENBLAS_NUM_THREADS="${OPENBLAS_NUM_THREADS:-1}"
export OMP_NUM_THREADS="${OMP_NUM_THREADS:-$THREADS}"

# Dynamic record-until-silence knobs
CHUNK_SECS="${CHUNK_SECS:-0.6}"
MIN_SECS="${MIN_SECS:-1.2}"
SILENT_CHUNKS_TO_STOP="${SILENT_CHUNKS_TO_STOP:-3}"
SILENCE_DBFS="${SILENCE_DBFS:--45}"
MAX_SECS="${MAX_SECS:-12}"

WAV="/tmp/asr.wav"
OUT="/tmp/asr"
TXT="/tmp/asr.txt"
LOG="/tmp/whisper_ptt_host.log"

die() { echo "whisper-ptt: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null || die "missing '$1' (install it)"; }

need pactl
need pw-record
need wl-copy
need ffmpeg

[ -x "$WHISPER_BIN" ] || die "missing whisper-cli at $WHISPER_BIN"
[ -f "$MODEL" ]   || die "missing model at $MODEL (run: scripts/model-download.sh ${MODEL_NAME})"

# --- Audible beep (force to BT sink if possible) ---
BEEP_START="/usr/share/sounds/freedesktop/stereo/message-new-instant.oga"
BEEP_STOP="/usr/share/sounds/freedesktop/stereo/complete.oga"

bt_sink_name() {
  pactl list short sinks | awk -v pfx="$BT_SINK_PREFIX" '$2 ~ "^"pfx {print $2; exit}'
}

play_beep() {
  local file="$1"
  if command -v paplay >/dev/null && [ -f "$file" ]; then
    local sink
    sink="$(bt_sink_name || true)"
    if [ -n "${sink:-}" ]; then
      # temporarily nudge volume up a bit for audibility (doesn't persist)
      pactl set-sink-volume "$sink" 70% >/dev/null 2>&1 || true
      paplay --device="$sink" "$file" >/dev/null 2>&1 || true
      return 0
    fi
    paplay "$file" >/dev/null 2>&1 || true
    return 0
  fi
  printf '\a' || true
}

set_profile_try() {
  local prof="$1"
  pactl set-card-profile "$BT_CARD" "$prof" >>"$LOG" 2>&1
}

restore_a2dp() {
  echo "[3/4] Restoring A2DP (good headphone audio)..." | tee -a "$LOG" >&2
  for p in "${A2DP_PROFILES[@]}"; do
    if set_profile_try "$p"; then
      return 0
    fi
  done
  # If none worked, don't fail the whole run.
  echo "note: could not restore A2DP (leaving current profile)" | tee -a "$LOG" >&2
  return 0
}

ensure_hfp() {
  echo "[0/4] Switching headset to HFP (mic)..." | tee -a "$LOG" >&2
  if ! set_profile_try "$HFP_PROFILE_PRIMARY"; then
    echo "note: mSBC failed; trying CVSD..." | tee -a "$LOG" >&2
    set_profile_try "$HFP_PROFILE_FALLBACK" || die "could not switch to HFP; see $LOG"
  fi

  sleep 0.6

  pactl list short sources | awk '{print $2}' | grep -Fxq "$BT_SRC" \
    || die "BT mic source not found: $BT_SRC (run: pactl list short sources)"

  pactl set-default-source "$BT_SRC" >>"$LOG" 2>&1 || true
}

chunk_dbfs() {
  local f="$1"
  local mv
  mv="$(ffmpeg -hide_banner -nostats -i "$f" -af volumedetect -f null - 2>&1 \
        | awk -F': ' '/mean_volume/ {print $2}' | awk '{print $1}' | tail -n1 || true)"
  [ -n "${mv:-}" ] || return 1
  echo "${mv%.*}"
}

: >"$LOG"
rm -f "$WAV" "$TXT"

ensure_hfp

# Try beep after HFP switch (forced to BT sink); if you still don't hear it,
# you can move this before ensure_hfp.
play_beep "$BEEP_START"

echo "[1/4] Recording until silence..." | tee -a "$LOG" >&2

RAW="/tmp/asr.raw"
rm -f "$RAW"

speech_started=0
silent_run=0
total_secs=0

while :; do
  chunk="/tmp/asr_chunk.wav"
  rm -f "$chunk"

  timeout "$CHUNK_SECS" pw-record --rate 16000 --channels 1 "$chunk" >>"$LOG" 2>&1 || true

  # Append audio if we got anything
  if [ -s "$chunk" ]; then
    ffmpeg -hide_banner -loglevel error -y -i "$chunk" -f s16le -ac 1 -ar 16000 - >>"$RAW" || true

    db="$(chunk_dbfs "$chunk" || true)"
    rm -f "$chunk"

    if [ -n "${db:-}" ]; then
      if [ "$speech_started" -eq 0 ] && [ "$db" -gt "$SILENCE_DBFS" ]; then
        speech_started=1
      fi

      if [ "$speech_started" -eq 1 ]; then
        if [ "$db" -le "$SILENCE_DBFS" ]; then
          silent_run=$((silent_run + 1))
        else
          silent_run=0
        fi
      fi
    fi
  else
    rm -f "$chunk"
  fi

  # update time
  total_secs=$(python3 - <<PY
t=$total_secs
c=$CHUNK_SECS
print(round(t+c,3))
PY
)

  min_ok=$(python3 - <<PY
print(1 if $total_secs >= $MIN_SECS else 0)
PY
)
  max_ok=$(python3 - <<PY
print(1 if $total_secs >= $MAX_SECS else 0)
PY
)

  if [ "$max_ok" -eq 1 ]; then
    break
  fi
  if [ "$speech_started" -eq 1 ] && [ "$min_ok" -eq 1 ] && [ "$silent_run" -ge "$SILENT_CHUNKS_TO_STOP" ]; then
    break
  fi
done

# Finalize WAV from raw PCM
ffmpeg -hide_banner -loglevel error -y \
  -f s16le -ar 16000 -ac 1 -i "$RAW" \
  -c:a pcm_s16le "$WAV" >>"$LOG" 2>&1 || die "failed to finalize wav; see $LOG"

rm -f "$RAW"

[ -s "$WAV" ] || die "no audio captured (see $LOG)"

play_beep "$BEEP_STOP"

echo "[2/4] Transcribing..." | tee -a "$LOG" >&2
"$WHISPER_BIN" \
  -m "$MODEL" \
  -f "$WAV" \
  -t "$THREADS" \
  --no-timestamps \
  --beam-size 1 \
  --best-of 1 \
  --no-fallback \
  -otxt -of "$OUT" >>"$LOG" 2>&1 || die "whisper failed (see $LOG)"

[ -s "$TXT" ] || die "no output text (see $LOG)"

TEXT="$(tr '\n' ' ' < "$TXT" | sed 's/[[:space:]]\+/ /g; s/^ //; s/ $//')"
printf "%s" "$TEXT" | wl-copy
wl-paste -n

restore_a2dp

echo "[4/4] Done (copied to clipboard)." | tee -a "$LOG" >&2
