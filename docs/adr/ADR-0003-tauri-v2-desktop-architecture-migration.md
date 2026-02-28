---
title: "ADR-0003 Tauri v2 Desktop Architecture Migration"
category: "adr"
owner: "architecture-owner"
status: "draft"
last_reviewed: "2026-02-28"
audience: ["engineering", "platform", "release"]
invariants:
  - "Desktop reducer semantics remain platform-agnostic and independent of direct Tauri command implementations."
  - "platform_host contracts remain the canonical typed domain/IPC model for both browser and desktop host implementations."
  - "Filesystem operations execute only within explicitly scoped roots and reject traversal outside those roots."
  - "Browser and desktop distributions continue sharing the same site/desktop_runtime/app crates with adapter-selected host implementations."
tags: ["adr", "architecture", "tauri", "desktop", "migration"]
domain: "architecture"
lifecycle: "draft"
---

# ADR-0003 Tauri v2 Desktop Architecture Migration

## Status

Proposed

## Context

The current system is a browser-hosted desktop shell built with Rust, Leptos, and WASM. The runtime
and app layers are largely host-agnostic, while persistence and filesystem behavior are modeled
through `platform_host` contracts and implemented through `platform_host_web` and `desktop_tauri`.

The project now requires a production-ready desktop distribution with:

- native packaging and installation across macOS, Windows, and Linux
- stricter host security boundaries and permission scoping
- predictable native filesystem access and desktop capability integration
- continued maintainability of the shared runtime/app codebase

ADR-0002 defined the host-boundary direction for browser capabilities. This ADR extends that direction
to a dual-target architecture that supports both browser and Tauri v2 desktop distribution without a
flag-day rewrite.

## Decision

Adopt a staged migration to a Tauri v2 desktop architecture that preserves existing runtime/app crates,
keeps `platform_host` as the canonical typed contract layer, introduces Tauri-native command-backed
host implementations, and converges host adapter selection in `platform_host_web` plus desktop
transport wiring in `desktop_tauri`.

## Decision Details

### 1) Architecture and crate boundaries

- Keep shared frontend/runtime crates:
  - `crates/site`
  - `crates/desktop_runtime`
  - `crates/apps/*`
- Preserve `crates/platform_host` as the canonical typed host-domain boundary.
- Introduce Tauri-native host implementation crate(s) for app state, prefs, cache, and explorer FS.
- Keep browser/Tauri transport selection explicit in host wiring rather than baking it into runtime crates.
- Keep browser (`platform_host_web`) support for web distribution and parity testing.

### 2) IPC command interface contract

The desktop host boundary MUST expose explicit command endpoints aligned to existing platform domains:

- app-state:
  - `app_state_load`
  - `app_state_save`
  - `app_state_delete`
  - `app_state_namespaces`
- prefs:
  - `prefs_load`
  - `prefs_save`
  - `prefs_delete`
- cache:
  - `cache_put_text`
  - `cache_get_text`
  - `cache_delete`
- explorer/fs:
  - `explorer_status`
  - `explorer_pick_root`
  - `explorer_request_permission`
  - `explorer_list_dir`
  - `explorer_read_text_file`
  - `explorer_write_text_file`
  - `explorer_create_dir`
  - `explorer_create_file`
  - `explorer_delete`
  - `explorer_stat`

All request/response payloads MUST use `platform_host` models to avoid contract drift.

### 3) Security and capability model

- Enforce least-privilege Tauri capability scoping per window and command set.
- Reject unscoped filesystem access; canonicalize all paths server-side.
- Treat explorer permission states as explicit modeled outcomes even when they are synthesized in
  desktop mode (instead of browser File System Access permission semantics).
- Validate IPC inputs for namespace format, payload size, and path constraints before execution.
- Keep plugin usage minimal; enable only required plugins (dialog/deep-link/opener/fs as needed).

### 4) Data and compatibility policy

- Preserve stable app-state namespaces and envelope semantics already defined in `platform_host`.
- Preserve explorer metadata/result model compatibility used by app crates.
- Preserve read-fallback-to-cache behavior for explorer text preview flows.
- Provide an explicit import/migration strategy if web-origin persisted data needs to be carried into
  desktop distributions.

## Staged Migration Plan

### Stage 0: Contract freeze and test baseline

- Freeze `platform_host` serialization contracts used for host transport.
- Add/extend contract round-trip tests for all IPC DTOs.
- Document command-level error semantics and compatibility invariants.

### Stage 1: Adapter decoupling

- Refactor browser host bindings behind `platform_host_web` adapters.
- Add explicit host strategy selection for browser vs desktop distributions.
- Keep existing browser behavior unchanged as the first acceptance gate.

### Stage 2: Tauri application shell and workspace integration

- Add Tauri v2 app crate and configuration (`tauri.conf.json` plus capability files).
- Integrate Trunk frontend build hooks into Tauri dev/build entrypoints.
- Extend `xtask`/command catalog with Tauri-specific local workflows.

### Stage 3: Native command implementation

- Implement app-state, prefs, and cache command handlers in Rust.
- Implement explorer filesystem command handlers with scoped-root enforcement.
- Add native folder selection flow and root-state persistence.

## Implementation Status Snapshot (2026-02-28)

- Stage 1 complete: explicit browser/desktop host-strategy selection landed in `platform_host_web`.
- Stage 2 complete: `desktop_tauri` crate/config and command entrypoints landed.
- Stage 3 in progress:
  - landed: typed `app_state_load` / `app_state_save` / `app_state_delete` /
    `app_state_namespaces` plus `prefs_load` / `prefs_save` / `prefs_delete` Tauri command
    handlers, `cache_put_text` / `cache_get_text` / `cache_delete` command handlers,
    and explorer command handlers (`explorer_status`, `explorer_list_dir`,
    `explorer_read_text_file`, `explorer_write_text_file`, `explorer_create_dir`,
    `explorer_create_file`, `explorer_delete`, `explorer_stat`) with scoped-root enforcement.
  - landed: desktop feature wiring (`site` -> `desktop_runtime` -> `platform_host_web` /
    `desktop_tauri`) now routes app-state/prefs/cache/explorer.
  - landed: explorer UI preference hydrate/persist plus runtime theme/terminal-history compatibility
    paths now use typed host prefs helpers.
  - landed: the temporary `platform_storage` facade has been removed from the workspace.
  - pending: hardening/cross-platform validation phases.

### Stage 4: Frontend IPC transport and runtime integration

- Implement WASM-side Tauri transport adapter for command invocation.
- Wire browser/desktop host selection entirely through host adapter composition.
- Integrate desktop deep-link event handling into runtime action dispatch.

### Stage 5: Hardening and cross-platform validation

- Add integration tests for command semantics and path safety.
- Add end-to-end desktop flow tests across macOS, Windows, and Linux targets.
- Validate packaging/signing/notarization/distribution pipeline readiness.

### Stage 6: Rollout and deprecation

- Mark obsolete browser-only bridge surfaces for deprecation where replaced by shared contracts.
- Keep browser distribution path supported unless explicitly retired by a follow-up ADR.

## Consequences

### Positive

- Desktop distribution becomes a first-class supported target.
- Host security boundaries become explicit and enforceable at command/capability level.
- Existing runtime and app crates remain reusable with minimal behavior churn.
- `platform_host` becomes a stronger long-term compatibility and governance boundary.

### Negative

- Build and release complexity increases due to an additional platform target.
- IPC + native I/O paths introduce new performance tuning requirements.
- Additional integration test matrix and packaging infrastructure are required.
- Migration period temporarily increases cognitive load due to dual browser/desktop support.

### Operational

- Documentation and registries must track both browser and Tauri host paths.
- Verification workflows must include Tauri command-path checks in addition to existing web flows.
- Security review scope expands to include Tauri capability files and command exposure.

## Alternatives Considered

### Alternative A: Remain browser-only and avoid desktop packaging

Rejected because it does not satisfy the requirement for production desktop distribution and native
capability integration.

### Alternative B: Full rewrite into a new desktop-only stack

Rejected because it duplicates existing runtime/app logic, increases delivery risk, and discards the
existing host-contract migration work already represented by ADR-0002.

### Alternative C: Keep browser bridge logic and call it from Tauri without typed command boundary

Rejected because it weakens security scoping and observability, and does not provide a robust
long-term host boundary for native capability use.

## Related ADRs

- ADR-0002: Unified Browser Host Abstraction Layer (`docs/adr/ADR-0002-browser-host-abstraction-layer.md`)
