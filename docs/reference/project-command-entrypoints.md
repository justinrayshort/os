---
title: "Project Command Entry Points"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-27"
audience: ["engineering", "platform"]
invariants:
  - "Cargo aliases in .cargo/config.toml remain the preferred stable entry points for common project workflows."
  - "Root Makefile targets delegate to Cargo aliases or `xtask` docs commands and must not diverge silently."
tags: ["reference", "commands", "tooling", "developer-workflow"]
domain: "docs"
lifecycle: "ga"
---

# Project Command Entry Points

This page documents the supported top-level commands for local development, verification, performance engineering workflows, and documentation checks.

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

### Desktop (Tauri) Workflow

- `cargo xtask tauri check`: Compile-check the Tauri desktop crate wiring.
- `cargo tauri-dev`: Run `cargo tauri dev` from `crates/desktop_tauri/` using `tauri.conf.json` build hooks, which delegate frontend serve/build work to `cargo dev` (`xtask`) for consistent path/env handling.
- `cargo tauri-build`: Run `cargo tauri build` from `crates/desktop_tauri/` using `tauri.conf.json` build hooks, which delegate frontend serve/build work to `cargo dev` (`xtask`) for consistent path/env handling.

### Verification Workflow

- `cargo flow`: Run scoped inner-loop validation (`xtask flow`) using changed files (`git status --porcelain`) to target affected workspace crates.
- `cargo verify-fast`: Run workspace verification (`xtask verify fast`) with explicit handling for mutually-exclusive `platform_storage` host feature sets.
- `cargo verify`: Run full project verification (`xtask verify full`) including prototype compile checks and optional clippy.
- `cargo verify-fast` / `cargo verify`: print per-stage duration for measurable feedback-loop latency.

### Performance Engineering Workflow

- `cargo perf doctor`: Check availability of local benchmark/profiling tools (`cargo`, `cargo flamegraph`, `perf`, `heaptrack`).
- `cargo perf check`: Run functional test preflight (workspace tests, all-features tests, doctests) and compile benchmark targets (`cargo bench --no-run`).
- `cargo perf bench [args...]`: Run workspace benchmark suites (`cargo xtask perf bench` passthrough).
- `cargo perf baseline <name> [cargo-bench-args...]`: Run Criterion benchmarks and save a baseline for regression comparison.
- `cargo perf compare <name> [cargo-bench-args...]`: Run Criterion benchmarks and compare against a named baseline.
- `cargo perf flamegraph [args...]`: Run CPU profiling via `cargo flamegraph` with a default SVG output path under `.artifacts/perf/flamegraphs/` when none is provided.
- `cargo perf heaptrack [-- <cmd...>]`: Run heap profiling with `heaptrack` (default command: `cargo bench --workspace`).

### Documentation Workflow (Rustdoc + Wiki)

- `git submodule sync --recursive && git submodule update --init --recursive`: Refresh submodule wiring and initialize/update the `wiki/` submodule.
- `cargo docs-check`: Run `cargo xtask docs all` (Cargo alias convenience wrapper).
- `cargo docs-audit`: Generate `.artifacts/docs-audit.json` via `cargo xtask docs audit-report`.
- `cargo xtask docs wiki`: Validate wiki submodule wiring and required navigation/category pages.
- `cargo xtask docs storage-boundary`: Enforce typed app-state persistence boundaries (disallow direct `platform_storage::load_app_state_envelope(...)` usage in `crates/apps`, `crates/desktop_runtime`, and `crates/site`).
- `scripts/ui/capture-skin-matrix.sh [base_url] [output_dir]`: Capture screenshot evidence matrix across skins (`modern-adaptive`, `classic-xp`, `classic-95`) and breakpoints (`desktop`, `tablet`, `mobile`) into `.artifacts/ui-conformance/screenshots/`.
- `scripts/ui/keyboard-flow-smoke.sh [base_url] [output_dir]`: Run keyboard traversal smoke checks for context menu and Display Properties listbox/tab flows across all skins, writing `.artifacts/ui-conformance/keyboard/keyboard-smoke-report.json`.
- `cargo doc --workspace --no-deps`: Generate authoritative Rust API reference (`target/doc/`).
- `cargo test --workspace --doc`: Run rustdoc examples (doctests).
- `cargo xtask docs all`: Run docs contract validation (also includes `wiki` validation).

## Root `make` Compatibility Targets

These targets exist for operator convenience and local muscle memory. They delegate to Cargo aliases or `xtask` docs commands.

- `make verify-fast` -> `cargo verify-fast`
- `make verify` -> `cargo verify`
- `make flow` -> `cargo flow`
- `make wiki-init` -> `git submodule sync --recursive && git submodule update --init --recursive`
- `make rustdoc-check` -> `cargo doc --workspace --no-deps && cargo test --workspace --doc`
- `make docs-check` -> `cargo xtask docs all` + `make rustdoc-check`
- `make docs-audit` -> `cargo xtask docs audit-report --output .artifacts/docs-audit.json`
- `make perf-doctor` -> `cargo perf doctor`
- `make perf-check` -> `cargo perf check`
- `make perf-bench` -> `cargo perf bench`
- `make perf-baseline BASELINE=<name>` -> `cargo perf baseline <name>`
- `make perf-compare BASELINE=<name>` -> `cargo perf compare <name>`
- `make perf-flamegraph ARGS='--bench <bench_name>'` -> `cargo perf flamegraph <args>`
- `make perf-heaptrack [ARGS='-- cargo bench --workspace']` -> `cargo perf heaptrack <args>`
- `make proto-check` -> `cargo web-check`
- `make proto-build` -> `cargo web-build`
- `make proto-build-dev` -> `cargo dev build`
- `make proto-serve` -> `cargo dev serve`
- `make proto-start` -> `cargo dev start`
- `make proto-stop` -> `cargo dev stop`
- `make proto-status` -> `cargo dev status`
- `make proto-restart` -> `cargo dev restart`
- `make tauri-check` -> `cargo xtask tauri check`
- `make tauri-dev` -> `cargo tauri-dev`
- `make tauri-build` -> `cargo tauri-build`

## Local Browser Automation (Playwright CLI Wrapper)

Use the Codex Playwright CLI wrapper for quick interactive browser automation and UI smoke checks from the terminal.

### Entry Points

- `pwc`: local shell wrapper function that delegates to `"$PWCLI"` (the Codex skill wrapper script).
- `pwo <url>`: local shell helper that runs `pwc open --browser chrome --config /Users/justinshort/os/.playwright/cli.config.json <url>`.
- `"$PWCLI" ...`: direct wrapper usage (useful in non-interactive shells or scripts).

### Local Project Config (Machine-Local)

The local Playwright CLI config file is `.playwright/cli.config.json` (ignored from version control on this repo). Current local defaults:

- headed browser launch (`headless: false`)
- fixed viewport (`1440x900`) for repeatable screenshots/layout checks
- Chrome executable path pointing to the user-local install under `~/Applications/Google Chrome.app`

If your Chrome install path differs, update `browser.launchOptions.executablePath` in the local config.

### Quick Smoke Test Loop

From the repo root:

```bash
export PLAYWRIGHT_CLI_SESSION=t1   # keep session names short
pwc open https://example.com       # opens Chrome, headed, using .playwright/cli.config.json
pwc snapshot                       # capture refs
pwc eval "window.innerWidth + \"x\" + window.innerHeight"
pwc close
```

Expected viewport result with the current local config: `"1440x900"`.

### Notes

- `pwc open` reads `.playwright/cli.config.json` automatically when run from the repo root.
- `pwo` pins `--config` explicitly, so it works even when your current directory is not the repo root.
- Prefer short `PLAYWRIGHT_CLI_SESSION` values (for example `t1`, `ui1`) to avoid macOS socket path length issues in `playwright-cli`.

## Guidance

- Prefer Cargo aliases in automation and documentation because they are defined in-repo and map directly to `xtask`.
- Use `make` targets when you want shorter commands or to align with existing shell habits.
- When adding or changing a top-level command, update this page, `README.md`, and `AGENTS.md` in the same change.
- When public APIs change, update rustdoc comments and run doctests in the same change.
- When tutorials/how-to/explanations change, update the `wiki/` submodule in the same review cycle.
