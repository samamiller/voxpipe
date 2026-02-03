#!/usr/bin/env bash
set -euo pipefail

scripts=(
  scripts/voxpipe.sh
  scripts/model-download.sh
  scripts/render-packaging.sh
  scripts/update-whispercpp.sh
)

for s in "${scripts[@]}"; do
  bash -n "$s"
done

if command -v shellcheck >/dev/null; then
  shellcheck -x "${scripts[@]}"
else
  echo "note: shellcheck not installed; skipping" >&2
fi
