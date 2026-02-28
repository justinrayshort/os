---
title: "Desktop Shell Fluent Modern Design System"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-27"
audience: ["engineering", "design"]
invariants:
  - "Shell icon usage flows through the centralized `system_ui` icon abstraction instead of ad-hoc text glyphs or inline per-component SVG markup."
  - "Theme-specific visual changes are applied via `data-skin` scoped CSS overrides with shared shell structure to preserve behavior parity across skins."
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
- System Settings app (display and accessibility surfaces)

Wallpaper rendering is a separate shell subsystem, not a skin extension. Skin/theme layers own tokens and component styling; wallpaper rendering and wallpaper asset selection are supplied by the independent wallpaper renderer layer in `crates/site/src/theme_shell/05-wallpaper-renderer.css`.

App content areas (Calculator, Explorer, Notepad, Terminal, System Settings) now receive explicit modern-adaptive overrides for shared app chrome, control surfaces, and dark-mode parity while preserving existing app behavior.

Window manager and taskbar layout now use adaptive sizing heuristics tied to viewport constraints, with priority-based taskbar visibility (running windows, pinned strip, tray density, clock date) to reduce crowding across narrow and wide displays.

## Iconography Standard (Fluent UI System Icons)

Primary iconography is centralized in:

- [`crates/system_ui/src/icon.rs`](../../crates/system_ui/src/icon.rs)

The module provides:

- `IconName`: semantic icon identifiers (app icons, window controls, tray/status icons, launcher/overflow icons)
- `IconSize`: standardized shell icon sizes (`Xs`, `Sm`, `Md`, `Lg`)
- `Icon`: single SVG rendering component for shell and app usage
- `app_icon_name_by_id(ApplicationId)`: semantic app-to-icon mapping for desktop/taskbar/window reuse

Implementation notes:

- Icons use a curated subset of Fluent UI System Icons from `@fluentui/svg-icons` (regular 24px assets).
- Shell components do not embed text glyphs like `DIR`, `TXT`, `56K`, `X`, `_`, etc.
- The same semantic icon is reused across desktop launchers, taskbar buttons, menus, and window chrome for consistency.

## Theme and Token Model

The Fluent redesign is intentionally incremental and scoped by the shell root skin attribute:

- `data-skin="modern-adaptive"` on `.desktop-shell`

Primary theme and component tokens are defined/overridden in:

- [`crates/site/src/theme_shell/00-tokens-reset.css`](../../crates/site/src/theme_shell/00-tokens-reset.css)
- [`crates/site/src/theme_shell/01-components-shell.css`](../../crates/site/src/theme_shell/01-components-shell.css)
- [`crates/site/src/theme_shell/02-interactions-hover.css`](../../crates/site/src/theme_shell/02-interactions-hover.css)
- [`crates/site/src/theme_shell/03-responsive-base.css`](../../crates/site/src/theme_shell/03-responsive-base.css)
- [`crates/site/src/theme_shell/04-motion-base.css`](../../crates/site/src/theme_shell/04-motion-base.css)
- [`crates/site/src/theme_shell/05-wallpaper-renderer.css`](../../crates/site/src/theme_shell/05-wallpaper-renderer.css)
- [`crates/site/src/theme_shell/10-theme-xp-tokens.css`](../../crates/site/src/theme_shell/10-theme-xp-tokens.css)
- [`crates/site/src/theme_shell/11-theme-xp-overrides.css`](../../crates/site/src/theme_shell/11-theme-xp-overrides.css)
- [`crates/site/src/theme_shell/20-theme-legacy95-tokens.css`](../../crates/site/src/theme_shell/20-theme-legacy95-tokens.css)
- [`crates/site/src/theme_shell/21-theme-legacy95-overrides.css`](../../crates/site/src/theme_shell/21-theme-legacy95-overrides.css)
- [`crates/site/src/theme_shell/30-theme-fluent-modern-tokens-core.css`](../../crates/site/src/theme_shell/30-theme-fluent-modern-tokens-core.css)
- [`crates/site/src/theme_shell/31-theme-fluent-modern-primitives.css`](../../crates/site/src/theme_shell/31-theme-fluent-modern-primitives.css)
- [`crates/site/src/theme_shell/32-theme-fluent-modern-theme-tokens.css`](../../crates/site/src/theme_shell/32-theme-fluent-modern-theme-tokens.css)
- [`crates/site/src/theme_shell/33-theme-fluent-modern-overrides.css`](../../crates/site/src/theme_shell/33-theme-fluent-modern-overrides.css)

Token categories introduced:

- typography (`--fluent-font-family`, `--fluent-font-family-mono`)
- shell typography hierarchy tokens (`--fluent-shell-font-size-*`, `--fluent-shell-font-weight-*`, `--fluent-shell-line-height-*`) for body/chrome/title/caption text roles
- terminal content tokens (`--terminal-*`) for terminal surfaces, prompt hierarchy, semantic transcript states, overlay surfaces, selection/caret treatment, and monospace rhythm
- spacing scale (`--fluent-space-*`)
- radius scale (`--fluent-radius-*`)
- icon sizing (`--fluent-icon-*`)
- motion timing (`--fluent-motion-*`)
- shell surfaces, strokes, text, accent, focus, elevation, and component state/metrics tokens (including taskbar/start/menu/tray interactive states) (`--fluent-shell-*`)
- app-surface layout/control/touch ergonomics tokens (`--fluent-app-*`) used by all built-in app shells

Compatibility strategy:

- Existing `--ui-*` tokens remain available for legacy components and app surfaces.
- Fluent Modern maps/overrides selected `--ui-*` values while adding higher-level shell tokens.
- Most redesign rules are appended as theme-scoped CSS overrides rather than replacing legacy rules.

Wallpaper boundary rules:

- skin files may provide atmosphere overlays, but they do not choose wallpaper assets
- wallpaper selection, display mode, and animation policy are driven by runtime wallpaper state
- theme changes must not implicitly change wallpaper selection

## Component Primitives and Styling Conventions

The redesign standardizes these shell primitives through `crates/system_ui` and `data-ui-*` roots:

- `Icon` for SVG icon rendering
- icon wrapper containers (`.taskbar-app-icon`, `.taskbar-glyph`, `.tray-widget-glyph`, `.titlebar-app-icon`)
- reducer-backed taskbar tray accessibility toggles (high contrast and reduced motion)
- consistent shell surfaces for windows, taskbar, menus, and dialogs
- shared corner radii and elevation tokens
- focus-visible styles using a single theme focus token

Shared app-surface primitives (built-in apps + placeholder apps):

- `AppShell` / `data-ui-kind="app-shell"`: canonical app layout container
- `MenuBar`, `ToolBar`, `StatusBar`: shared app chrome rows
- `Button`: canonical interactive target primitive
- `TextField`, `SelectField`, `RangeField`, `ColorField`: canonical input primitives
- `TextArea`: canonical multiline editor primitive
- `ProgressBar`: canonical progress indicator primitive

Terminal-specific primitives:

- `terminal-screen`: scrollable transcript viewport with terminal typography tokens
- `terminal-transcript`: transcript stack wrapper for semantic line grouping
- `terminal-line-*`: semantic transcript classes for prompt/stdout/stderr/status/json/system rendering
- `terminal-composer-shell`: minimal prompt/input surface with anchored completion overlay
- `terminal-composer`, `terminal-prompt`, `terminal-prompt-cwd`, `terminal-prompt-separator`: structured prompt hierarchy
- `terminal-completions`, `terminal-completion`: compact completion overlay contract

Adaptive shell layout primitives:

- viewport-aware default window geometry generation (`default_open_request(..., viewport)`) with per-app min/max ratios
- reducer-side window open clamping to viewport bounds (readability-preserving min dimensions plus max-size caps)
- taskbar runtime layout planning (`compute_taskbar_layout`) with priority-based visibility and overflow budgeting

Built-in app conformance rule:

- Explorer, Notepad, Terminal, Calculator, System Settings, Paint placeholder, and Dial-up placeholder must compose from the primitives above and avoid one-off control styling when an equivalent primitive exists.
- App-specific classes may extend surface semantics (for example `explorer-*`, `calc-*`) but must inherit interaction metrics (target size, padding, state transitions) from `app-action` and `app-field`.
- Terminal remains transcript-first: no persistent toolbar, run button, or status chrome; one minimal startup hint; command-driven utility actions; and auto-follow output that disengages while the user reviews older transcript lines.

Conventions:

- Icons are decorative unless otherwise specified (`aria-hidden="true"` by default).
- Labels remain text-based for discoverability and assistive tech compatibility.
- Icon color is inherited from component foreground color (`currentColor`) to preserve state styling.
- Layout and component metrics should use spacing/radius/component metric tokens; remaining raw `px` values in Fluent overrides are limited to effect geometry (for example hairline borders, shadows, outline widths/offsets, transform nudges, and decorative gradient dimensions) unless explicitly documented otherwise.
- App interaction states are standardized through shared `system_ui` primitives and `data-ui-*` states instead of per-app ad hoc state rules.
- Touch/hybrid ergonomics are tokenized via `--fluent-app-touch-*`; coarse-pointer contexts elevate controls to touch target minimums and increase control spacing/gaps.

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

Fluent Modern supports adaptive theming through `prefers-color-scheme` token overrides while preserving the explicit runtime skin preset (`data-skin`).

Performance safeguards:

- redesign is implemented as CSS overrides on existing DOM structure (no shell layout algorithm changes)
- expensive effects are limited to key surfaces (taskbar/menus/dialogs) and guarded with `@supports (backdrop-filter: ...)`
- reduced-motion mode disables transitions/animations at the shell scope

## Incremental Rollout Guidance

When extending the redesign to more shell or app surfaces:

1. Reuse `IconName` and `Icon` instead of introducing new icon markup per component.
2. Prefer token additions (`--fluent-shell-*`) over one-off color literals in component rules.
3. Scope visual changes under `.desktop-shell[data-skin="modern-adaptive"]` unless the change is intended for all skins.
4. Preserve keyboard, focus, and screen-reader semantics before adjusting appearance.
5. Validate on standard and high-DPI displays (especially icon alignment and small text in tray/taskbar regions).

## Related Files

- [`crates/desktop_runtime/src/components.rs`](../../crates/desktop_runtime/src/components.rs)
- [`crates/desktop_runtime/src/model.rs`](../../crates/desktop_runtime/src/model.rs)
- [`crates/desktop_runtime/src/reducer.rs`](../../crates/desktop_runtime/src/reducer.rs)
- [`crates/site/src/theme_shell/01-components-shell.css`](../../crates/site/src/theme_shell/01-components-shell.css)
- [`crates/site/src/theme_shell/33-theme-fluent-modern-overrides.css`](../../crates/site/src/theme_shell/33-theme-fluent-modern-overrides.css)
- [`docs/reference/desktop-shell-hig-fluent-conformance-checklist.md`](desktop-shell-hig-fluent-conformance-checklist.md)
- [`docs/sop/ui-design-conformance-review-sop.md`](../sop/ui-design-conformance-review-sop.md)
- [`wiki/Reference-Design-Materials-and-Artifacts.md`](../../wiki/Reference-Design-Materials-and-Artifacts.md)
