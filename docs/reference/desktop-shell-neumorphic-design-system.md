---
title: "Desktop Shell Neumorphic Design System"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-03-01"
audience: ["engineering", "design"]
invariants:
  - "Shell icon usage flows through the centralized `system_ui` icon abstraction instead of ad-hoc text glyphs or inline per-component SVG markup."
  - "Theme-specific visual changes are applied through `--sys-*` token remapping under `data-skin` scopes with shared shell structure to preserve behavior parity across skins."
  - "Accessibility behaviors (focus visibility, keyboard navigation, high contrast, reduced motion) remain functional during visual refinements."
tags: ["reference", "design-system", "desktop-shell", "neumorphic", "icons", "accessibility"]
domain: "frontend"
lifecycle: "ga"
---

# Desktop Shell Neumorphic Design System

This reference documents the rebuilt `soft-neumorphic` shell skin and the supporting token model used to apply a production neumorphic surface language across shell chrome, shared primitives, and the built-in UI showcase app without changing runtime behavior.

## Scope

The neumorphic skin covers:

- desktop wallpaper atmosphere and launcher icons
- window chrome, titlebar controls, and window bodies
- taskbar, start button, taskbar app buttons, tray, overflow, and menus
- built-in app primitives rendered through `system_ui` shell/content primitives and `data-ui-*` roots
- built-in app interiors for Explorer, Notepad, Terminal, Calculator, System Settings, the built-in UI Showcase app, and the lightweight Paint/Connection utility surfaces that replaced placeholder-grade windows

Canonical automation-backed validation surfaces for this skin are:

- `shell.soft-neumorphic.default`
- `shell.soft-neumorphic.context-menu-open`
- `shell.soft-neumorphic.start-button-hover`
- `shell.soft-neumorphic.start-button-focus`
- `shell.soft-neumorphic.high-contrast`
- `shell.soft-neumorphic.reduced-motion`
- `settings.desktop.appearance-tab`
- `settings.desktop.accessibility-tab`
- `system.ui-showcase.controls`
- `terminal.desktop.default`

Wallpaper asset selection remains a separate subsystem. Skin files may style atmosphere and surface treatment, but they must not choose wallpapers or mutate wallpaper runtime state.

## Runtime Contract

The skin is exposed through [`DesktopSkin`](../../crates/desktop_runtime/src/model.rs) as:

- stable id: `soft-neumorphic`
- label: `Soft Neumorphic`
- default typed skin for fresh state and legacy theme payloads that omitted `skin`

The shell root renders the active value through `data-skin`, while high-contrast and reduced-motion remain driven by `data-high-contrast` and `data-reduced-motion`.

## Token Model

The neumorphic skin is implemented in:

- [`crates/site/src/theme_shell/00-foundations.css`](../../crates/site/src/theme_shell/00-foundations.css)
- [`crates/site/src/theme_shell/01-primitives.css`](../../crates/site/src/theme_shell/01-primitives.css)
- [`crates/site/src/theme_shell/34-theme-soft-neumorphic.css`](../../crates/site/src/theme_shell/34-theme-soft-neumorphic.css)

Primary token families:

- `--sys-color-*`
- `--sys-font-*`
- `--sys-type-*`
- `--sys-space-*`
- `--sys-size-*`
- `--sys-radius-*`
- `--sys-border-*`
- `--sys-shadow-*`
- `--sys-elevation-*`
- `--sys-motion-*`
- `--sys-focus-*`
- `--sys-state-*`
- `--sys-opacity-*`
- `--sys-z-*`
- `--sys-comp-*`
- `--sys-light-*`
- `--sys-depth-*`
- `--sys-shadow-geometry-*`
- `--sys-surface-depth-*`

The neumorphic theme file is token-only. It remaps the shared `--sys-*` surface, elevation, border, focus, and component-role tokens for `data-skin="soft-neumorphic"` without targeting app-specific DOM contracts.

Key soft-neumorphic defaults now standardized by token remapping:

- base surface family centered on `#e6e7ee`
- muted blue accent family centered on `#5b8ccf`
- dual-shadow raised geometry:
  - highlight `rgba(255,255,255,0.8)` at `-6px -6px 12px`
  - shadow `rgba(0,0,0,0.12)` at `6px 6px 12px`
- inset, overlay, and pressed states derived from the same semantic shadow aliases rather than per-component literals
- large shared radii:
  - controls `12px`
  - panels/cards `18px` to `24px`
  - pills `9999px`
- 8px-based spacing and motion tokens

The deterministic UI feedback workflow snapshots the following token values from the shell root on every canonical slice:

- `--sys-color-surface`
- `--sys-color-accent`
- `--sys-radius-control`
- `--sys-radius-panel`
- `--sys-space-2`
- `--sys-space-3`
- `--sys-space-4`
- `--sys-elevation-raised`
- `--sys-elevation-inset`
- `--sys-focus-ring`

New semantic tokens added for the expanded kit include:

- `--sys-color-placeholder`
- `--sys-color-track`
- `--sys-color-track-active`
- `--sys-color-thumb`
- `--sys-color-ring`
- `--sys-color-ring-active`
- `--sys-color-icon-button`
- `--sys-color-icon-button-active`
- `--sys-glow-accent-soft`
- `--sys-scale-hover`
- `--sys-scale-pressed`
- `--sys-comp-switch-*`
- `--sys-comp-icon-button-size`
- `--sys-comp-knob-*`
- `--sys-comp-progress-ring-*`

## Visual Rules

- Light source direction is fixed: top-left highlight and bottom-right shadow.
- Raised controls use low-amplitude outer shadows rather than dramatic extrusion.
- Pressed, toggled, and input-well states use inset treatments, not color changes alone.
- Hover feedback is limited to slight light intensification plus `1.01` scale rather than large motion or glow effects.
- Focus indication uses an explicit outline token and must remain visible independently of elevation styling.
- Primitive selectors consume token aliases such as `--sys-radius-control`, `--sys-space-panel`, `--sys-surface-depth-muted`, and `--sys-state-hover` instead of direct literal geometry or color recipes.
- High-contrast mode may intentionally flatten surfaces to preserve separation and contrast.
- Dark mode uses dedicated neumorphic shadow/highlight recipes rather than simple light-theme inversion.
- Guided flows use the same depth grammar as direct-use tools: setup steps read as raised cards, advanced controls use restrained disclosure surfaces, and primary actions use the accent family sparingly.
- App and shell surfaces inherit those rules from shared primitives in `01-primitives.css`; the skin file does not restyle calculator/explorer/notepad/terminal/settings selectors directly.

## Component Conventions

- Reuse `system_ui::Icon`, `IconName`, and `IconSize` for shell iconography.
- Preserve shared shell structure and reducer-driven state semantics.
- Prefer semantic token remapping over one-off literals in theme files.
- Compose runtime shell chrome through `system_ui` primitives (`WindowFrame`, `WindowControlButton`, `TaskbarButton`, `ClockButton`, `MenuSurface`, `MenuItem`) rather than local raw button/menu markup.
- The expanded primitive kit now includes `IconButton`, `SegmentedControl`, `SegmentedControlOption`, `Switch`, `CircularProgress`, and `KnobDial`, all rendered through stable `data-ui-*` roots.
- `RangeField`, `SelectField`, `ProgressBar`, `TextField`, and `TextArea` now expose richer neumorphic hooks while preserving the shared contract.
- Keep app-specific classes nonvisual; styling must flow through shared primitives and `data-ui-*` contracts.
- Preserve terminal transcript readability over tactile styling.

## Accessibility and Validation

The neumorphic skin must preserve:

- keyboard navigation and dismissal behavior
- visible `:focus-visible` affordances
- reduced-motion suppression
- high-contrast accessibility mode
- minimum contrast goals of 4.5:1 for body text and 3:1 for UI boundaries/focus indicators

Validation and evidence requirements are governed by:

- [`docs/reference/desktop-shell-hig-neumorphic-conformance-checklist.md`](desktop-shell-hig-neumorphic-conformance-checklist.md)
- [`docs/sop/ui-design-conformance-review-sop.md`](../sop/ui-design-conformance-review-sop.md)

Current review-cycle evidence artifacts:

- [`/.artifacts/e2e/runs/`](../../.artifacts/e2e/runs/) now contains the canonical Playwright UI feedback runs, including deterministic screenshot, DOM, accessibility, layout, style, timing, log, trace, and diff artifacts for the shell slices under review.
- [`tools/e2e/baselines/`](../../tools/e2e/baselines/) stores the currently approved screenshot/DOM/accessibility/layout/style baselines promoted from accepted runs.
- [`/.artifacts/ui-conformance/contrast/soft-neumorphic-contrast-report.json`](../../.artifacts/ui-conformance/contrast/soft-neumorphic-contrast-report.json) records the current soft-neumorphic contrast sample set.

Current measured observations from that contrast artifact:

- Taskbar start-button focus outline: `5.2:1` against the taskbar background, meeting the `3:1` focus-indicator target.
- Terminal transcript sample text: `3.15:1` against the terminal surface, which preserves readability better than the ambient shell chrome but remains below the documented `4.5:1` body-text goal.
- Window-frame boundary sample: `2.24:1` against the desktop root, below the `3:1` non-text boundary target.
- Taskbar start-button label sample: `1.2:1` against its control background, below the documented text target and a concrete follow-up item for the skin.
- Browser `prefers-color-scheme: dark` emulation did not change the sampled computed shell colors during this review cycle, so dark-mode-specific contrast evidence remains incomplete.

Implementation note for the current review cycle:

- the shared neumorphic primitive kit and `system.ui-showcase` app are now implemented in code
- contrast evidence should be regenerated for the rebuilt skin before claiming all checklist items as complete
- canonical layout, navigation, interaction, accessibility, and representative app-view evidence now flows through `cargo e2e run`, `cargo e2e inspect`, and `cargo e2e promote` rather than ad hoc screenshot directories
- the style snapshot checks the computed shell primitives `[data-ui-kind="taskbar"]`, `[data-ui-kind="window-frame"]`, `[data-ui-kind="menu-surface"]`, and `[data-ui-kind="button"][data-ui-slot="start-button"]` for background, box-shadow, border-radius, outline, spacing, and transform drift

## Related Files

- [`crates/desktop_runtime/src/model.rs`](../../crates/desktop_runtime/src/model.rs)
- [`crates/desktop_runtime/src/reducer/appearance.rs`](../../crates/desktop_runtime/src/reducer/appearance.rs)
- [`crates/apps/ui_showcase/src/lib.rs`](../../crates/apps/ui_showcase/src/lib.rs)
- [`crates/site/src/theme_shell/01-primitives.css`](../../crates/site/src/theme_shell/01-primitives.css)
- [`crates/site/src/theme_shell/34-theme-soft-neumorphic.css`](../../crates/site/src/theme_shell/34-theme-soft-neumorphic.css)
