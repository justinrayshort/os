#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${1:-}"
OUT_DIR="${2:-.artifacts/ui-conformance/keyboard}"

echo "warning: scripts/ui/keyboard-flow-smoke.sh is deprecated; use cargo e2e run --profile local-dev --scenario ui.shell.interaction-state or ui.shell.navigation-state" >&2

if [[ -n "$BASE_URL" ]]; then
  cargo e2e run --profile local-dev --scenario ui.shell.interaction-state --base-url "$BASE_URL" --artifact-dir "$OUT_DIR"
else
  cargo e2e run --profile local-dev --scenario ui.shell.interaction-state --artifact-dir "$OUT_DIR"
fi
