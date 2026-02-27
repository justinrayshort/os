---
title: "Unified Window Manager and App Runtime Contract"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-27"
audience: ["platform", "engineering"]
invariants:
  - "DesktopState remains the authoritative source of truth for window stack, focus, and lifecycle state."
  - "Apps integrate through desktop_app_contract primitives and do not own ad hoc shell window container behavior."
  - "Manager-owned app state persists through WindowRecord.app_state with legacy namespace fallback during migration windows."
tags: ["reference", "desktop-runtime", "window-manager", "contracts"]
domain: "desktop"
lifecycle: "ga"
---

# Unified Window Manager and App Runtime Contract

## Scope

This reference defines the contract-driven desktop window model implemented by `desktop_runtime` and `desktop_app_contract`.

It covers:

- standardized window lifecycle semantics
- reducer actions and runtime effects for app/window orchestration
- shared app mount and command interfaces
- manager-owned app state persistence and migration behavior

It does not define visual design tokens or shell theming behavior.

## Core Components

- `desktop_runtime::model::DesktopState`: authoritative desktop/window state.
- `desktop_runtime::reducer::DesktopAction`: typed runtime intents.
- `desktop_runtime::reducer::RuntimeEffect`: typed side-effect intents.
- `desktop_runtime::window_manager`: reusable stack/focus/snap/resize primitives.
- `desktop_runtime::app_runtime`: per-window lifecycle/inbox signals and topic subscriptions.
- `desktop_app_contract`: app/runtime bridge types used by app crates.

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

## Shared App Contract

`desktop_app_contract` defines the integration contract:

- `AppModule`: module mount primitive used by runtime registry.
- `AppMountContext`: per-window context (window id, launch params, restored state, lifecycle signal, inbox signal, host bridge).
- `AppHost`: command channel from app to runtime.
- `AppCommand`: manager commands (`SetWindowTitle`, `PersistState`, `OpenExternalUrl`, `Subscribe`, `Unsubscribe`, `PublishEvent`, `SetDesktopSkin`, `SetDesktopWallpaper`, `SetDesktopHighContrast`, `SetDesktopReducedMotion`).
- `AppEvent`: topic payload delivered through runtime inbox.
- `SuspendPolicy`: manager suspend behavior (`OnMinimize`, `Never`).

## Runtime Effect Handling

`DesktopHostContext::run_runtime_effect` executes effect intents centrally, including:

- persistence writes (`PersistLayout`, `PersistTheme`, `PersistTerminalHistory`)
- deep-link expansion (`ParseAndOpenDeepLink`)
- host hooks (`OpenExternalUrl`, focus input)
- app runtime dispatch (`DispatchLifecycle`, `DeliverAppEvent`, subscribe/unsubscribe/publish topic routing)

## Persistence Contract and Migration

Manager-owned app state path:

1. app emits `AppCommand::PersistState`.
2. reducer applies `DesktopAction::SetAppState`.
3. `WindowRecord.app_state` is updated.
4. runtime emits `RuntimeEffect::PersistLayout`.

Migration compatibility behavior:

- hydrate path prefers `AppMountContext.restored_state`.
- legacy namespace reads remain fallback during migration windows.
- dual-write is supported during migration windows: manager-owned write + legacy namespace write.

## App Integration Requirements

For any internal desktop app integration:

1. register `AppDescriptor.module` and `AppDescriptor.suspend_policy` in `desktop_runtime::apps`.
2. mount through `AppModule` and consume `AppMountContext`.
3. route title/state/actions through `AppHost` commands (no ad hoc shell mutation paths).
4. preserve restore compatibility by honoring `restored_state` and migration fallback rules.
