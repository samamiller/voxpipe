#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
VERSION_FILE="${ROOT_DIR}/packaging/whisper-cpp/VERSION"
SHA_FILE="${ROOT_DIR}/packaging/whisper-cpp/SHA256"

LATEST_TAG=$(curl -fsSL https://api.github.com/repos/ggml-org/whisper.cpp/releases/latest | \
  python3 - <<'PY'
import json,sys
print(json.load(sys.stdin)["tag_name"])
PY
)

LATEST_VERSION=${LATEST_TAG#v}
CURRENT_VERSION=$(cat "${VERSION_FILE}")

if [[ "${LATEST_VERSION}" == "${CURRENT_VERSION}" ]]; then
  echo "whisper.cpp is up to date: ${CURRENT_VERSION}"
  exit 0
fi

echo "Updating whisper.cpp ${CURRENT_VERSION} -> ${LATEST_VERSION}"

echo "${LATEST_VERSION}" > "${VERSION_FILE}"

"${ROOT_DIR}/scripts/render-packaging.sh" "${LATEST_VERSION}"

TARBALL_URL="https://github.com/ggml-org/whisper.cpp/archive/refs/tags/v${LATEST_VERSION}.tar.gz"
TARBALL_SHA=$(curl -fsSL "${TARBALL_URL}" | sha256sum | awk '{print $1}')

echo "${TARBALL_SHA}  v${LATEST_VERSION}.tar.gz" > "${SHA_FILE}"
