---
title: "ADR-0004 Unified Desktop App Platform Contract v2"
category: "adr"
owner: "architecture-owner"
status: "active"
last_reviewed: "2026-02-27"
audience: ["engineering", "platform", "release"]
invariants:
  - "Desktop apps integrate only through desktop_app_contract v2 (`ApplicationId`, `AppServices`, lifecycle/inbox context, and registration descriptors)."
  - "desktop_runtime owns lifecycle orchestration, focus/window-manager semantics, app catalog loading, policy evaluation, and IPC routing."
  - "Host-dependent capability implementations remain behind platform host boundaries (`platform_host`, `platform_host_web`, `desktop_tauri`, `platform_storage`)."
  - "All built-in apps are manifest-backed and declare requested capabilities used for runtime enforcement and policy evaluation."
tags: ["adr", "architecture", "desktop-runtime", "contracts", "capabilities", "ipc"]
domain: "architecture"
lifecycle: "ga"
---

# ADR-0004 Unified Desktop App Platform Contract v2

## Status

Accepted

## Context

The previous desktop app contract centered on enum-based app identifiers and app-facing `AppHost` command forwarding. That model allowed app integration, but it did not provide a single manifest-driven packaging/discovery contract, capability-scoped service injection, or a unified policy surface for privileged and non-privileged app behavior.

As the desktop environment is treated as an integrated platform, app integration needs stronger boundaries:

- canonical and stable app identity across runtime/deep-link/manifest/catalog surfaces
- consistent app lifecycle/mount/focus/teardown integration semantics
- typed, injected app services rather than ad hoc host command usage
- bounded IPC with request/reply metadata and predictable routing behavior
- policy-enforced capability grants with persisted overlay support

## Decision

Adopt `desktop_app_contract` v2 as the mandatory desktop app integration surface and cut over runtime/app modules to v2 in one refactor pass.

### Contract changes

- Replace enum-only app IDs at the contract boundary with canonical string `ApplicationId`.
- Replace app-facing `AppHost` usage with injected `AppServices` in `AppMountContext`.
- Preserve lifecycle semantics and non-veto close behavior.
- Extend IPC envelope metadata with schema/version, request/reply correlation, source identity, and timestamp.
- Standardize manifest-backed app descriptors/registration metadata for discovery and runtime wiring.

### Boundary ownership

- `desktop_runtime` owns window manager semantics, lifecycle orchestration, focus behavior, app catalog consumption, capability/policy enforcement, and IPC routing.
- `platform_host`, `platform_host_web`, `platform_storage`, and `desktop_tauri` own host-dependent capability implementations and transport boundaries.
- `crates/apps/*` consume only `desktop_app_contract` lifecycle and services APIs.

### Packaging/discovery model

- Built-in apps must ship `crates/apps/<app>/app.manifest.toml`.
- `desktop_runtime/build.rs` performs build-time manifest parsing/validation and emits a generated typed catalog payload.
- External package loading remains disabled in this phase, but the extension install path contract is standardized for future activation.

### Capability/policy model

- Apps declare `requested_capabilities` in manifests.
- Built-in privileged app IDs are shell-owned allowlist entries.
- Effective capability grants are evaluated at runtime using declared capabilities and policy overlays persisted in typed prefs (`system.app_policy.v1`).

## Consequences

### Positive

- App integration points are now explicit and consistent across built-in modules.
- Runtime behavior is more deterministic with stronger policy gates and bounded IPC handling.
- Catalog/discovery is standardized and decoupled from ad hoc registry-only metadata.
- Future third-party extension loading has a stable contract foundation.

### Trade-offs

- Contract cutover is breaking for app modules using legacy `AppHost` usage patterns.
- Runtime still carries internal enum app IDs for some internal-only paths; this is acceptable in the current phase because external boundaries now use canonical IDs.
- Capability policy overlays are intentionally simple for this phase and may be extended in follow-up ADRs.

## Alternatives Considered

### Alternative A: Keep `AppHost` and add incremental wrappers

Rejected because it preserves fragmented integration patterns and postpones consistent capability scoping.

### Alternative B: Split into spec-only phase before implementation

Rejected for this change set because the workspace required a single integration cutover to eliminate parallel legacy/new paths.

## Related Artifacts

- `docs/reference/unified-window-manager-and-app-contract.md`
- `crates/desktop_app_contract/src/lib.rs`
- `crates/desktop_runtime/build.rs`
- `crates/desktop_runtime/src/apps.rs`
- `crates/desktop_runtime/src/reducer.rs`
- `docs/adr/ADR-0003-tauri-v2-desktop-architecture-migration.md`
