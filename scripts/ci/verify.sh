#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-full}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

log() {
  printf '\n==> %s\n' "$1"
}

warn() {
  printf '\n[warn] %s\n' "$1"
}

run() {
  printf '+ %s\n' "$*"
  "$@"
}

run_cargo_matrix() {
  log "Rust format and tests"
  run cargo fmt --all --check
  run cargo check --workspace
  run cargo test --workspace

  log "Rust feature matrix"
  run cargo check --workspace --all-features
  run cargo test --workspace --all-features
}

run_docs_checks() {
  log "Documentation validation"
  run python3 scripts/docs/validate_docs.py all
  run python3 scripts/docs/validate_docs.py audit-report --output .artifacts/docs-audit.json
}

run_optional_clippy() {
  if cargo clippy -V >/dev/null 2>&1; then
    log "Clippy (all targets/features)"
    run cargo clippy --workspace --all-targets --all-features -- -D warnings
  else
    warn "cargo clippy not available; skipping clippy stage"
  fi
}

run_prototype_compile_checks() {
  log "Prototype compile checks"
  run cargo check -p site --features csr

  if rustup target list --installed | grep -q '^wasm32-unknown-unknown$'; then
    run cargo check -p site --target wasm32-unknown-unknown --features csr
  else
    warn "wasm32-unknown-unknown target not installed; skipping wasm cargo check"
  fi

  if command -v trunk >/dev/null 2>&1; then
    log "Prototype static build (Trunk)"
    run trunk build crates/site/index.html --release --dist target/trunk-dist
  else
    warn "trunk not installed; skipping trunk build"
  fi
}

case "$MODE" in
  fast)
    run_cargo_matrix
    run_docs_checks
    ;;
  full)
    run_cargo_matrix
    run_docs_checks
    run_prototype_compile_checks
    run_optional_clippy
    ;;
  *)
    echo "Usage: $0 [fast|full]" >&2
    exit 2
    ;;
esac

log "Verification complete"
