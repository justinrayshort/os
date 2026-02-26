---
title: "Project Command Entry Points"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-26"
audience: ["engineering", "platform"]
invariants:
  - "Cargo aliases in .cargo/config.toml remain the preferred stable entry points for common project workflows."
  - "Root Makefile targets delegate to Cargo aliases or `xtask` docs commands and must not diverge silently."
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

### Documentation Workflow (Rustdoc + Wiki)

- `git submodule update --init --recursive`: Initialize/update the `wiki/` submodule.
- `cargo docs-check`: Run `cargo xtask docs all` (Cargo alias convenience wrapper).
- `cargo docs-audit`: Generate `.artifacts/docs-audit.json` via `cargo xtask docs audit-report`.
- `cargo xtask docs wiki`: Validate wiki submodule wiring and required navigation/category pages.
- `cargo doc --workspace --no-deps`: Generate authoritative Rust API reference (`target/doc/`).
- `cargo test --workspace --doc`: Run rustdoc examples (doctests).
- `cargo xtask docs all`: Run docs contract validation (also includes `wiki` validation).

## Root `make` Compatibility Targets

These targets exist for operator convenience and local muscle memory. They delegate to Cargo aliases or `xtask` docs commands.

- `make verify-fast` -> `cargo verify-fast`
- `make verify` -> `cargo verify`
- `make wiki-init` -> `git submodule update --init --recursive`
- `make rustdoc-check` -> `cargo doc --workspace --no-deps && cargo test --workspace --doc`
- `make docs-check` -> `cargo xtask docs all` + `make rustdoc-check`
- `make docs-audit` -> `cargo xtask docs audit-report --output .artifacts/docs-audit.json`
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
- When public APIs change, update rustdoc comments and run doctests in the same change.
- When tutorials/how-to/explanations change, update the `wiki/` submodule in the same review cycle.
