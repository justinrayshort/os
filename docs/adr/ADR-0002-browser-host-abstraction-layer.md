---
title: "ADR-0002 Unified Browser Host Abstraction Layer"
category: "adr"
owner: "architecture-owner"
status: "draft"
last_reviewed: "2026-02-28"
audience: ["engineering", "platform"]
invariants:
  - "Desktop reducer semantics remain independent of direct browser API calls."
  - "Browser capabilities are accessed through typed host services with runtime capability detection."
  - "Existing persisted app-state namespaces and schema versioning remain compatible during migration."
tags: ["adr", "architecture", "web", "platform", "wasm"]
domain: "architecture"
lifecycle: "draft"
---

# ADR-0002 Unified Browser Host Abstraction Layer

## Status

Proposed

## Current State Note

This ADR records the migration plan that introduced `platform_host` and `platform_host_web`.
The temporary `platform_storage` compatibility facade described below has since been removed after
the host-boundary migration completed. Historical discussion of that facade remains in this ADR as
implementation history and rationale, not as the current target-state architecture.

## Problem Statement

The application is a browser-hosted OS emulator and desktop shell implemented with Rust, Leptos, and
WASM. The codebase already separates deterministic desktop state transitions from some browser-specific
operations, but browser API usage is still distributed across UI components, runtime persistence code,
and storage bridges. This makes it difficult to:

- define a consistent systems boundary between OS-like semantics and browser-native capabilities
- add new browser capabilities (clipboard, notifications, devices, networking, background tasks)
  without repeating permission and fallback logic
- preserve long-term maintainability as the repository grows beyond storage and explorer use cases
- test core behavior independently from browser API behavior

The repository needs a unified abstraction layer that maps OS emulator responsibilities onto standardized
Web APIs while preserving browser sandbox constraints and progressive enhancement.

## Architectural Context

### Current Repository Structure (relevant parts)

The workspace currently contains:

- `crates/desktop_runtime`: desktop state model, reducer, runtime effects, and Leptos desktop shell UI
- `crates/platform_storage`: browser persistence/cache/explorer filesystem access wrappers and WASM bridge
- `crates/site`: Leptos site entry, routing, and deep-link parsing
- `crates/apps/*`: desktop applications (`calculator`, `explorer`, `notepad`, `terminal`)

### Current Architectural Strengths

- `crates/desktop_runtime` already has a reducer/effect pattern that separates state transitions from
  side effects.
- `crates/platform_storage` already defines versioned `AppStateEnvelope` persistence and namespaced
  storage for application state.
- Explorer already models a storage backend distinction between virtual IndexedDB-backed storage and
  native File System Access mounts.

### Current Architectural Gaps

- Browser API access is not centralized. Examples:
  - `desktop_runtime` reads viewport size and handles persistence-related effects in UI component code.
  - `desktop_runtime::persistence` directly uses `localStorage`.
  - app crates directly call `platform_storage` and duplicate hydrate/serialize/save patterns.
- `crates/platform_storage/src/wasm_bridge.rs` contains a large inline JavaScript bridge that mixes:
  - IndexedDB app-state storage
  - virtual filesystem logic
  - native File System Access integration
  - Cache API usage
- The current abstraction is storage-focused, not a general host capability model.

## Decision Drivers

- **Clear systems boundary** between emulator semantics and browser-native implementation details
- **Maintainability** through domain-based modules and smaller implementation units
- **Compatibility preservation** for existing persisted user data (IndexedDB/localStorage keys and
  namespaces)
- **Progressive enhancement** so the desktop remains functional on browsers with limited API support
- **Security correctness** by making permission prompts, secure-context requirements, and user
  activation constraints explicit
- **Performance** by allowing domain-specific implementations (workers, cache, async I/O) without
  changing application semantics
- **Testability** via pure core logic and replaceable host implementations (browser, no-op, in-memory)

## Web API Inventory and Host Boundary Mapping

Support status changes over time. This ADR records a support posture, not hardcoded browser
allowlists. Runtime capability detection is required.

### Inventory (by Web API)

| Web API | Primary Host Domain | Security / Permission Model | Performance Notes | Support Posture / Constraints |
| --- | --- | --- | --- | --- |
| IndexedDB | Durable app state, VFS metadata | Same-origin, async | Good for structured data and blobs; transaction overhead | Broadly available; baseline durable store |
| File System Access API | Native folder/file mounts | Secure context, user picker, permission prompts | Direct file access; async; native-like UX when supported | Limited/partial; must provide VFS fallback |
| Storage Manager | Quota and persistence policy | Secure context | Low-cost quota/persistence introspection | Broad enough to use opportunistically; methods vary |
| Cache API | File preview cache, response cache | Same-origin; secure context for SW scenarios | Efficient text/blob cache; HTTP response model | Broadly available; not a relational store |
| WebHID | HID device drivers | Secure context, explicit permission, policy-gated | Low-latency report I/O | Limited; capability-gated only |
| WebUSB | USB device drivers | Secure context, explicit permission | High throughput, protocol-specific handling | Limited; capability-gated only |
| WebBluetooth | BLE device access | Secure context, user activation for device request, permission prompt | BLE-limited throughput/latency | Limited; capability-gated only |
| Web Serial | Serial device access | Secure context, user activation, policy-gated | Stream-oriented I/O | Limited; capability-gated only |
| WebSocket | Client-server realtime channel | Network security via `wss`; origin constraints apply | Low latency; no built-in backpressure control | Broadly available |
| WebRTC | Peer data/media channels | Encrypted transport; signaling app-defined; media perms as needed | Strong for P2P realtime transport | Broadly available with feature variance |
| Service Workers | Background proxy, offline support | Secure context, lifecycle controlled by browser | Async; good for caching/offline and queued work | Broadly available |
| Web Workers | Background compute / process simulation | Same-origin worker loading | Main-thread offload; message passing overhead | Broadly available |
| SharedArrayBuffer | Shared memory IPC | Requires secure context and cross-origin isolation | Highest IPC throughput with Atomics | Available with strict isolation requirements |
| WebAssembly | Compute/runtime acceleration | Browser sandbox | Strong CPU performance; good Rust/WASM fit | Broadly available |
| Canvas API | 2D rendering surfaces | Standard canvas security model | Good 2D rendering path | Broadly available |
| WebGL | GPU rendering path | Sandboxed GPU API | Mature accelerated rendering | Broadly available; device capability varies |
| WebGPU | Advanced GPU rendering/compute (future) | Secure context | High performance; explicit resource control | Partial/limited; optional feature path |
| Web Audio | Audio subsystem / mixer | Browser autoplay and gesture constraints apply | Low-latency graph audio processing | Broadly available with behavior variance |
| Notifications | OS notification bridge | Secure context, permission prompt | Asynchronous user-visible alerts | Partial support/behavior varies |
| Clipboard API | Clipboard read/write | Secure context; activation and permission constraints vary | Fast for text; UX gated for reads | Broadly available for core text flows, constraints vary |
| Credential Management | Credential/identity broker | Secure context | Browser-managed identity/session UX | Broadly available entry point; subtype support varies |
| Permissions API | Capability query broker | Secure context; APIs vary in query support | Low overhead; improves UX and fallback decisions | Broadly available with API-specific gaps |
| Background Sync | Deferred background outbox | Secure context, service worker, browser-managed scheduling | Good for retry/outbox, not general compute | Limited; use best-effort fallback |

### OS Responsibility -> Browser Primitive Mapping (selected boundary)

| OS-like Responsibility | Host Service Abstraction | Primary Browser APIs | Notes |
| --- | --- | --- | --- |
| Persistent app state | `storage::AppStateStore` | IndexedDB, Storage Manager | Preserve versioned envelopes and namespaces |
| Preferences / boot hints | `storage::PrefsStore` | localStorage (small values), IndexedDB fallback | Minimize direct `localStorage` usage |
| Virtual file system semantics | `fs::VfsService`, `fs::MountService` | IndexedDB/OPFS/File System Access API | Async-only semantics; mount-specific permissions |
| File preview cache | `cache::ContentCache` | Cache API | Keep Explorer preview fallback behavior |
| Pointer/keyboard input | `input::InputSource` | DOM events | Browser remains physical input source |
| Device drivers (HID/USB/BLE/Serial) | `devices::*` | WebHID/WebUSB/WebBluetooth/Web Serial | Optional, capability-gated, permission-brokered |
| Networking / sockets | `net::NetService` | WebSocket, WebRTC, Fetch | Multiple channel types behind one API surface |
| Process simulation / background compute | `process::ProcessHost` | Web Workers, SharedArrayBuffer, Wasm | Cooperative scheduling; browser owns real threads |
| Background execution / offline tasks | `background::BackgroundService` | Service Worker, Background Sync | Best-effort fallback when unsupported |
| Graphics surfaces | `graphics::GraphicsHost` | Canvas, WebGL, WebGPU (optional) | Rendering backend choice remains an implementation detail |
| Audio output and mixer | `audio::AudioHost` | Web Audio | Explicit unlock/gesture state |
| Clipboard integration | `clipboard::ClipboardHost` | Clipboard API | User-activation constraints are first-class |
| Notifications | `notifications::NotificationHost` | Notifications, Service Worker notifications | Permission and platform variance exposed |
| Credentials / identity | `identity::IdentityHost` | Credential Management (+ WebAuthn/FedCM via browser) | Keep API surface generic, capability-based |
| Permission state | `permissions::PermissionBroker` | Permissions API + per-API requests | Centralize query/request/fallback logic |

## Considered Alternatives

### Alternative A: Keep the current pattern and extend `platform_storage` ad hoc

**Description**

Continue adding browser integrations directly to `platform_storage`, `desktop_runtime`, and app crates
as needed.

**Why not selected**

- Reinforces current duplication patterns (hydrate/save, permission checks, feature detection).
- Blurs boundaries between storage, UI concerns, and future capabilities.
- Makes long-term review and ownership harder as non-storage domains are added.

### Alternative B: Single monolithic browser bridge crate/file for all Web APIs

**Description**

Create one large crate (or file) that exposes many functions to the rest of the application, but without
domain separation.

**Why not selected**

- Repeats the main maintainability issue present in `wasm_bridge.rs`.
- Hides domain boundaries and makes testing/fallback behavior hard to reason about.
- Increases risk of regressions when unrelated browser domains change.

### Alternative C: Full application rewrite around a host command bus before migration

**Description**

Pause feature work and perform a broad rewrite to a new host runtime and application API all at once.

**Why not selected**

- High delivery risk and large migration window.
- Unnecessary because the existing code already has useful boundaries (reducer/effects, envelopes,
  explorer backend split) that can be evolved incrementally.

### Alternative D: Browser-specific APIs only through JavaScript modules, no Rust host traits

**Description**

Move all browser logic into JS modules and keep Rust as a thin caller.

**Why not selected**

- Weakens Rust-level type safety and discoverability for emulator semantics.
- Pushes architectural boundaries into JS implementation details instead of explicit crate contracts.

## Decision (Selected Approach)

Introduce a unified, capability-driven host abstraction composed of:

1. `crates/platform_host` (new): typed host interfaces, domain models, capability registry, and
   trait-based service contracts with no direct `web_sys` usage in the core API.
2. `crates/platform_host_web` (new): wasm32/browser implementation of `platform_host` using
   standardized Web APIs and domain-specific adapters.
3. `crates/platform_storage` (temporary compatibility facade): preserved public API during migration,
   internally delegating to `platform_host_web` so existing app code continues to work while imports are
   migrated.

The desktop reducer and desktop state model remain the source of truth for emulator semantics. Browser
capabilities become implementation details behind typed host services and runtime capability detection.

This ADR explicitly does **not** require a rendering rewrite. DOM/Leptos rendering remains the current
desktop renderer while the host boundary is standardized.

## Proposed Crate and Module Layout

### Workspace-Level Target Layout

```text
crates/
  apps/
    calculator/
    explorer/
    notepad/
    terminal/
  desktop_runtime/
  platform_host/          # NEW: host API/contracts/types (no direct browser calls in public API)
  platform_host_web/      # NEW: wasm32 browser implementation and Web API adapters
  platform_storage/       # TEMP: compatibility facade (deprecated after migration)
  site/
```

### `crates/platform_host` (new, API-first)

```text
crates/platform_host/src/
  lib.rs
  error.rs
  capabilities.rs         # feature detection model and support tiers
  command.rs              # optional host command types for async orchestration
  event.rs                # optional host event types for notifications/IPC
  time.rs                 # timestamps, clocks, monotonic helpers
  session.rs              # in-memory session abstractions
  storage/
    mod.rs
    app_state.rs          # AppStateEnvelope, namespace/schema contracts
    prefs.rs              # small preference storage abstraction
  cache/
    mod.rs
    content_cache.rs
  fs/
    mod.rs
    path.rs               # normalized virtual path semantics
    types.rs              # metadata, entries, backend/mount enums
    mounts.rs             # mount descriptors and backend status
    service.rs            # VfsService / MountService traits
  input/
    mod.rs
    events.rs             # keyboard/pointer/window events modeled for desktop runtime
  permissions/
    mod.rs
    broker.rs
  clipboard/
    mod.rs
    service.rs
  notifications/
    mod.rs
    service.rs
  identity/
    mod.rs
    service.rs
  net/
    mod.rs
    websocket.rs
    webrtc.rs
    service.rs
  process/
    mod.rs
    worker.rs
    scheduler.rs
  background/
    mod.rs
    service.rs
  audio/
    mod.rs
    service.rs
  graphics/
    mod.rs
    service.rs
  devices/
    mod.rs
    hid.rs
    usb.rs
    bluetooth.rs
    serial.rs
```

Notes:

- Not all modules need production implementations in phase 1.
- Stubs/no-op implementations are acceptable where the repository does not yet use the capability.

### `crates/platform_host_web` (new, browser implementation)

```text
crates/platform_host_web/src/
  lib.rs
  runtime.rs              # PlatformHostWeb entry point / service assembly
  detect.rs               # browser capability detection and support posture
  storage/
    mod.rs
    indexed_db.rs
    local_prefs.rs
    storage_manager.rs
  cache/
    mod.rs
    cache_api.rs
  fs/
    mod.rs
    virtual_fs_idb.rs     # current IndexedDB-backed VFS behavior
    native_fs_access.rs   # File System Access API mount adapter
    shared_types.rs
  permissions/
    mod.rs
    permissions_api.rs
  clipboard/
    mod.rs
    clipboard_api.rs
  notifications/
    mod.rs
    notifications_api.rs
  identity/
    mod.rs
    credentials_api.rs
  net/
    mod.rs
    websocket.rs
    webrtc.rs
  process/
    mod.rs
    workers.rs
    service_worker.rs
    background_sync.rs
  audio/
    mod.rs
    web_audio.rs
  graphics/
    mod.rs
    canvas.rs
    webgl.rs
    webgpu.rs             # optional, feature-gated
  devices/
    mod.rs
    webhid.rs
    webusb.rs
    web_bluetooth.rs
    web_serial.rs
```

Notes:

- Replace the current `wasm_bindgen(inline_js = "...")` monolith with domain-local adapters.
- Keep implementation details internal to `platform_host_web`; export typed `platform_host` services.

### `crates/platform_storage` (temporary compatibility facade)

`platform_storage` remains during migration to avoid a flag day. It should:

- re-export compatible data types where possible (or preserve serialized forms exactly)
- delegate existing functions to `platform_host_web`
- preserve current DB names, object store names, namespaces, and keys until the migration completes
- be marked as deprecated in code comments and follow-up docs once phase 2 begins

### `crates/desktop_runtime` (boundary cleanup, no semantic rewrite)

Proposed internal reorganization (incremental):

```text
crates/desktop_runtime/src/
  lib.rs
  model.rs
  reducer.rs
  apps.rs
  components.rs           # UI composition only (progressively reduced host logic)
  host/
    mod.rs
    context.rs            # injected host services/capabilities for runtime + apps
    boot.rs               # desktop boot snapshot orchestration
    effect_runner.rs      # runtime effect execution (moved out of components.rs)
    viewport.rs           # viewport/resize abstraction (browser implementation via host)
  persistence.rs          # compatibility wrapper during migration, then shrink/remove
```

## Migration Plan (Practical, Staged)

The migration is intentionally incremental. Each phase should preserve runtime behavior and stored data.

### Phase 0: ADR adoption and non-functional guardrails

**Goal**

Establish the decision and protect compatibility-sensitive assumptions before code movement.

**Changes**

- Add this ADR and link it in docs navigation.
- Add a short code comment in `platform_storage` and `desktop_runtime::persistence` noting upcoming
  host abstraction migration (optional but recommended).
- Define migration invariants (see compatibility section below) in team review criteria.

**Compatibility considerations**

- No code path changes.

**Risk mitigation**

- None required beyond review alignment.

### Phase 1: Extract host API contracts (`platform_host`) without behavior changes

**Goal**

Create a typed API crate that captures current semantics while leaving implementations unchanged.

**Sequencing**

1. Add `crates/platform_host` to the workspace.
2. Move or copy shared domain types into `platform_host`:
   - `AppStateEnvelope`
   - explorer/fs backend and metadata types
   - preference/session abstractions
3. Introduce trait interfaces for storage/cache/fs services.
4. Add no-op/in-memory implementations for non-wasm and tests.

**Refactor strategy**

- Prefer re-exports and type aliases first to reduce churn.
- Keep `platform_storage` as the implementation source during this phase.

**Compatibility considerations**

- Serialized formats and names must remain byte-compatible where already persisted.

**Risk mitigation**

- Add tests for envelope serialization compatibility and path normalization compatibility.

### Phase 2: Create browser implementation crate (`platform_host_web`) and split bridge by domain

**Goal**

Move Web API implementation details into a dedicated browser adapter crate while preserving existing
behavior.

**Sequencing**

1. Add `crates/platform_host_web`.
2. Split current `platform_storage/src/wasm_bridge.rs` responsibilities into domain modules:
   - storage (IndexedDB app state)
   - cache (Cache API)
   - fs virtual backend (IndexedDB VFS)
   - fs native backend (File System Access API)
3. Implement `platform_host` traits in `platform_host_web`.
4. Update `platform_storage` to delegate to `platform_host_web` while preserving its public API.

**Refactor strategy**

- Preserve function signatures in `platform_storage` first; internal delegation only.
- Keep DB name and object store names unchanged:
  - database: `retrodesk_os`
  - stores: `app_state`, `vfs_nodes`, `fs_config`

**Compatibility considerations**

- No data migration should be required if storage schema names are unchanged.
- Preserve current explorer namespace/path semantics and cached preview key format.

**Risk mitigation**

- Browser smoke test manual checklist for Explorer operations:
  - list/read/write/create/delete in virtual backend
  - connect native folder
  - permission request flow
  - cache preview fallback
- Keep old bridge code path behind an internal feature flag until parity is confirmed (optional).

### Phase 3: Move desktop runtime host side effects out of UI components

**Goal**

Make `desktop_runtime` consume host services through an explicit runtime host context.

**Sequencing**

1. Extract effect execution from `components.rs` into `desktop_runtime::host::effect_runner`.
2. Introduce `DesktopHostContext` (or similarly named injected service bundle).
3. Migrate `desktop_runtime::persistence` functions to call `platform_host` storage services.
4. Move viewport/window queries behind a host viewport interface.

**Refactor strategy**

- Keep `DesktopAction` and reducer behavior unchanged.
- Keep `RuntimeEffect` enum initially; map effects to host service calls in the effect runner.
- Avoid changing UI behavior and event wiring during this phase.

**Compatibility considerations**

- Preserve existing boot behavior:
  - legacy `localStorage` reads for migration compatibility
  - durable IndexedDB snapshot reads/writes

**Risk mitigation**

- Reuse existing reducer tests.
- Add integration tests (or scripted manual checks) for hydrate and persist flows.

### Phase 4: Migrate app crates from `platform_storage` calls to host services

**Goal**

Remove app-level duplication and make all app/browser integration pass through the host abstraction.

**Sequencing**

1. Migrate `calculator`, `notepad`, and `terminal` to shared app-state host helpers.
2. Migrate `explorer` to `platform_host` fs/cache/prefs/session services.
3. Extract repeated hydrate/serialize/save app logic into a shared helper module or support crate if
   needed.

**Refactor strategy**

- Migrate one app at a time (lowest risk first: `calculator` -> `terminal`/`notepad` -> `explorer`).
- Preserve UI behavior and persisted namespaces exactly during migration.

**Compatibility considerations**

- Keep existing namespaces:
  - `app.calculator`
  - `app.notepad`
  - `app.terminal`
  - `app.explorer`
  - `system.desktop`
- Keep current envelope version and schema migration guards.

**Risk mitigation**

- Add per-app hydration/persistence smoke tests.
- Use dual-read compatibility where practical (read old path, write new path only when identical).

### Phase 5: Expand host domains (permissions, clipboard, notifications, audio, net, devices)

**Goal**

Complete the unified abstraction beyond storage and explorer concerns without forcing immediate UI
adoption.

**Sequencing**

1. Implement `capabilities` and `permissions` domains with runtime detection.
2. Add clipboard, notifications, and audio services (high-value, common UX needs).
3. Add networking and background services (WebSocket/WebRTC/Service Worker/Background Sync).
4. Add device services (HID/USB/Bluetooth/Serial) as optional capability-gated modules.

**Refactor strategy**

- Introduce services and capability reporting first, then adopt in apps/runtime later.
- Use no-op implementations for unsupported APIs and non-wasm targets.

**Compatibility considerations**

- No persistence schema changes required.

**Risk mitigation**

- Gate user-visible features by capability and permission state.
- Centralize permission prompting to avoid inconsistent UX.

### Phase 6: Deprecate and remove `platform_storage` compatibility facade

**Goal**

Finish the migration and simplify the workspace around explicit boundaries.

**Sequencing**

1. Remove direct `platform_storage` imports from all crates.
2. Update workspace members and dependency graph (if retiring the crate).
3. Remove compatibility re-exports and deprecated wrappers.
4. Update docs and ADR follow-up status.

**Refactor strategy**

- Complete only after all dependents are migrated and tested.
- Prefer one final cleanup PR after phased migrations land.

**Compatibility considerations**

- Removal of `platform_storage` must not change persisted data shape or keys.

**Risk mitigation**

- Run full docs and workspace verification before removing the facade.
- Land cleanup separately from feature additions.

## Compatibility and Data Migration Invariants

The following are mandatory during phases 1-6 unless superseded by a later ADR:

- Preserve IndexedDB database name `retrodesk_os` during the migration.
- Preserve object store names `app_state`, `vfs_nodes`, and `fs_config`.
- Preserve persisted namespace strings (for example `system.desktop`, `app.notepad`).
- Preserve `AppStateEnvelope` semantics and versioning behavior.
- Preserve existing `localStorage` keys used for desktop theme/history compatibility until a dedicated
  migration step is implemented and validated.
- Preserve Explorer virtual path normalization and preview cache key format to avoid breaking cached
  previews and user expectations.

## Consequences

### Positive

- Clearer boundary between emulator semantics and browser implementations.
- Safer addition of new Web API integrations with consistent permission and capability handling.
- Improved maintainability through domain-based modules and smaller adapter units.
- Better testability via no-op/in-memory implementations and explicit host contracts.

### Negative / Costs

- Additional crates and modules increase short-term complexity.
- Multi-phase migration requires disciplined sequencing and compatibility testing.
- Some temporary duplication will exist while `platform_storage` acts as a compatibility facade.

### Operational Implications

- Documentation and code review should reference this ADR when introducing new browser API usage.
- New browser capability work should start in `platform_host` / `platform_host_web`, not in app/UI
  crates.

## Follow-Up Work (Non-binding checklist)

- Add a follow-up ADR (or update this ADR status) when the compatibility facade is removed.
- Document capability detection and fallback policy in `docs/reference/` once phase 5 begins.
- Add a contributor guide snippet covering "where browser APIs may be introduced" in code review.
