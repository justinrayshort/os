#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${1:-}"
OUT_DIR="${2:-.artifacts/ui-conformance/screenshots}"

echo "warning: scripts/ui/capture-skin-matrix.sh is deprecated; use cargo e2e run --profile local-dev --scenario ui.shell.layout-baseline" >&2

if [[ -n "$BASE_URL" ]]; then
  cargo e2e run --profile local-dev --scenario ui.shell.layout-baseline --base-url "$BASE_URL" --artifact-dir "$OUT_DIR"
else
  cargo e2e run --profile local-dev --scenario ui.shell.layout-baseline --artifact-dir "$OUT_DIR"
fi
