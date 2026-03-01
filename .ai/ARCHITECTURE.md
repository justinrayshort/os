# Architecture: Crate Topology & Boundaries

**Version:** 1.0  
**Last Updated:** 2026-03-01  
**Audience:** Claude, code reviewers, architecture auditors

## High-Level System Composition

```
┌─────────────────────────────────────────────────────┐
│  User-Facing Layer                                   │
├──────────────────┬──────────────────┬────────────────┤
│  crates/site     │  crates/desktop_ │  crates/apps/* │
│  (Leptos web)    │  tauri (Tauri)   │  (Calculator,  │
│                  │                  │   Explorer,    │
│                  │                  │   Notepad,     │
│                  │                  │   Terminal,    │
│                  │                  │   Settings,    │
│                  │                  │   UI Showcase) │
├──────────────────┴──────────────────┴────────────────┤
│  Shared Shell & Runtime Layer                        │
├─────────────────────────┬───────────────────────────┤
│  crates/desktop_runtime │  crates/system_ui         │
│  (State, Reducer,       │  (Shared visual          │
│   Effects, App Bus)     │   primitives, icons,     │
│                         │   theme tokens)          │
├─────────────────────────┴───────────────────────────┤
│  Host Boundary Layer (Domain Abstraction)           │
├──────────────────┬──────────────────────────────────┤
│  crates/platform │  crates/platform_host_web       │
│  _host           │  (Browser/wasm implementations  │
│  (Typed          │   of platform_host contracts)   │
│   contracts,     │                                 │
│   models)        │  crates/desktop_tauri           │
│                  │  (Native transport, Tauri glue) │
├──────────────────┴──────────────────────────────────┤
│  Terminal & Shell Infrastructure                    │
├──────────────────┬────────────────────────────────┤
│  crates/system   │  crates/system_shell_contract  │
│  _shell          │  (Command spec, I/O contract)  │
│  (Terminal       │                                │
│   execution,     │  crates/shrs_core_headless    │
│   shell logic)   │  (Minimal shell evaluator)     │
├──────────────────┴────────────────────────────────┤
│  Supporting & Contracts                            │
├─────────────────────────────────────────────────────┤
│  crates/desktop_app_contract (App registration)    │
│  crates/platform_storage (RESERVED, currently      │
│   empty; may hold storage abstraction in future)   │
└─────────────────────────────────────────────────────┘
```

## Crate Purposes & Invariants

### User-Facing Crates

**crates/site** (Leptos web entrypoint)
- Single-page application for browser deployment
- Routes, deep-link parsing, browser mount points
- Depends on: desktop_runtime (via platform_host_web bridge), system_ui, platform_host
- Invariant: Contains Leptos view definitions and routing; no shell business logic

**crates/apps/\*** (Built-in applications)
- Independent app implementations: calculator, explorer, notepad, terminal, settings, ui_showcase
- Each registers with desktop_app_contract and loads via desktop_runtime
- Invariant: No direct access to desktop_runtime reducer; use provided effects and host services
- Invariant: All UI must consume system_ui primitives

**crates/desktop_tauri** (Tauri native bootstrap)
- Native window, process lifecycle, IPC transport
- Depends on: desktop_runtime, platform_host (owns the impl)
- Invariant: Platform-host is the single point of native/wasm API boundary

### Shared Shell & Runtime

**crates/desktop_runtime** (Core state machine)
- Reducer-driven state, effects executor, app bus, host integration
- Centralized desktop/web shell state and effects (wallpaper, persistence, preferences, app lifecycle)
- Depends on: platform_host, system_ui, system_shell, desktop_app_contract
- Invariant: Single reducer; all state mutations via actions/effects
- Invariant: No direct external API calls from reducer; route through effects

**crates/system_ui** (Shared visual layer)
- Semantic icons (FluentIcon, IconName enum), themed tokens, primitive components
- Centralized theming architecture; enables atomic design-system changes
- Invariant: No raw HTML or unthemed CSS; all styling via token layer
- Invariant: Shared primitives (FluentButton, FluentIcon, etc.) are the only UI building blocks for shell/apps
- Invariant: No app-local layout-only class contracts

### Host Boundary & Platform Abstraction

**crates/platform_host** (Typed contracts & models)
- Defines all host-domain services (filesystem, cache, notifications, terminal process, external URLs, wallpaper, session)
- Defines error types, request/response envelopes, persistence models
- Invariant: No implementation; platform_host is pure contract/type definitions
- Invariant: Used by both browser (platform_host_web impl) and native (desktop_tauri impl)

**crates/platform_host_web** (Browser/wasm implementations)
- IndexedDB cache, browser storage adapters, mock terminal, fetch-based external URL dispatch
- Implements platform_host traits for browser environment
- Invariant: Contains adapters only; business logic remains in desktop_runtime
- Invariant: All state persisted via IndexedDB or browser storage

**crates/desktop_tauri** (Native implementations)
- Tauri-managed file system, native cache, OS notifications, actual terminal process
- Owns Tauri bootstrap and window lifecycle
- Invariant: Single point of native/wasm API boundary; all native APIs routed through typed platform_host contracts

### Terminal & Shell

**crates/system_shell** (Command execution & builtins)
- Command registry, execution context, builtin commands (apps, filesystem, config, theme, data, windows, inspect)
- Orchestrates command parsing, execution, and output marshaling
- Depends on: system_shell_contract, shrs_core_headless, desktop_runtime
- Invariant: Commands are registered via system_shell_contract traits; no hardcoded command dispatch

**crates/system_shell_contract** (Command & I/O schema)
- Defines command registration interface, argument spec, output shape
- Defines I/O contracts for command execution
- Invariant: Pure contract; no business logic

**crates/shrs_core_headless** (Minimal shell evaluator)
- Line tokenization, quoting/escaping, argument-vector construction
- Intentionally minimal; only features needed by terminal app
- Invariant: No external dependencies; strictly parsing/tokenization

### Supporting Crates

**crates/desktop_app_contract** (App registration)
- Defines app metadata, mount requirements, lifecycle hooks
- Used by apps to register with desktop_runtime
- Invariant: Pure contract; enables app discovery and loading

**crates/platform_storage** (RESERVED)
- Currently empty; reserved for future storage abstraction layer
- Do not use in current implementation

**xtask** (Local automation)
- Cargo command façade for docs validation, performance profiling, E2E testing
- Implements validation rules, linting, artifact generation
- Invariant: No build-time logic; purely local workflow orchestration

## Dependency Rules

### Allowed Dependencies
- Apps may depend on: desktop_app_contract, platform_host, system_ui, system_shell_contract
- Apps must NOT depend on: desktop_runtime (use host services instead)
- desktop_runtime may depend on: platform_host, system_ui, system_shell, desktop_app_contract
- platform_host_web may depend on: platform_host (implements), desktop_runtime (for host context)
- desktop_tauri may depend on: platform_host (implements), desktop_runtime (for host context)

### Forbidden Dependencies
- No circular dependencies between crates
- No app-to-app dependencies
- No bypassing platform_host contracts for host services
- No direct imports of shell/runtime internals from apps

## Host Boundary Invariants

The **platform_host** layer defines the typed contract between:
- **Browser/WASM side** (platform_host_web implementation)
- **Native/Tauri side** (desktop_tauri implementation)
- **Consumers** (desktop_runtime, apps via host services)

All cross-boundary communication flows through:
1. Typed request/response envelopes (platform_host models)
2. Async effects in desktop_runtime
3. Platform-specific implementations (platform_host_web for browser, desktop_tauri for native)

**Never bypass this boundary** by directly accessing native APIs from runtime code or apps.

## Change Propagation

When modifying crates, coordinate updates across layers:

- **platform_host contract change** → Requires updates in platform_host_web AND desktop_tauri
- **system_ui token/primitive change** → Requires audit via `cargo xtask docs ui-conformance`
- **system_shell command change** → Update system_shell_contract and all command registrations
- **desktop_runtime reducer change** → Update rustdoc, model comments, and Wiki explanations

All cross-boundary changes require docs updates per AGENTS.md section 6.
