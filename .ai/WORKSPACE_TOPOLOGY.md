# Workspace Topology: Crate Inventory

**Version:** 1.0  
**Last Updated:** 2026-03-01  
**Purpose:** Machine-readable crate catalog for rapid navigation and dependency auditing.

## Crate Inventory

### Platform & Core Runtime (6 crates)

| Crate | Location | Purpose | Maintainer | Status |
|---|---|---|---|---|
| platform_host | crates/platform_host/ | Typed host contracts and models (fs, cache, notifications, etc.) | @system-owner | Active |
| platform_host_web | crates/platform_host_web/ | Browser/wasm implementations of platform_host | @web-team | Active |
| desktop_runtime | crates/desktop_runtime/ | Reducer-driven state machine, effects executor, app bus, shell UI | @runtime-team | Active |
| desktop_tauri | crates/desktop_tauri/ | Tauri native bootstrap, window lifecycle, IPC transport | @native-team | Active |
| desktop_app_contract | crates/desktop_app_contract/ | App registration and lifecycle contracts | @app-team | Active |
| system_ui | crates/system_ui/ | Shared visual primitives, icons, theme tokens | @design-system | Active |

### User-Facing Applications (6 crates)

| Crate | Location | Purpose | Status |
|---|---|---|---|
| desktop_app_calculator | crates/apps/calculator/ | Built-in calculator app | Active |
| desktop_app_explorer | crates/apps/explorer/ | File explorer app | Active |
| desktop_app_notepad | crates/apps/notepad/ | Text editor app | Active |
| desktop_app_settings | crates/apps/settings/ | System settings app | Active |
| desktop_app_terminal | crates/apps/terminal/ | Terminal/command shell app | Active |
| desktop_app_ui_showcase | crates/apps/ui_showcase/ | Design system component showcase | Active |

### Shell & Terminal Infrastructure (3 crates)

| Crate | Location | Purpose | Status |
|---|---|---|---|
| system_shell | crates/system_shell/ | Command execution, registry, builtins | Active |
| system_shell_contract | crates/system_shell_contract/ | Command registration and I/O contracts | Active |
| shrs_core_headless | crates/shrs_core_headless/ | Minimal shell evaluator (parsing, tokenization) | Active |

### Web & Site (1 crate)

| Crate | Location | Purpose | Status |
|---|---|---|---|
| site | crates/site/ | Leptos single-page app, web entrypoint | Active |

### Reserved/Inactive (1 crate)

| Crate | Location | Purpose | Status |
|---|---|---|---|
| platform_storage | crates/platform_storage/ | Reserved for future storage abstraction | Inactive/Empty |

### Build Automation (1 crate)

| Crate | Location | Purpose | Status |
|---|---|---|---|
| xtask | xtask/ | Cargo automation (docs, perf, e2e, verify) | Active |

## Dependency Matrix

### Core Dependencies

**All crates depend on (directly or transitively):**
- serde (serialization/deserialization)
- thiserror (error types)

**Desktop runtime layer depends on:**
- platform_host (contracts)
- system_ui (shared visuals)
- system_shell (command execution)
- desktop_app_contract (app registration)
- tokio (async runtime)
- leptos (UI framework)

**Apps depend on (standard pattern):**
- desktop_app_contract
- platform_host (for host services)
- system_ui (for visual primitives)
- leptos (for views)

**Browser bridge (platform_host_web) depends on:**
- platform_host (implements)
- web-sys (browser APIs)
- wasm-bindgen (JS interop)
- gloo-storage (browser storage)

**Native bridge (desktop_tauri) depends on:**
- platform_host (implements)
- tauri (native framework)
- tokio (async)

## Crate Sizes & Complexity

| Crate | Code Size | Complexity | Test Coverage |
|---|---|---|---|
| desktop_runtime | ~350 KB | High (reducer, effects, host integration) | Comprehensive |
| system_ui | ~120 KB | Medium (primitives, tokens, icons) | High |
| platform_host | ~80 KB | High (contract definitions, models) | Comprehensive |
| system_shell | ~40 KB | Medium (command execution) | Good |
| site | ~80 KB | Medium (Leptos app, routing) | Moderate |
| Apps (each) | ~20-40 KB | Low-Medium (mostly views) | Moderate |
| platform_host_web | ~70 KB | Medium (WASM adapters) | Good |
| desktop_tauri | ~20 KB | Low (mostly Tauri glue) | Basic |

## Feature Flags

### Per-Crate Features

| Crate | Features | Purpose |
|---|---|---|
| desktop_runtime | default | Standard runtime |
| site | default | Web-only build |
| platform_host | default | Standard contracts |
| desktop_tauri | none | Single profile |

### Workspace-Level Features

None currently defined. Feature decisions are per-crate.

## Cargo Aliases & xtask Commands

See AUTOMATION_COMMANDS.md for full reference. Key entry points:

```bash
cargo verify-fast        # Quick format, tests, docs
cargo verify             # Full verification (all profiles)
cargo xtask docs all     # Validate all docs contracts
cargo xtask docs ui-conformance  # Audit UI tokens/primitives
cargo test --workspace   # Run all tests
cargo doc --workspace --no-deps  # Build rustdoc
```

## Inter-Crate Communication Patterns

### Allowed Patterns

1. **Via platform_host contracts:**
   - Apps call host services (filesystem, cache, etc.)
   - Host context passed through effects

2. **Via system_ui primitives:**
   - Apps render using FluentButton, FluentIcon, etc.
   - All theming through token layer

3. **Via system_shell_contract:**
   - Commands registered and executed
   - I/O marshaled through typed contract

4. **Via desktop_app_contract:**
   - Apps register metadata and lifecycle hooks
   - Runtime discovers and manages apps

### Forbidden Patterns

- Apps importing from desktop_runtime internals
- Apps calling reducer directly
- Bypassing host contracts for native APIs
- Direct inter-app dependencies
- Raw HTML or unthemed CSS in shell/apps

## Version & Maintenance

- **Crate versioning:** All crates at 0.1.0 (pre-1.0 development)
- **MSRV (Minimum Supported Rust Version):** Check Cargo.toml for rustc version
- **Deprecation policy:** Mark deprecated APIs with `#[deprecated]` and rustdoc notes before removal

## Audit Checklist (Quarterly)

- [ ] No unused crate directories or empty src/ folders
- [ ] All public APIs documented via rustdoc
- [ ] ARCHITECTURE.md and this file match actual crate boundaries
- [ ] No circular dependencies (verify via `cargo tree`)
- [ ] All tests pass: `cargo test --workspace`
- [ ] Docs validation passes: `cargo xtask docs all`
- [ ] No orphaned features or unused feature flags
