#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCCACHE_BIN="${SCCACHE_BIN:-$(command -v sccache || true)}"
SCCACHE_DIR_DEFAULT="${REPO_ROOT}/.artifacts/sccache"
SCCACHE_SIZE_DEFAULT="${SCCACHE_CACHE_SIZE:-20G}"

if [[ -z "${SCCACHE_BIN}" ]]; then
  echo "sccache was not found in PATH."
  echo "Install with: cargo install sccache"
  exit 1
fi

mkdir -p "${SCCACHE_DIR_DEFAULT}"

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  cat <<EOF
Run the following in your shell to enable local compiler caching:

  export RUSTC_WRAPPER="${SCCACHE_BIN}"
  export SCCACHE_DIR="${SCCACHE_DIR_DEFAULT}"
  export SCCACHE_CACHE_SIZE="${SCCACHE_SIZE_DEFAULT}"

Then verify cache stats with:

  sccache --show-stats
EOF
  exit 0
fi

export RUSTC_WRAPPER="${SCCACHE_BIN}"
export SCCACHE_DIR="${SCCACHE_DIR_DEFAULT}"
export SCCACHE_CACHE_SIZE="${SCCACHE_SIZE_DEFAULT}"

echo "sccache enabled for this shell session:"
echo "  RUSTC_WRAPPER=${RUSTC_WRAPPER}"
echo "  SCCACHE_DIR=${SCCACHE_DIR}"
echo "  SCCACHE_CACHE_SIZE=${SCCACHE_CACHE_SIZE}"
