---
title: "Desktop Wallpaper Subsystem Contract"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-27"
audience: ["engineering", "design"]
invariants:
  - "Wallpaper configuration is persisted independently from skin/theme preferences."
  - "Wallpaper rendering behavior is deterministic in CSS pixels across screen resolutions and device pixel ratios."
  - "Theme services may not select wallpaper assets, and wallpaper services may not mutate theme tokens or skin selection."
tags: ["reference", "desktop", "wallpaper", "runtime", "host-boundary"]
domain: "frontend"
lifecycle: "ga"
---

# Desktop Wallpaper Subsystem Contract

This reference defines the independent wallpaper subsystem used by the desktop shell and the System Settings app.

## Ownership Boundary

- `desktop_runtime` owns wallpaper state, preview state, built-in catalog resolution, reducer actions, and shell rendering.
- `platform_host` defines the wallpaper asset-management boundary for import, metadata updates, library listing, deletion, and source resolution.
- `platform_host_web` and future desktop-host implementations provide concrete storage/import behavior.
- app crates consume wallpaper features only through injected `desktop_app_contract::WallpaperService`.

Theme and wallpaper are separate domains:

- `DesktopTheme` owns skin selection, high contrast, reduced motion, and related visual-policy state.
- `WallpaperConfig` owns asset selection, display mode, anchor position, and animation policy.

## Contract Surface

The wallpaper domain model now lives in [`/Users/justinshort/os/crates/platform_host/src/wallpaper.rs`](/Users/justinshort/os/crates/platform_host/src/wallpaper.rs).
The app-facing wallpaper service surface continues to live in [`/Users/justinshort/os/crates/desktop_app_contract/src/lib.rs`](/Users/justinshort/os/crates/desktop_app_contract/src/lib.rs).

Primary types:

- `WallpaperSelection`
- `WallpaperDisplayMode`
- `WallpaperPosition`
- `WallpaperAnimationPolicy`
- `WallpaperConfig`
- `WallpaperAssetRecord`
- `WallpaperCollection`
- `WallpaperLibrarySnapshot`
- `ResolvedWallpaperSource`
- `WallpaperImportRequest`

Primary service:

- `WallpaperService`

`WallpaperService` exposes:

- committed wallpaper state (`current`)
- optional preview state (`preview`)
- wallpaper library metadata (`library`)
- preview/apply/revert commands
- import, rename, tag, favorite, collection, and delete operations

Runtime update semantics:

- metadata edits and collection create/rename operations upsert the returned record directly into
  the merged runtime wallpaper library
- import, asset delete, and collection delete operations now return typed mutation results so the
  runtime can update library state without a follow-up `list_library()` round-trip
- full `WallpaperLibrarySnapshot` loads remain the bootstrap/listing contract for initial hydration
  and explicit library refresh

## Rendering Rules

Supported display modes:

- `Fill`
- `Fit`
- `Stretch`
- `Tile`
- `Center`

Determinism rules:

- placement is computed from CSS-pixel viewport dimensions
- device pixel ratio changes sampling quality only
- device pixel ratio does not change mode selection, anchor choice, or tile origin

Animated media rules:

- `LoopMuted` renders looping muted playback
- `None` renders a poster or static frame
- `Tile` is invalid for animated-image and video assets
- reduced motion forces static/poster rendering without mutating persisted wallpaper configuration

Failure behavior:

- unresolved selections fall back to built-in `cloud-bands`
- fallback does not mutate theme state

## Persistence Domains

Independent preference keys:

- `system.desktop_theme.v2`
- `system.desktop_wallpaper.v1`
- `system.wallpaper_library.v1`

Migration support continues to read legacy wallpaper data from:

- `retrodesk.theme.v1`
- snapshot schema `1` payloads that embedded `theme.wallpaper_id`

New layout snapshots no longer persist theme or wallpaper state.

## Built-In Catalog

Built-in wallpaper metadata is generated from:

- [`/Users/justinshort/os/assets/wallpapers/catalog.toml`](/Users/justinshort/os/assets/wallpapers/catalog.toml)

Build-time validation in [`/Users/justinshort/os/crates/desktop_runtime/build.rs`](/Users/justinshort/os/crates/desktop_runtime/build.rs) enforces:

- exact schema version
- unique wallpaper IDs
- existing referenced assets
- media-kind and poster-path consistency
- valid featured entries

## Related References

- [/Users/justinshort/os/docs/reference/desktop-shell-neumorphic-design-system.md](/Users/justinshort/os/docs/reference/desktop-shell-neumorphic-design-system.md)
- [/Users/justinshort/os/docs/reference/desktop-shell-hig-neumorphic-conformance-checklist.md](/Users/justinshort/os/docs/reference/desktop-shell-hig-neumorphic-conformance-checklist.md)
- [GitHub Wiki: Explanation - System Architecture Overview](https://github.com/justinrayshort/os/wiki/Explanation-System-Architecture-Overview)
