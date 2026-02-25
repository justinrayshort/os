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

Build a production-like static bundle:

```bash
cargo web-build
```

Run prototype-specific compile checks (hydrate/ssr/wasm):

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
