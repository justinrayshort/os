---
title: "Desktop Shell Fluent Modern Design System"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-26"
audience: ["engineering", "design"]
invariants:
  - "Shell icon usage flows through the centralized `desktop_runtime::icons` abstraction instead of ad-hoc text glyphs or inline per-component SVG markup."
  - "Theme-specific visual changes are applied via `data-theme` scoped CSS overrides to preserve incremental rollout and backward compatibility for legacy themes."
  - "Accessibility behaviors (focus visibility, keyboard navigation, high contrast, reduced motion) remain functional during visual refinements."
tags: ["reference", "design-system", "desktop-shell", "fluent", "icons", "accessibility"]
domain: "frontend"
lifecycle: "ga"
---

# Desktop Shell Fluent Modern Design System

This reference documents the tokenized shell redesign that modernizes the desktop interface with Fluent 2-inspired visuals while preserving existing runtime behavior, layout logic, and keyboard interaction semantics.

## Scope

The Fluent Modern redesign currently covers the desktop shell chrome and shared shell controls:

- desktop launcher icons
- window chrome (titlebars and window controls)
- taskbar (start button, pinned apps, running windows, overflow)
- tray widgets and clock
- launcher/start menu and taskbar menus
- desktop context menu wallpaper selection affordances
- display properties dialog shell controls

App content areas (Calculator, Explorer, Notepad, Terminal) continue to use existing layouts and inherit shell typography/tokens where compatible.

## Iconography Standard (Fluent UI System Icons)

Primary iconography is centralized in:

- [`crates/desktop_runtime/src/icons.rs`](../../crates/desktop_runtime/src/icons.rs)

The module provides:

- `IconName`: semantic icon identifiers (app icons, window controls, tray/status icons, launcher/overflow icons)
- `IconSize`: standardized shell icon sizes (`Xs`, `Sm`, `Md`, `Lg`)
- `FluentIcon`: single SVG rendering component for shell usage
- `app_icon_name(AppId)`: semantic app-to-icon mapping for desktop/taskbar/window reuse

Implementation notes:

- Icons use a curated subset of Fluent UI System Icons from `@fluentui/svg-icons` (regular 24px assets).
- Shell components do not embed text glyphs like `DIR`, `TXT`, `56K`, `X`, `_`, etc.
- The same semantic icon is reused across desktop launchers, taskbar buttons, menus, and window chrome for consistency.

## Theme and Token Model

The Fluent redesign is intentionally incremental and scoped by the shell root attribute:

- `data-theme="Fluent Modern"` on `.desktop-shell`

Primary theme and component tokens are defined/overridden in:

- [`crates/site/src/theme_shell.css`](../../crates/site/src/theme_shell.css)

Token categories introduced:

- typography (`--fluent-font-family`, `--fluent-font-family-mono`)
- shell typography hierarchy tokens (`--fluent-shell-font-size-*`, `--fluent-shell-font-weight-*`, `--fluent-shell-line-height-*`) for body/chrome/title/caption text roles
- spacing scale (`--fluent-space-*`)
- radius scale (`--fluent-radius-*`)
- icon sizing (`--fluent-icon-*`)
- motion timing (`--fluent-motion-*`)
- shell surfaces, strokes, text, accent, focus, elevation, and component state/metrics tokens (including taskbar/start/menu/tray interactive states) (`--fluent-shell-*`)

Compatibility strategy:

- Existing `--ui-*` tokens remain available for legacy components and app surfaces.
- Fluent Modern maps/overrides selected `--ui-*` values while adding higher-level shell tokens.
- Most redesign rules are appended as theme-scoped CSS overrides rather than replacing legacy rules.

## Component Primitives and Styling Conventions

The redesign standardizes these shell primitives:

- `FluentIcon` for SVG icon rendering
- icon wrapper containers (`.taskbar-app-icon`, `.taskbar-glyph`, `.tray-widget-glyph`, `.titlebar-app-icon`)
- reducer-backed taskbar tray accessibility toggles (high contrast and reduced motion)
- consistent shell surfaces for windows, taskbar, menus, and dialogs
- shared corner radii and elevation tokens
- focus-visible styles using a single theme focus token

Conventions:

- Icons are decorative unless otherwise specified (`aria-hidden="true"` by default).
- Labels remain text-based for discoverability and assistive tech compatibility.
- Icon color is inherited from component foreground color (`currentColor`) to preserve state styling.

## Accessibility and Usability Requirements

The visual refresh must preserve shell usability:

- keyboard navigation remains unchanged (taskbar shortcuts, menu navigation, context menus, window activation)
- focus indicators remain visible via `:focus-visible` theme overrides
- reduced motion is supported via a user-facing taskbar tray toggle, runtime theme state (`data-reduced-motion="true"`), and CSS transition suppression
- high contrast is supported via a user-facing taskbar tray toggle and runtime theme state (`data-high-contrast="true"`) token overrides
- text labels are not replaced by icon-only controls where labels carry meaning (taskbar/start/menu entries)

Contrast guidance for future refinements:

- maintain a minimum 4.5:1 contrast ratio for body text and iconography used as primary status indicators
- maintain a minimum 3:1 contrast ratio for non-text UI boundaries and focus indicators
- re-verify both light and dark adaptive variants when adjusting accent or surface tokens

## Adaptive Theming and Performance Notes

Fluent Modern supports adaptive theming through `prefers-color-scheme` token overrides while preserving the explicit runtime theme preset (`data-theme`).

Performance safeguards:

- redesign is implemented as CSS overrides on existing DOM structure (no shell layout algorithm changes)
- expensive effects are limited to key surfaces (taskbar/menus/dialogs) and guarded with `@supports (backdrop-filter: ...)`
- reduced-motion mode disables transitions/animations at the shell scope

## Incremental Rollout Guidance

When extending the redesign to more shell or app surfaces:

1. Reuse `IconName` and `FluentIcon` instead of introducing new icon markup per component.
2. Prefer token additions (`--fluent-shell-*`) over one-off color literals in component rules.
3. Scope visual changes under `.desktop-shell[data-theme="Fluent Modern"]` unless the change is intended for all themes.
4. Preserve keyboard, focus, and screen-reader semantics before adjusting appearance.
5. Validate on standard and high-DPI displays (especially icon alignment and small text in tray/taskbar regions).

## Related Files

- [`crates/desktop_runtime/src/components.rs`](../../crates/desktop_runtime/src/components.rs)
- [`crates/desktop_runtime/src/model.rs`](../../crates/desktop_runtime/src/model.rs)
- [`crates/desktop_runtime/src/reducer.rs`](../../crates/desktop_runtime/src/reducer.rs)
- [`crates/site/src/theme_shell.css`](../../crates/site/src/theme_shell.css)
- [`docs/reference/desktop-shell-hig-fluent-conformance-checklist.md`](desktop-shell-hig-fluent-conformance-checklist.md)
- [`docs/sop/ui-design-conformance-review-sop.md`](../sop/ui-design-conformance-review-sop.md)
- [`wiki/Reference-Design-Materials-and-Artifacts.md`](../../wiki/Reference-Design-Materials-and-Artifacts.md)
