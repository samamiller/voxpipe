#!/usr/bin/env bash
set -euo pipefail

MODEL_NAME="${1:-tiny.en}"
CACHE_DIR="${ASR_HUD_MODELS_DIR:-$HOME/.cache/asr-hud/models}"

mkdir -p "$CACHE_DIR"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd -P)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." >/dev/null 2>&1 && pwd -P)"

"$REPO_ROOT/vendor/models/download-ggml-model.sh" "$MODEL_NAME" "$CACHE_DIR"

MODEL_PATH="$CACHE_DIR/ggml-$MODEL_NAME.bin"
echo "Model ready: $MODEL_PATH"
echo "Use it with:"
echo "  export ASR_MODEL=\"$MODEL_PATH\""
