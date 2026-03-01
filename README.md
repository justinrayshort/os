# OS Desktop Runtime Workspace (Rust/WASM + Tauri)

This project is a Rust workspace implementing a desktop-style runtime and app shell that can run in browser (`wasm32`) or Tauri-hosted environments. It is organized into contract, runtime, host, storage-adapter, and app crates, with state managed through a reducer/effect model and persistence routed through typed host/storage abstractions.

It includes strong local tooling (`xtask`) for verification, documentation contract enforcement, and operational workflows, with moderate unit/integration test coverage concentrated in runtime and adapter logic.

Documentation is split by intent:

- `rustdoc` (generated from Rust source comments) is the authoritative API/reference surface.
- GitHub Wiki (`wiki/` submodule) is the canonical documentation hub and navigation surface, organized with Diataxis (tutorials, how-to guides, explanations, and project reference indexes).
- `docs/` (repo-native Markdown) remains the canonical storage for documentation governance, contracts, ADRs, SOPs, diagrams/assets, and tooling reference, validated by `cargo xtask docs`.

## Prototype Status

- Desktop shell with taskbar, launcher menu, windows, and persistence hooks
- Runtime behavior centered on reducer state transitions and `RuntimeEffect` execution
- Cross-environment shell execution path (browser/WASM and Tauri-hosted desktop workflows)
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
- trunk

Install prerequisites (one-time):

```bash
cargo setup-web
```

Initialize the GitHub Wiki submodule (required for wiki/docs updates):

```bash
cargo wiki sync
```

Start local prototype server:

```bash
cargo dev
```

Start/stop a managed background dev server (Rust-managed lifecycle; logs/state under `.artifacts/dev-server/`):

```bash
cargo dev start
cargo dev status
cargo dev logs --lines 80
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

Run the Tauri desktop shell workflow (Stage 2 scaffold):

```bash
cargo xtask tauri check
cargo tauri-dev
cargo tauri-build
```

`cargo tauri-dev` / `cargo tauri-build` use Tauri hooks that delegate frontend serve/build work to `cargo dev` (`xtask`) and normalize `NO_COLOR=1` to `NO_COLOR=true` for Trunk compatibility in environments that export numeric `NO_COLOR`.

`cargo dev` serve/build defaults disable Trunk SRI and file hashing (`--no-sri=true`, `--filehash=false`) unless explicitly overridden, which keeps the local prototype loop stable when assets are rebuilt frequently.

Run prototype-specific compile checks (CSR native + WASM):

```bash
cargo web-check
```

## Standardized Verification

Fast inner-loop validation scoped to changed packages/docs:

```bash
cargo flow
```

Fast verification (workspace Rust + docs):

```bash
cargo verify-fast
```

`cargo verify-fast` automatically skips desktop host (`desktop_tauri`) checks when changed files do not touch desktop/host boundary trigger paths. Override auto-detection when needed:

```bash
cargo verify-fast --with-desktop
cargo verify-fast --without-desktop
```

Profile-driven verification is also available:

```bash
cargo verify --profile dev
cargo verify --profile ci-fast
cargo verify --profile ci-full
cargo verify --profile release
```

Profile definitions live in `tools/automation/verify_profiles.toml`.

Run local automation/tooling diagnostics and safe auto-remediation:

```bash
cargo doctor
cargo doctor --fix
```

Resolve the Cargo-managed E2E profile surface and prerequisites:

```bash
cargo e2e list
cargo e2e doctor
cargo e2e run --profile local-dev --dry-run
cargo e2e run --profile local-dev --scenario shell.boot
```

The current E2E slice now includes the first executable browser path: profiles and scenario sets are
versioned under `tools/automation/`, `cargo e2e doctor` checks the local prerequisites, and
`cargo e2e run --profile ...` starts an isolated per-run `trunk serve` instance under
`.artifacts/e2e/`, bootstraps `tools/e2e/` with `npm ci` when needed, and runs the Playwright
harness with artifacts under `.artifacts/e2e/runs/`. Profile settings now materially affect the
run: `ci-headless` uses headless Chromium plus retry-on-failure policy, `cross-browser` fans out
across Chromium/Firefox/WebKit, and `debug` runs headed with slow motion and always-retained
traces. Desktop `tauri-webdriver` profiles are now versioned in the same config surface and
reported by `cargo e2e doctor`, but macOS remains the stable browser-first development path.
Desktop Linux/Windows execution is staged behind the same Cargo surface and tracked in
`docs/reference/cargo-e2e-desktop-platform-todo-spec.md`; on macOS, those profiles fail
immediately with an explicit unsupported-platform message.
`cargo e2e run --dry-run` remains available for config-only validation.

Full verification (fast verification + prototype checks + optional clippy/trunk build):

```bash
cargo verify
```

`cargo verify` remains exhaustive and always includes desktop host coverage. Profile-driven
verification via `cargo verify --profile <name>` can now optionally append a Cargo-managed E2E
stage from `tools/automation/verify_profiles.toml` after the normal Rust/docs/clippy stages.
`cargo verify-fast` and `cargo verify` print per-stage timing so bottlenecks are observable
directly from command output.

Compatibility `make` targets still work (delegating to Cargo aliases):

```bash
make flow
make doctor
make verify-fast
make verify
make proto-serve
make proto-start
make proto-stop
make proto-logs
```

Direct commands remain available if you prefer (`cargo run -p xtask -- ...`, `trunk ...`).
Workflow run artifacts are emitted to `.artifacts/automation/runs/<run-id>/` with `manifest.json` and `events.jsonl` for postmortem/debug tooling.

Useful ad-hoc full-workspace aliases:

```bash
cargo check-all
cargo test-all
```

## Performance Engineering Workflow (Benchmarks + Profiling)

The repository now exposes a standardized performance workflow through `xtask` so benchmark runs, baselines, and profiling artifacts are repeatable across local environments.

Tooling availability check:

```bash
cargo perf doctor
```

Capture a repeatable local development-loop baseline report:

```bash
cargo perf dev-loop-baseline --output .artifacts/perf/reports/dev-loop-baseline.json
```

Enable local compiler caching (optional, local-first):

```bash
source scripts/dev/setup-sccache.sh
sccache --show-stats
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

`cargo xtask docs all` already includes wiki validation. Use `cargo xtask docs wiki` separately when you want isolated wiki diagnostics.
Use `cargo wiki status` to inspect the submodule state and `cargo wiki sync` to initialize or refresh it through the same Cargo-managed workflow surface.

## Project Layout (Current)

- `crates/site` - Leptos app shell, routes, runtime mounting, theme CSS
- `crates/desktop_runtime` - desktop state, reducer, effects, shell components, registry
- `crates/desktop_app_contract` - typed app/runtime contracts for module mount context, host commands, and lifecycle/events
- `crates/desktop_tauri` - Tauri desktop shell host crate/configuration (`tauri.conf.json`, capabilities, CLI hooks)
- `crates/platform_host` - API-first host contracts/types crate (Phase 1 migration foundation)
- `crates/platform_host_web` - browser (`wasm32`) implementations of `platform_host` services (app-state/cache/prefs/explorer)
- `crates/apps/explorer` - Explorer app UI crate
- `crates/apps/notepad` - Notepad app UI crate
- `crates/apps/calculator` - Calculator app UI crate
- `crates/apps/terminal` - Terminal app UI crate
- `wiki/` - GitHub Wiki submodule (canonical documentation hub: Diataxis pages + reference indexes)
- `docs/` - repo-native formal documentation artifacts (ADR/SOP/contracts/reference/assets) backing the wiki hub
- `xtask/src/commands/docs/` - docs command-family façade used by `cargo xtask docs`
- `xtask/src/docs.rs` + `xtask/src/docs/` - docs validation/audit module root plus split validation surfaces
- `xtask/src/commands/` - standardized project verification and developer workflow command families (`cargo xtask ...`)
- `xtask/src/runtime/` - shared xtask automation runtime (process execution, workflow recording, config loading, artifacts)
