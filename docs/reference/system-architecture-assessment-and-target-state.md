---
title: "System Architecture Assessment and Target State"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-28"
audience: ["engineering", "platform"]
invariants:
  - "This assessment must remain evidence-based and reference concrete code structure, dependency direction, or runtime behavior rather than intended architecture alone."
  - "Target-state recommendations must preserve current browser and Tauri compatibility constraints unless an explicit migration step says otherwise."
  - "Architectural findings are prioritized for feature scaling first, then by user impact, refactor complexity, and compatibility risk."
tags: ["reference", "architecture", "assessment", "runtime", "host-boundary", "technical-debt"]
domain: "desktop"
lifecycle: "draft"
---

# System Architecture Assessment and Target State

This document records a focused architectural assessment of the current workspace and a target-state architecture proposal. It is intentionally grounded in the current crate graph, source structure, and runtime wiring rather than design intent alone.

Assessment date: 2026-02-28

Status note: the first target-state boundary change from this assessment has now been implemented in the workspace. Wallpaper domain models were moved from `desktop_app_contract` into `platform_host`, so the assessment's original host-boundary leakage finding is now partially addressed and remains here as historical diagnosis context for the broader refactor program.

Primary evidence set:

- `cargo metadata --no-deps --format-version 1`
- `cargo check --workspace`
- `cargo xtask docs storage-boundary`
- targeted source inspection across `desktop_runtime`, `desktop_app_contract`, `platform_host`, `platform_host_web`, `desktop_tauri`, `site`, `system_shell`, and `crates/apps/*`

## Executive Summary

The workspace has a coherent high-level direction: a reducer-driven desktop runtime, manifest-backed app registration, typed host contracts, and browser/Tauri host implementations. The main structural problem is that the codebase only partially honors those boundaries in practice.

The highest-risk issues are:

1. The host boundary leaks app/runtime-facing wallpaper types back into `platform_host`, so host contracts are not independent of the app contract.
2. `desktop_runtime` has become a runtime monolith that owns state, reducer logic, app registration, host orchestration, shell integration, and a large share of the UI.
3. Built-in apps and runtime modules previously bypassed formal service injection and reached directly into `platform_host_web`, which meant the formal boundaries were not the actual extension seams.
4. Persistence ownership was split between runtime-managed `restored_state` and direct app-owned namespace storage, creating inconsistent state semantics for multi-instance and future third-party app scenarios.
5. Several typed interfaces are asymmetric or incomplete enough that behavior still depends on informal coordination, especially around config, policy overlays, and host effect execution.

The current architecture scales acceptably for a small set of built-in apps. It does not scale cleanly for:

- more host strategies
- more built-in apps
- third-party or optional app loading
- richer policy/config surfaces
- larger volumes of async workflows and host mutations

The recommended target state is:

- move all host-domain models into `platform_host`
- reduce `desktop_app_contract` to app/runtime integration concerns only
- make `AppServices` the real built-in app integration boundary
- decompose `desktop_runtime` into clearer internal subsystems
- establish one authoritative persistence model for window-local vs app-shared state
- replace direct `platform_host_web` calls from apps and runtime features with an injected host service bundle

## Current Architecture Map

### Crate roles

| Crate | Current role | Observed architectural posture |
| --- | --- | --- |
| `site` | Web entry, routes, deep-link bootstrap | Thin entry layer; relatively clean |
| `desktop_runtime` | Desktop state, reducer, shell UI, app registry, host effect execution, shell commands | Architectural center of gravity and largest concentration of mixed responsibilities |
| `desktop_app_contract` | App/runtime integration contract | Broad and useful, but not sufficient to keep apps off concrete host adapters |
| `platform_host` | Typed host-domain contracts | Mostly clean, but not independent because wallpaper contract types come from `desktop_app_contract` |
| `platform_host_web` | Browser/Tauri adapter selection and concrete host wiring | Useful adapter layer, but it has become a concrete dependency of apps and runtime modules |
| `desktop_tauri` | Native command transport and desktop bootstrap | Cleanly acyclic, but mostly a transport/provider crate rather than a full runtime boundary |
| `system_shell_contract` | Shell command contracts | Clean and focused |
| `system_shell` | Shell engine and session runtime | Focused, but tied to Leptos reactive primitives |
| `crates/apps/*` | Built-in app UI and app-local behavior | Mixed: some use `AppServices`, several also depend on concrete host adapters directly |

### High-value dependency observations

From `Cargo.toml` manifests and `cargo metadata`:

- `desktop_runtime` depends directly on every built-in app crate, `platform_host`, `platform_host_web`, `system_shell`, and both contract crates.
- `platform_host` depends on `desktop_app_contract`.
- `platform_host_web` depends on `desktop_app_contract` and `platform_host`.
- several app crates depend directly on both `platform_host` and `platform_host_web`.

This produces an acyclic Rust crate graph, but the logical layering is weaker than the docs suggest:

- host contracts are not independent from app/runtime contract types
- built-in apps are not independent from concrete host implementations
- runtime modules are not independent from concrete browser/Tauri adapter selection

### Runtime control flow

Current primary runtime path:

1. `site` parses URL/deep-link state and dispatches runtime actions through `DesktopProvider`.
2. `desktop_runtime::reduce_desktop(...)` mutates `DesktopState` and emits `RuntimeEffect` values.
3. `DesktopProvider` stores effects in a reactive queue and runs them in a `create_effect`.
4. `DesktopHostContext::run_runtime_effect(...)` routes host-sensitive effects into persistence, wallpaper, notification, app-bus, and host UI helpers.
5. those helper modules historically called `platform_host_web::*_service()` factories directly rather than consuming one runtime-injected host bundle.
6. built-in apps originally mounted through `desktop_runtime::apps` but also reached into concrete host factories for persistence and filesystem work; the app side of that drift has now been removed.

### Scale indicators

Line counts in the architectural hot path:

| File | Lines |
| --- | ---: |
| `crates/desktop_runtime/src/reducer.rs` | 1878 |
| `crates/desktop_runtime/src/components.rs` | 1022 |
| `crates/desktop_runtime/src/shell.rs` | 963 |
| `crates/apps/explorer/src/lib.rs` | 954 |
| `crates/apps/terminal/src/lib.rs` | 922 |
| `crates/desktop_runtime/src/apps.rs` | 768 |
| `crates/desktop_app_contract/src/lib.rs` | 1290 |
| `crates/system_shell/src/lib.rs` | 1132 |

Large files are not automatically an architectural flaw, but here they correlate with responsibility concentration and boundary mixing.

## Prioritized Findings

Scores use a 1-5 scale where 5 is highest.

| Priority | Finding | Risk | Feature-scaling impact | Refactor complexity | Compatibility risk |
| --- | --- | ---: | ---: | ---: | ---: |
| P1 | Host boundary leakage through wallpaper/domain types | 5 | 5 | 3 | 3 |
| P1 | `desktop_runtime` is a runtime monolith | 5 | 5 | 5 | 3 |
| P1 | Built-in apps bypass `AppServices` and depend on `platform_host_web` | 5 | 5 | 4 | 4 |
| P1 | Persistence ownership is duplicated and inconsistent | 5 | 5 | 4 | 5 |
| P2 | Interfaces are typed but incomplete or asymmetric | 4 | 4 | 3 | 3 |
| P2 | Extension mechanism is static and runtime-coupled | 4 | 5 | 4 | 2 |
| P2 | Reactive sequencing is implicit rather than explicit | 4 | 4 | 4 | 2 |
| P3 | Governance checks do not enforce the documented boundary strongly enough | 3 | 4 | 2 | 1 |
| P3 | Some host integration points remain coarse or placeholder-grade | 3 | 3 | 2 | 2 |

## Detailed Evidence by Subsystem

### 1. Host boundary leakage

#### Evidence

- `platform_host` depends on `desktop_app_contract` in `crates/platform_host/Cargo.toml`.
- `platform_host::wallpaper` imports `ResolvedWallpaperSource`, `WallpaperAssetRecord`, `WallpaperCollection`, `WallpaperImportRequest`, `WallpaperLibrarySnapshot`, and `WallpaperSelection` from `desktop_app_contract`.
- `platform_host_web::adapters` and `platform_host_web::wallpaper` also import those same types from `desktop_app_contract`.

#### Why this is a weakness

The intended layering says:

- `platform_host` defines host-domain contracts
- `desktop_app_contract` defines app/runtime integration contracts

In the code, wallpaper asset models and selection types are shared in the opposite direction. That means:

- host-domain service definitions are not host-owned
- changing wallpaper behavior for rendering or app UX can force host contract changes
- host portability is coupled to a UI-facing model crate

#### Structural implication

`platform_host` is not a truly independent compatibility boundary. It is partially downstream of the app/runtime contract layer.

### 2. Runtime monolith and mixed responsibilities

#### Evidence

- `desktop_runtime::components` owns `DesktopProvider`, reducer dispatch, effect queue processing, app runtime sync, and shell engine registration.
- `desktop_runtime::host` owns effect execution dispatch, but that dispatch is still hardwired to concrete helper modules.
- `desktop_runtime::apps` owns manifest-backed registry construction, built-in app mount wiring, legacy app-id compatibility mappings, and placeholder app implementations.
- `desktop_runtime::shell` owns runtime-side command bridging, structured rendering, filesystem-backed command behavior, and app command registration.
- `desktop_runtime::reducer` owns window management, app command routing, capability enforcement, theme/wallpaper transitions, deep-link behavior, and lifecycle effect emission.

#### Why this is a weakness

The crate is not just the runtime core. It is simultaneously:

- state and reducer core
- host execution coordinator
- app framework
- shell command integration layer
- UI composition layer
- registry and discovery layer

That concentration increases:

- merge pressure
- change blast radius
- difficulty isolating tests by subsystem
- onboarding cost for contributors adding features in only one area

#### Structural implication

There is no small, stable runtime core to build around. New capabilities will keep landing in the same crate and usually in the same few files.

### 3. App contract bypasses by built-in apps

#### Evidence

Direct concrete host-adapter imports:

- `crates/apps/notepad/src/lib.rs` imports `platform_host_web::app_state_store`
- `crates/apps/calculator/src/lib.rs` imports `platform_host_web::app_state_store`
- `crates/apps/terminal/src/lib.rs` imports `platform_host_web::app_state_store`
- `crates/apps/explorer/src/lib.rs` imports `platform_host_web::{app_state_store, content_cache, explorer_fs_service, prefs_store}`

At the same time, the runtime injects `AppServices` into every mounted app through `AppMountContext`.

#### Why this is a weakness

The formal app contract says apps should integrate through injected services and runtime-managed lifecycle/inbox/state surfaces. In practice, built-in apps still need concrete adapter access for real behavior:

- persistence hydration and direct saves
- filesystem actions
- preference writes
- cache fallback behavior

That means `AppServices` is not the actual extension seam. The actual seam is a hybrid of:

- `AppServices`
- `platform_host` helpers
- `platform_host_web` factories
- app-local assumptions about namespace ownership and storage topology

#### Structural implication

Feature scaling to more apps, alternative host strategies, or package-loaded apps is constrained because the integration contract is incomplete.

### 4. Inconsistent and duplicated persistence ownership

#### Evidence

Patterns in app crates:

- Notepad restores runtime-provided `restored_state`, then separately loads and saves `app.notepad` via `app_state_store()`.
- Calculator does the same with `app.calculator`.
- Terminal does the same with `app.terminal`.
- Explorer persists manager-owned window state and also writes a direct `app.explorer` namespace snapshot, but its current implementation does not symmetrically hydrate from that namespace in the same path the other apps do.

Runtime-level persistence also exists:

- `DesktopAction::SetAppState` and `SetSharedAppState` store app-local and shared data in `DesktopState`.
- `DesktopSnapshot` persists those runtime-managed values.

#### Why this is a weakness

There are two competing state ownership models:

1. runtime-managed window state via `restored_state`
2. app-owned namespace state via direct host store access

This creates ambiguity for:

- multi-instance behavior
- cross-window shared state
- migration ownership
- which layer is allowed to evolve schemas

The problem is sharper for multi-instance apps. For example:

- `system.notepad` is multi-instance
- `system.explorer` is multi-instance
- both also write direct app-global namespaces

That can cause state convergence across windows where per-window isolation is expected.

#### Structural implication

The persistence contract is unstable because state locality is not encoded clearly enough in the architecture.

### 5. Interface inconsistency and incomplete abstraction

#### Evidence

- `AppServices::send(...)` existed as a low-level escape hatch during the initial assessment and
  has now been removed from the app contract.
- runtime config persistence is implemented as `format!("{}.{}", namespace, key)` plus a prefs write.
- docs mention policy overlays, and `APP_POLICY_KEY` plus load/save helpers exist, but reducer enforcement currently checks only manifest-requested capabilities and privileged app IDs.

#### Why this is a weakness

The interfaces are typed, but they are not yet fully opinionated. That leaves room for ad hoc behavior:

- config semantics remain stringly and namespace/key based even after the app-facing read path was
  added
- the app/runtime boundary previously relied on transport escape hatches, which indicates how thin
  some service contracts were before the current hardening pass
- policy overlay persistence exists without corresponding runtime enforcement

#### Structural implication

Interface stability is stronger than it was at the start of this assessment, but the implementation
still depends on convention and discipline in the remaining policy/config paths.

### 6. Extension mechanism is static and runtime-coupled

#### Evidence

- `desktop_runtime::apps::build_app_registry()` constructs a static vector of all built-in app descriptors.
- `desktop_runtime/build.rs` validates manifests and generates catalog artifacts, but runtime mount mapping remains centralized in `desktop_runtime::apps`.
- docs state built-in modules are the only loadable runtime app modules in the current phase.

#### Why this is a weakness

Adding an app requires editing:

- workspace manifests
- runtime crate features/dependencies
- runtime registry wiring
- manifest catalog generation inputs
- often deep-link routing

That is manageable now, but it scales poorly if the system later wants:

- optional built-in apps
- app packages
- app bundles loaded by configuration
- host-specific app availability

#### Structural implication

The extension mechanism is manifest-driven at the metadata level but runtime-hardcoded at the instantiation level.

### 7. Reactive sequencing and effect orchestration are implicit

#### Evidence

- `DesktopProvider` queues reducer effects in a `RwSignal<Vec<RuntimeEffect>>` and drains them in a `create_effect`.
- boot hydration is performed in a `create_effect` that issues multiple async calls and dispatches runtime actions in sequence.
- app crates use `create_effect` plus `spawn_local` for hydration and persistence loops.
- Explorer and Terminal especially coordinate significant behavior through reactive signal changes and async closures.

#### Why this is a weakness

This approach is idiomatic for Leptos, but the architecture increasingly depends on the timing semantics of:

- signal snapshots
- untracked reads
- effect re-execution
- async tasks closing over captured values

That works for current scale, but as workflows grow more complex it becomes harder to reason about:

- ordering guarantees
- duplicate async work
- cancellation
- race conditions between hydration and user interaction

#### Structural implication

The system has a clear reducer/effect idea, but not all asynchronous control flow is expressed through that model.

### 8. Enforcement/documentation gap

#### Evidence

- docs and wiki guidance say app/runtime/site code should use typed helpers and avoid low-level boundary bypasses.
- `cargo xtask docs storage-boundary` passes.
- the actual validator logic in `xtask/src/docs.rs` only checks for legacy `load_app_state_envelope(...)` usage patterns, largely tied to historical `platform_storage`.

#### Why this is a weakness

The check enforces one narrow anti-pattern. It does not flag:

- direct `platform_host_web::app_state_store()` calls in apps
- direct `prefs_store()` or `explorer_fs_service()` adapter usage in apps
- concrete adapter imports that bypass the app/runtime contract

#### Structural implication

Architecture drift can continue while validation remains green. The docs currently promise stronger discipline than the tool actually enforces.

### 9. Ad hoc or incomplete host integration points

#### Evidence

- `OpenExternalUrl` now resolves through a typed `platform_host::ExternalUrlService` and no longer
  relies on a runtime-side log placeholder.
- the browser bridge opens URLs with `window.open(...)` when no Tauri transport is present, and
  the desktop host now exposes a dedicated `external_open_url` command backed by
  `tauri-plugin-opener`.
- wallpaper import and destructive mutations still reload the full library snapshot even though
  metadata and collection upserts now update runtime state directly.

#### Why this is a weakness

These paths are not broken, but they show where the contract surface is still coarse:

- URL opening is now a stable cross-host capability, but it still depends on coarse-grained
  transport fallback logic in the browser bridge
- wallpaper import/delete flows still use whole-library refreshes instead of narrower update
  semantics

#### Structural implication

Some current contracts are still transitional or placeholder-grade, which will become more visible as data volumes or host features grow.

## Target Architecture

### 1. Boundary realignment

Target boundary rules:

- `platform_host` owns all host-domain models, including wallpaper data types and selection/source records.
- `desktop_app_contract` owns only app/runtime integration contracts:
  - `ApplicationId`
  - lifecycle and inbox types
  - app mount context
  - app service traits or service handles
  - app command registration and command-session contracts
- `platform_host_web` and `desktop_tauri` implement `platform_host` contracts without importing app-layer or UI-layer model types.
- built-in apps depend on `desktop_app_contract` and host-agnostic models only, not on `platform_host_web`.

#### Immediate mapping

Move these concepts out of `desktop_app_contract` and into `platform_host`:

- wallpaper asset records
- wallpaper collections and library snapshots
- wallpaper import requests
- resolved wallpaper source
- wallpaper selection/source metadata used by host services

### 2. Runtime decomposition

Target `desktop_runtime` internal subsystem map:

| Target subsystem | Current source concentration | Responsibility |
| --- | --- | --- |
| runtime core | `model.rs`, `reducer.rs`, `window_manager.rs` | state, actions, reducer, lifecycle semantics, effect intents |
| app framework | `apps.rs`, mount wiring in `components/window.rs` | app registry, capability evaluation, service injection, app lifecycle/session coordination |
| shell integration | `shell.rs`, `shell/commands/*`, `shell/policy.rs` | command bridge, built-in command pack, shell policy integration |
| host execution | `host.rs`, `host/*`, `persistence.rs` | effect execution, host bundle injection, persistence execution, boot hydration |
| shell UI | `components.rs`, `components/*`, `wallpaper.rs`, `icons.rs` | Leptos components, theming surfaces, desktop shell visuals |

#### Key rule

The reducer core should not know about:

- concrete adapter factories
- Leptos component structure
- manifest-loading details
- full shell command registry behavior

### 3. Host service bundle

Introduce a host service bundle trait or injected adapter object that `desktop_runtime` consumes, for example:

- app-state store
- prefs/config store
- explorer filesystem service
- content cache
- notification service
- wallpaper asset service
- external opener

This bundle should be composed in:

- browser mode by `platform_host_web`
- desktop mode by `desktop_tauri` plus host-web bridge transport where needed

The runtime should consume that bundle through traits or erased service objects, not via direct `platform_host_web::*_service()` calls.

### 4. Persistence model

Target persistence policy:

- window-local UI/session state is manager-owned and restored only through `restored_state`
- app-shared state is explicit and uses `StateService::persist_shared_state`
- direct app-level access to app-state stores is not allowed in built-in apps
- namespaces are owned by one layer only:
  - runtime-owned layout and app-shared namespaces
  - host-domain-owned namespaces for host-managed libraries or caches

#### Concrete outcome

- Notepad, Calculator, and Terminal should not independently hydrate from app-global namespaces when they already receive runtime-restored state.
- Explorer should choose one authoritative owner for persisted view/editor/session state.

### 5. Extension mechanism

Target extension path:

- app manifests remain the source of metadata and capability declarations
- runtime mount resolution becomes table-driven from generated catalog data plus a mount-provider registry
- built-in app loading can remain compile-time for now
- app registration logic should no longer require one large hand-maintained vector with hardcoded labels, icon behavior, and policy overrides in a single file

This preserves current constraints while reducing the cost of future package or optional-module work.

## Phased Remediation Roadmap

### Phase 1: Boundary hardening without behavior change

- move wallpaper host-domain types from `desktop_app_contract` to `platform_host`
- update `platform_host_web` and runtime imports accordingly
- document external URL opening as an explicit host capability
- tighten docs to describe current policy overlay enforcement accurately

Success criteria:

- `platform_host` no longer depends on `desktop_app_contract`
- browser and Tauri host crates do not import app/runtime-only wallpaper types

### Phase 2: Make `AppServices` the real app boundary

- add missing app-service capabilities needed by built-in apps
- remove direct `platform_host_web` imports from built-in apps
- narrow or remove `AppServices::send`
- add symmetric typed config read APIs if config remains an app service

Status update:

- implemented for built-in apps `calculator`, `explorer`, `notepad`, and `terminal`
- `AppServices` now injects host-backed wrappers for app-state, prefs, explorer/filesystem, and cache access
- those apps no longer depend on `platform_host_web` directly
- the low-level `AppServices::send(...)` escape hatch has been removed

Success criteria:

- built-in apps can compile without `platform_host_web`
- app behavior routes through injected services rather than concrete host factories

### Phase 3: Normalize persistence ownership

- define which app state is window-local vs shared
- migrate built-in apps away from direct app-global namespace hydration where not required
- reserve direct namespaces for host-owned domains or explicit shared app state only

Status update:

- implemented for built-in apps `calculator`, `explorer`, `notepad`, and `terminal`
- those apps now treat window/session state as manager-owned and persist through `persist_window_state`
- direct app-global namespace persistence has been removed from those apps

Success criteria:

- multi-instance apps no longer share window-local state through one app-global namespace
- restore semantics are deterministic and documented

### Phase 4: Decompose runtime internals

- separate runtime core, app framework, shell integration, host execution, and shell UI into clearer internal modules or crates
- keep reducer/effect semantics stable while moving registration and host execution concerns out of the central runtime files

Success criteria:

- `desktop_runtime` no longer acts as the default owner for every new feature
- the reducer core is readable and testable without shell UI or concrete host execution details

### Phase 5: Strengthen governance enforcement

- expand `cargo xtask docs storage-boundary` to detect concrete host-adapter bypasses in apps/runtime code
- optionally add a separate validator for forbidden `platform_host_web` imports in app crates
- align docs with actual enforced rules

Success criteria:

- architecture checks fail on direct adapter bypasses that contradict the documented boundary

### Phase 6: Incremental scalability improvements

- replace whole-library wallpaper reload patterns with narrower update flows where practical
- introduce explicit async workflow ownership where signal/effect sequencing is currently implicit
- prepare app registration for optional or package-loaded modules

Success criteria:

- host mutation paths carry smaller payloads
- async behavior is easier to trace and reason about

## Risks and Migration Constraints

The following constraints materially limit refactor shape:

- browser/Tauri dual-host behavior must continue working during migration
- current stored namespaces and envelope semantics are compatibility-sensitive
- `DesktopSnapshot` and app-state migration hooks already encode legacy behaviors that cannot be removed casually
- some direct store access in apps may exist because `AppServices` is currently incomplete, so removal requires replacement capabilities first
- docs and validators must be updated together or the governance layer will continue to misrepresent the architecture

Highest-risk migration areas:

- wallpaper type relocation, because those types currently straddle runtime, app, and host layers
- persistence ownership normalization for multi-instance apps
- runtime decomposition, because reducer/effect behavior is the architectural center of the system

## Appendix: Command Outputs and File Evidence

### Commands run

```text
cargo metadata --no-deps --format-version 1
cargo check --workspace
cargo xtask docs storage-boundary
```

Observed results:

- `cargo check --workspace` passed on 2026-02-28.
- `cargo xtask docs storage-boundary` passed on 2026-02-28.

### Dependency and structure evidence

- `platform_host` manifest: `crates/platform_host/Cargo.toml`
- runtime manifest: `crates/desktop_runtime/Cargo.toml`
- host adapter manifest: `crates/platform_host_web/Cargo.toml`
- app manifests: `crates/apps/*/app.manifest.toml`

### Core runtime files inspected

- `crates/desktop_runtime/src/components.rs`
- `crates/desktop_runtime/src/reducer.rs`
- `crates/desktop_runtime/src/model.rs`
- `crates/desktop_runtime/src/apps.rs`
- `crates/desktop_runtime/src/host.rs`
- `crates/desktop_runtime/src/host/boot.rs`
- `crates/desktop_runtime/src/host/persistence_effects.rs`
- `crates/desktop_runtime/src/host/wallpaper_effects.rs`
- `crates/desktop_runtime/src/shell.rs`
- `crates/desktop_runtime/src/shell/policy.rs`

### Contract and host files inspected

- `crates/desktop_app_contract/src/lib.rs`
- `crates/platform_host/src/lib.rs`
- `crates/platform_host/src/storage/app_state.rs`
- `crates/platform_host/src/storage/prefs.rs`
- `crates/platform_host/src/fs/service.rs`
- `crates/platform_host/src/wallpaper.rs`
- `crates/platform_host_web/src/lib.rs`
- `crates/platform_host_web/src/adapters.rs`
- `crates/platform_host_web/src/wallpaper.rs`
- `crates/desktop_tauri/src/lib.rs`
- `crates/desktop_tauri/src/app_state.rs`
- `crates/desktop_tauri/src/explorer.rs`

### App files inspected

- `crates/apps/explorer/src/lib.rs`
- `crates/apps/terminal/src/lib.rs`
- `crates/apps/notepad/src/lib.rs`
- `crates/apps/calculator/src/lib.rs`
- `crates/apps/settings/src/lib.rs`

### Governance/docs files inspected

- `xtask/src/docs.rs`
- `docs/reference/unified-window-manager-and-app-contract.md`
- `wiki/Explanation-System-Architecture-Overview.md`
- `wiki/Explanation-Browser-Host-Boundary-and-Storage-Model.md`
- `wiki/Reference-System-Architecture-Map.md`
- `wiki/How-to-Investigate-Persistence-and-Explorer-State.md`
