#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd -P)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." >/dev/null 2>&1 && pwd -P)"

if ! command -v make >/dev/null 2>&1; then
  echo "error: 'make' is required" >&2
  exit 1
fi

if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "error: 'ffmpeg' is required for sample conversion" >&2
  exit 1
fi

SAMPLES_DIR="${VOXPIPE_SAMPLES_DIR:-$HOME/.cache/voxpipe/samples}"
mkdir -p "$SAMPLES_DIR"

echo "Downloading extra development samples via vendor Makefile..."
make -C "$REPO_ROOT/vendor" -j samples

echo "Converting downloaded samples to 16-bit WAV in: $SAMPLES_DIR"
shopt -s nullglob
for input in "$REPO_ROOT"/vendor/samples/*.{ogg,mp3,flac,wav}; do
  base="$(basename "$input")"
  stem="${base%.*}"
  output="$SAMPLES_DIR/$stem.wav"
  ffmpeg -y -i "$input" -acodec pcm_s16le -ac 1 -ar 16000 "$output" >/dev/null 2>&1
  echo "  wrote $output"
done
shopt -u nullglob

echo
echo "Done. Development/debug samples are available at:"
echo "  $SAMPLES_DIR"
echo
echo "Note: samples are optional and not required for release builds/runtime."
