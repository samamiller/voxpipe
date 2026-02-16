#!/usr/bin/env bash
set -euo pipefail

MODEL_NAME="${1:-base.en-q5_1}"
CACHE_DIR="${VOXPIPE_MODELS_CACHE_DIR:-$HOME/.cache/voxpipe/models}"

mkdir -p "$CACHE_DIR"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd -P)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." >/dev/null 2>&1 && pwd -P)"

"$REPO_ROOT/vendor/models/download-ggml-model.sh" "$MODEL_NAME" "$CACHE_DIR"

MODEL_PATH="$CACHE_DIR/ggml-$MODEL_NAME.bin"
echo "Model ready: $MODEL_PATH"
echo "Use it with:"
echo "  export ASR_MODEL=\"$MODEL_PATH\""
