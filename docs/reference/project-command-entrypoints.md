---
title: "Project Command Entry Points"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-28"
audience: ["engineering", "platform"]
invariants:
  - "Cargo aliases in .cargo/config.toml remain the preferred stable entry points for common project workflows."
  - "Root Makefile targets remain a minimal compatibility shell over Cargo aliases and must not diverge silently."
tags: ["reference", "commands", "tooling", "developer-workflow"]
domain: "docs"
lifecycle: "ga"
---

# Project Command Entry Points

This page documents the supported top-level commands for local development, verification, performance engineering workflows, and documentation checks.

## Source of Truth

- Cargo aliases: [`.cargo/config.toml`](../../.cargo/config.toml)
- CLI surface: [`xtask/src/bin/xtask.rs`](../../xtask/src/bin/xtask.rs)
- Shared automation runtime: [`xtask/src/runtime/`](../../xtask/src/runtime/)
- Command domains: [`xtask/src/commands/`](../../xtask/src/commands/)
- Typed automation config: [`tools/automation/`](../../tools/automation/)
- Compatibility wrappers: [`Makefile`](../../Makefile)
- Internal architecture reference: [xtask Automation Runtime Architecture](xtask-automation-runtime-architecture.md)

## Preferred Commands (Cargo Aliases)

### Prototype / Web Workflow

- `cargo setup-web`: Install the WASM target and `trunk` if missing.
- `cargo dev`: Prototype dev workflow entry point (delegates to `xtask dev`).
- `cargo dev` / `cargo dev serve`: Start the prototype dev server in the foreground (defaults to opening the browser).
- `cargo dev start`: Start the prototype dev server in the background (managed PID/state/logs under `.artifacts/dev-server/`).
- `cargo dev stop`: Stop the managed background dev server.
- `cargo dev status`: Show managed background dev server status.
- `cargo dev logs [--lines <N>]`: Show recent managed dev server logs without ad-hoc `tail` commands.
- `cargo dev restart`: Restart the managed background dev server.
- `cargo dev build`: Build a development static bundle via `trunk` (non-release).
- `cargo dev` serve/build defaults include `--no-sri=true`; file hashing remains enabled unless explicitly overridden so browser asset URLs change across rebuilds.
- `cargo dev serve` also auto-adds `--ignore <active-dist-path>` when not explicitly provided so Trunk does not watch-and-rebuild on its own output directory.
- Dev server defaults are loaded from `tools/automation/dev_server.toml`, which version-controls the canonical managed server host, port, timeouts, and artifact paths.
- `cargo web-check`: Run prototype compile checks (CSR native and WASM when target is installed).
- `cargo web-build`: Build the production-style static bundle via `trunk`.

### Desktop (Tauri) Workflow

- `cargo xtask tauri check`: Compile-check the Tauri desktop crate wiring.
- `cargo tauri-dev`: Run `cargo tauri dev` from `crates/desktop_tauri/` using `tauri.conf.json` build hooks, which delegate frontend serve/build work to `cargo dev` (`xtask`) for consistent path/env handling.
- `cargo tauri-build`: Run `cargo tauri build` from `crates/desktop_tauri/` using `tauri.conf.json` build hooks, which delegate frontend serve/build work to `cargo dev` (`xtask`) for consistent path/env handling.

### Verification Workflow

- `cargo flow`: Run scoped inner-loop validation (`xtask flow`) using changed files (`git status --porcelain`) to target affected workspace crates.
- `cargo doctor` / `cargo doctor --fix`: Validate local automation prerequisites (tooling, wiki wiring, managed state) and optionally apply safe fixes.
- `cargo verify-fast`: Run fast workspace verification (`xtask verify fast`) with conditional desktop host checks.
  - `cargo verify-fast --with-desktop`: Force include desktop host checks.
  - `cargo verify-fast --without-desktop`: Force skip desktop host checks.
- `cargo verify`: Run full project verification (`xtask verify`, default mode `full`) including prototype compile checks, optional clippy, and always-on desktop host coverage.
- `cargo verify --profile <name>`: Run profile-driven verification from `tools/automation/verify_profiles.toml` (`dev`, `ci-fast`, `ci-full`, `release`).
- `cargo verify-fast` / `cargo verify`: print per-stage duration for measurable feedback-loop latency.
- `cargo flow`, `cargo verify*`, and `cargo doctor` emit structured run artifacts under `.artifacts/automation/runs/<run-id>/` (`manifest.json`, `events.jsonl`).
- `cargo doctor`, `cargo flow`, `cargo verify*`, `cargo perf *`, and `cargo xtask docs *` now execute through a shared library-backed xtask runtime (`CommandContext`, `ProcessRunner`, `WorkspaceState`, lifecycle helpers, `WorkflowRecorder`, `ArtifactManager`) instead of a monolithic binary entry file.
- `cargo check-all`: Explicit full-workspace compile check alias (`cargo check --workspace`).
- `cargo test-all`: Explicit full-workspace test alias (`cargo test --workspace`).

### Performance Engineering Workflow

- `cargo perf doctor`: Check availability of local benchmark/profiling tools (`cargo`, `cargo flamegraph`, `perf`, `heaptrack`, `sccache`) and report `RUSTC_WRAPPER`/sccache activation status.
- `cargo perf check`: Run functional test preflight (workspace tests, all-features tests, doctests) and compile benchmark targets (`cargo bench --no-run`).
- `cargo perf bench [args...]`: Run workspace benchmark suites (`cargo xtask perf bench` passthrough).
- `cargo perf baseline <name> [cargo-bench-args...]`: Run Criterion benchmarks and save a baseline for regression comparison.
- `cargo perf compare <name> [cargo-bench-args...]`: Run Criterion benchmarks and compare against a named baseline.
- `cargo perf dev-loop-baseline [--output <path>]`: Run a repeatable local developer-loop timing bundle (`clean`, `check --workspace`, `test --no-run`, `verify-fast`, `xtask docs all`) and emit a JSON report (default: `.artifacts/perf/reports/dev-loop-baseline.json`).
- `cargo perf flamegraph [args...]`: Run CPU profiling via `cargo flamegraph` with a default SVG output path under `.artifacts/perf/flamegraphs/` when none is provided.
- `cargo perf heaptrack [-- <cmd...>]`: Run heap profiling with `heaptrack` (default command: `cargo bench --workspace`).
- `source scripts/dev/setup-sccache.sh`: Optional local helper to configure `RUSTC_WRAPPER=sccache`, `SCCACHE_DIR=.artifacts/sccache`, and `SCCACHE_CACHE_SIZE=20G` in the current shell.

### Documentation Workflow (Rustdoc + Wiki)

- `cargo wiki status`: Show whether the `wiki/` submodule is initialized, which branch/HEAD it is on, and whether it has local changes.
- `cargo wiki sync`: Refresh submodule wiring and initialize/update the `wiki/` submodule.
  - Refuses to run when `wiki/` has local modifications, so scripted refreshes do not trample in-progress wiki edits.
- `cargo docs-check`: Run `cargo xtask docs all` (Cargo alias convenience wrapper).
- `cargo docs-audit`: Generate `.artifacts/docs-audit.json` via `cargo xtask docs audit-report`.
- `cargo xtask docs wiki`: Validate wiki submodule wiring and required navigation/category pages (useful for staged/isolated wiki diagnostics).
- `cargo xtask docs ui-inventory --output .artifacts/ui/styling-inventory.json`: Generate a machine-readable inventory of Rust/CSS styling entry points, local visual contracts, token definitions, and hard-coded literals across shell/apps/system_ui/theme files.
- `cargo xtask docs storage-boundary`: Enforce typed app-state persistence boundaries by flagging legacy low-level envelope access patterns in `crates/apps`, `crates/desktop_runtime`, and `crates/site`.
- `cargo xtask docs app-contract`: Validate app manifest contract shape, app-id conventions, and forbidden ad hoc app integration patterns.
- `scripts/ui/capture-skin-matrix.sh [base_url] [output_dir]`: Capture screenshot evidence matrix across skins (`soft-neumorphic`, `modern-adaptive`, `classic-xp`, `classic-95`) and breakpoints (`desktop`, `tablet`, `mobile`) into `.artifacts/ui-conformance/screenshots/`.
- `scripts/ui/keyboard-flow-smoke.sh [base_url] [output_dir]`: Run keyboard traversal smoke checks for context menu and system-surface settings flows across all skins, writing `.artifacts/ui-conformance/keyboard/keyboard-smoke-report.json`.
- `cargo doc --workspace --no-deps`: Generate authoritative Rust API reference (`target/doc/`).
- `cargo test --workspace --doc`: Run rustdoc examples (doctests).
- `cargo xtask docs all`: Run docs contract validation (includes `wiki` validation).

## Root `make` Compatibility Targets

These targets exist only as a small compatibility shell for operator convenience and local muscle memory. Prefer the Cargo aliases above for everyday use.

- `make verify-fast` -> `cargo verify-fast`
- `make verify` -> `cargo verify`
- `make wiki-init` -> `cargo wiki sync`
- `make docs-check` -> `cargo docs-check`
- `make rustdoc-check` -> `cargo doc --workspace --no-deps && cargo test --workspace --doc`
- `make proto-serve` -> `cargo dev serve`
- `make proto-stop` -> `cargo dev stop`
- `make proto-status` -> `cargo dev status`

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
- Use `make` targets only when you need compatibility with existing local shell habits.
- When adding or changing a top-level command, update this page, `README.md`, and `AGENTS.md` in the same change.
- When extending `xtask`, add workflow logic under `xtask/src/commands/` and shared orchestration under `xtask/src/runtime/`; avoid adding new workflow logic directly to the binary entrypoint.
- When public APIs change, update rustdoc comments and run doctests in the same change.
- When tutorials/how-to/explanations change, update the `wiki/` submodule in the same review cycle.
