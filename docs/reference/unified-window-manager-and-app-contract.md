---
title: "Unified Window Manager and App Runtime Contract"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-27"
audience: ["platform", "engineering"]
invariants:
  - "DesktopState remains the authoritative source of truth for window stack, focus, lifecycle state, and manager-owned per-window app state."
  - "Apps integrate through desktop_app_contract v2 (`ApplicationId`, `AppServices`, lifecycle/inbox context) and do not mutate shell state via ad hoc integration paths."
  - "Runtime capability enforcement uses declared app capabilities with policy overlays before executing host-sensitive effects."
tags: ["reference", "desktop-runtime", "window-manager", "contracts"]
domain: "desktop"
lifecycle: "ga"
---

# Unified Window Manager and App Runtime Contract

## Scope

This reference defines the v2 contract-driven desktop window model implemented by `desktop_runtime` and `desktop_app_contract`.

It covers:

- standardized window lifecycle semantics
- reducer actions and runtime effects for app/window orchestration
- shared app mount/service interfaces
- manager-owned app state and app-shared state persistence
- capability and policy gates for app-originated runtime commands
- manifest-driven app catalog/discovery constraints

## Core Components

- `desktop_runtime::model::DesktopState`: authoritative desktop/window state.
- `desktop_runtime::reducer::DesktopAction`: typed runtime intents.
- `desktop_runtime::reducer::RuntimeEffect`: typed side-effect intents.
- `desktop_runtime::window_manager`: reusable stack/focus/snap/resize primitives.
- `desktop_runtime::app_runtime`: per-window lifecycle/inbox signals and topic subscriptions.
- `desktop_runtime::apps`: app descriptors, built-in mount mapping, and declared capability surfaces.
- `desktop_app_contract`: v2 app/runtime bridge types used by app crates.

## Window Lifecycle Model

Lifecycle events dispatched by the manager:

- `Mounted`
- `Focused`
- `Blurred`
- `Minimized`
- `Restored`
- `Suspended`
- `Resumed`
- `Closing`
- `Closed`

`WindowRecord.last_lifecycle_event` stores the latest lifecycle token for persisted windows.
Close remains non-veto by app modules.

## Shared App Contract (v2)

`desktop_app_contract` defines the app integration contract:

- `ApplicationId`: canonical namespaced dotted app identifier (`system.settings`, `system.terminal`, ...).
- `AppModule`: module mount primitive used by runtime registry.
- `AppMountContext`: per-window context (`window_id`, `app_id`, `launch_params`, `restored_state`, `lifecycle`, `inbox`, injected `services`).
- `AppServices`: typed service bundle injected at mount:
  - `WindowService`
  - `StateService`
  - `ConfigService`
  - `AppStateHostService`
  - `PrefsHostService`
  - `ExplorerHostService`
  - `CacheHostService`
  - `ThemeService`
  - `WallpaperService`
  - `NotificationService`
  - `IpcService`
  - `CommandService`

`AppServices` does not expose a raw transport send hook; apps integrate through the typed services above.
`ConfigService` now provides typed namespaced reads through the runtime-selected prefs backend and
keeps writes on the runtime command path so config remains on the formal app/runtime integration
surface instead of ad hoc host imports.
- `IpcEnvelope`: typed IPC payload (`schema_version`, `topic`, `correlation_id`, `reply_to`, `source_app_id`, `payload`, `timestamp_unix_ms`).
- `AppRegistration`: manifest-backed app registration descriptor model.
- `SuspendPolicy`: manager suspend behavior (`OnMinimize`, `Never`).
- `window_primary_input_dom_id(window_id)`: stable DOM anchor apps can opt into so `FocusWindowInput` restores keyboard focus to the correct field.

## IPC Contract and Routing

IPC topic format is canonical and versioned:

- `app.<app_id>.<channel>.v1`

Runtime routing behavior:

- per-window inboxes are bounded ring buffers (default capacity `256`)
- overflow policy is drop-oldest with deterministic counters
- request/reply correlation uses `correlation_id` and optional `reply_to`

## Capability and Policy Enforcement

- Apps declare requested capabilities in manifest metadata and runtime descriptors.
- Runtime maps command intents to required capabilities and rejects unauthorized operations.
- Built-in privileged app IDs are allowlisted by shell policy.
- Policy overlay persistence key: `system.app_policy.v1`.
- Effective grants combine declared capabilities and policy overlay evaluation.

## Runtime Effect Handling

`DesktopHostContext::run_runtime_effect` executes effect intents centrally, including:

- persistence writes (`PersistLayout`, `PersistTheme`, `PersistTerminalHistory`)
- deep-link expansion (`ParseAndOpenDeepLink`)
- host hooks (`OpenExternalUrl`, focus input)
- app runtime dispatch (`DispatchLifecycle`, `DeliverAppEvent`, subscribe/unsubscribe/publish topic routing)
- config and notification host operations (`SaveConfig`, `Notify`)

`OpenExternalUrl` now executes through the runtime-selected host bundle's explicit external URL
service, using browser `window.open(...)` fallback in web builds and the Tauri opener command on
desktop-host builds.

## Persistence Contract

Manager-owned state paths:

1. App persists per-window state through `StateService::persist_window_state`.
2. Reducer updates `WindowRecord.app_state`.
3. Runtime emits `RuntimeEffect::PersistLayout`.

Shared app state path:

1. App writes keyed shared payload via `StateService::persist_shared_state`.
2. Reducer updates `DesktopState.app_shared_state`.
3. Snapshot/hydration round-trips shared state with desktop layout persistence.

## App Integration Requirements

For any built-in desktop app integration:

1. Define `crates/apps/<app>/app.manifest.toml` with v2 schema metadata and declared capabilities.
2. Register app descriptor/module/suspend policy in `desktop_runtime::apps`.
3. Mount via `AppModule` and consume `AppMountContext` + injected `AppServices`.
4. Use canonical IDs for deep links and app registry routing (`system.<name>` form).
5. Route app-originated shell requests through service APIs only (no ad hoc runtime mutation paths).

## Discovery and Packaging Constraints (Current Phase)

- `desktop_runtime/build.rs` validates manifests and generates catalog constants consumed at runtime.
- Built-in modules are the only loadable runtime app modules in this phase.
- External third-party package loading is intentionally disabled by policy in this phase.
