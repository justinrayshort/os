# Retro Desktop Prototype (Leptos + Rust/WASM)

This repository contains a debranded retro desktop website prototype with a desktop shell runtime, window manager reducer, and mounted mini-apps (`Explorer`, `Notepad`, `Terminal`).

## Prototype Status

- Desktop shell with taskbar, launcher menu, windows, and persistence hooks
- Deep-link bootstrap (`/?open=...`, hash variants)
- Mounted app crates:
  - `Explorer`
  - `Notepad`
  - `Terminal`
- Placeholder app panels:
  - `Paint`
  - `Dial-up`
- Docs-as-code system with Diataxis structure, governance contracts, validation, and CI workflows

## Run the Prototype (Browser / WASM)

Prerequisites:

- Rust toolchain
- `wasm32-unknown-unknown` target
- [`trunk`](https://trunkrs.dev/)

Install prerequisites (one-time):

```bash
cargo setup-web
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

Direct commands remain available if you prefer (`cargo run -p xtask -- ...`, `trunk ...`, `./scripts/ci/verify.sh ...`).

## Project Layout (Current)

- `crates/site` - Leptos app shell, routes, runtime mounting, theme CSS
- `crates/desktop_runtime` - desktop state, reducer, effects, shell components, registry
- `crates/apps/explorer` - Explorer app UI crate
- `crates/apps/notepad` - Notepad app UI crate
- `crates/apps/terminal` - Terminal app UI crate
- `docs/` - Diataxis documentation and SOP/governance system
- `scripts/docs/validate_docs.py` - docs validation/audit CLI
- `scripts/ci/verify.sh` - standardized project verification script
