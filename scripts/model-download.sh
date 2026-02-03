#!/usr/bin/env bash
set -euo pipefail

MODEL_NAME="${1:-${MODEL_NAME:-base.en-q5_1}}"
WHISPER_REPO="${WHISPER_REPO:-$HOME/whisper.cpp}"
DL_SCRIPT="${WHISPER_REPO}/models/download-ggml-model.sh"

if [[ ! -x "${DL_SCRIPT}" ]]; then
  echo "missing download script: ${DL_SCRIPT}" >&2
  echo "hint: clone whisper.cpp and run: ./models/download-ggml-model.sh ${MODEL_NAME}" >&2
  exit 1
fi

"${DL_SCRIPT}" "${MODEL_NAME}"
