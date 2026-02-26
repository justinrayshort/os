# Retro Desktop Prototype (Leptos + Rust/WASM)

This repository contains a debranded retro desktop website prototype with a desktop shell runtime, window manager reducer, and mounted mini-apps (`Explorer`, `Notepad`, `Terminal`).

Documentation is split by intent:

- `rustdoc` (generated from Rust source comments) is the authoritative API/reference surface.
- GitHub Wiki (`wiki/` submodule) is the canonical documentation hub and navigation surface, organized with Diataxis (tutorials, how-to guides, explanations, and project reference indexes).
- `docs/` (repo-native Markdown) remains the canonical storage for documentation governance, contracts, ADRs, SOPs, diagrams/assets, and tooling reference, validated by `cargo xtask docs`.

## Prototype Status

- Desktop shell with taskbar, launcher menu, windows, and persistence hooks
- Offline-first browser storage architecture:
  - IndexedDB for versioned namespaced app/system state and virtual filesystem
  - Cache API for cached file previews/responses
  - localStorage for lightweight preferences/config
  - in-memory session store for ephemeral UI state
- Deep-link bootstrap (`/?open=...`, hash variants)
- Mounted app crates:
  - `Explorer` (File System Access API + IndexedDB virtual FS fallback)
  - `Notepad` (editable persisted workspace)
  - `Calculator` (persisted memory/tape/history state)
  - `Terminal` (persisted transcript/input workspace)
- Placeholder app panels:
  - `Paint` (persisted placeholder settings/state schema scaffold)
  - `Dial-up`
- Docs-as-code system with Diataxis structure, governance contracts, and Rust-native local validation/audit workflows

## Run the Prototype (Browser / WASM)

Prerequisites:

- Rust toolchain
- `wasm32-unknown-unknown` target
- [`trunk`](https://trunkrs.dev/)

Install prerequisites (one-time):

```bash
cargo setup-web
```

Initialize the GitHub Wiki submodule (required for wiki/docs updates):

```bash
git submodule update --init --recursive
```

Start local prototype server:

```bash
cargo dev
```

Start/stop a managed background dev server (Rust-managed lifecycle; logs/state under `.artifacts/dev-server/`):

```bash
cargo dev start
cargo dev status
cargo dev stop
```

Restart the managed background server:

```bash
cargo dev restart
```

Build a development static bundle (non-release) with the same trunk pipeline:

```bash
cargo dev build
```

Build a production-like static bundle:

```bash
cargo web-build
```

Run prototype-specific compile checks (CSR native + WASM):

```bash
cargo web-check
```

## Standardized Verification

Fast verification (Rust + docs):

```bash
cargo verify-fast
```

Full verification (feature matrix, docs audit, prototype checks, optional clippy/trunk build):

```bash
cargo verify
```

Equivalent legacy `make` targets still work (they now delegate to Cargo aliases):

```bash
make verify-fast
make verify
make proto-serve
make proto-start
make proto-stop
```

Direct commands remain available if you prefer (`cargo run -p xtask -- ...`, `trunk ...`).

## Performance Engineering Workflow (Benchmarks + Profiling)

The repository now exposes a standardized performance workflow through `xtask` so benchmark runs, baselines, and profiling artifacts are repeatable across local environments.

Tooling availability check:

```bash
cargo perf doctor
```

Preflight functional correctness before optimization (unit/integration tests, all-features tests, doctests, benchmark target compile):

```bash
cargo perf check
```

Run workspace benchmarks (including Criterion benches where present):

```bash
cargo perf bench
```

Capture and compare Criterion baselines:

```bash
cargo perf baseline local-main
cargo perf compare local-main
```

Profile CPU and memory (optional tools; host/OS dependent):

```bash
cargo perf flamegraph --bench <bench_name>
cargo perf heaptrack -- cargo bench --workspace
```

Performance artifacts default to `.artifacts/perf/`. See the performance SOP/reference pages (indexed from the wiki) for baseline thresholds, workload guidance, and documentation expectations for optimization decisions.

## Documentation Workflow (Rustdoc + Wiki + Repo Docs)

Use the GitHub Wiki (`wiki/Home.md`) as the primary documentation entry point for project navigation and artifact discovery. Update the relevant wiki reference/index pages when adding or changing formal artifacts (for example ADRs, SOPs, diagrams, or command references).

Generate Rust API reference locally:

```bash
cargo doc --workspace --no-deps
```

Run rustdoc examples (doctests):

```bash
cargo test --workspace --doc
```

Run repo docs validation (docs contracts + wiki submodule checks):

```bash
cargo xtask docs all
```

## Project Layout (Current)

- `crates/site` - Leptos app shell, routes, runtime mounting, theme CSS
- `crates/desktop_runtime` - desktop state, reducer, effects, shell components, registry
- `crates/platform_host` - API-first host contracts/types crate (Phase 1 migration foundation)
- `crates/platform_host_web` - browser (`wasm32`) implementations of `platform_host` services (app-state/cache/prefs/explorer)
- `crates/platform_storage` - temporary compatibility facade exposing legacy wrappers during host-boundary migration
- `crates/apps/explorer` - Explorer app UI crate
- `crates/apps/notepad` - Notepad app UI crate
- `crates/apps/calculator` - Calculator app UI crate
- `crates/apps/terminal` - Terminal app UI crate
- `wiki/` - GitHub Wiki submodule (canonical documentation hub: Diataxis pages + reference indexes)
- `docs/` - repo-native formal documentation artifacts (ADR/SOP/contracts/reference/assets) backing the wiki hub
- `xtask/src/docs.rs` - docs validation/audit implementation used by `cargo xtask docs`
- `xtask/src/main.rs` - standardized project verification and developer workflow orchestration (`cargo xtask ...`)
