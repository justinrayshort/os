---
title: "Project Command Entry Points"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-25"
audience: ["engineering", "platform"]
invariants:
  - "Cargo aliases in .cargo/config.toml remain the preferred stable entry points for common project workflows."
  - "Root Makefile targets delegate to Cargo aliases or docs validation scripts and must not diverge silently."
tags: ["reference", "commands", "tooling", "developer-workflow"]
domain: "docs"
lifecycle: "ga"
---

# Project Command Entry Points

This page documents the supported top-level commands for local development, verification, and documentation checks.

## Source of Truth

- Cargo aliases: [`.cargo/config.toml`](../../.cargo/config.toml)
- Task implementation: [`xtask/src/main.rs`](../../xtask/src/main.rs)
- Compatibility wrappers: [`Makefile`](../../Makefile)

## Preferred Commands (Cargo Aliases)

### Prototype / Web Workflow

- `cargo setup-web`: Install the WASM target and `trunk` if missing.
- `cargo dev`: Prototype dev workflow entry point (delegates to `xtask dev`).
- `cargo dev` / `cargo dev serve`: Start the prototype dev server in the foreground (defaults to opening the browser).
- `cargo dev start`: Start the prototype dev server in the background (managed PID/state/logs under `.artifacts/dev-server/`).
- `cargo dev stop`: Stop the managed background dev server.
- `cargo dev status`: Show managed background dev server status.
- `cargo dev restart`: Restart the managed background dev server.
- `cargo dev build`: Build a development static bundle via `trunk` (non-release).
- `cargo web-check`: Run prototype compile checks (CSR native and WASM when target is installed).
- `cargo web-build`: Build the production-style static bundle via `trunk`.

### Verification Workflow

- `cargo verify-fast`: Run fast project verification (`xtask verify fast`).
- `cargo verify`: Run full project verification (`xtask verify full`).

## Root `make` Compatibility Targets

These targets exist for operator convenience and CI/local muscle memory. They delegate to Cargo aliases or docs validation commands.

- `make verify-fast` -> `cargo verify-fast`
- `make verify` -> `cargo verify`
- `make docs-check` -> `python3 scripts/docs/validate_docs.py all`
- `make docs-audit` -> `python3 scripts/docs/validate_docs.py audit-report --output .artifacts/docs-audit.json`
- `make proto-check` -> `cargo web-check`
- `make proto-build` -> `cargo web-build`
- `make proto-build-dev` -> `cargo dev build`
- `make proto-serve` -> `cargo dev serve`
- `make proto-start` -> `cargo dev start`
- `make proto-stop` -> `cargo dev stop`
- `make proto-status` -> `cargo dev status`
- `make proto-restart` -> `cargo dev restart`

## Guidance

- Prefer Cargo aliases in automation and documentation because they are defined in-repo and map directly to `xtask`.
- Use `make` targets when you want shorter commands or to align with existing shell habits.
- When adding or changing a top-level command, update this page, `README.md`, and `AGENTS.md` in the same change.
