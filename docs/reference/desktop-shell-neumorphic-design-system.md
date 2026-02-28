---
title: "Desktop Shell Neumorphic Design System"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-28"
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

This reference documents the `soft-neumorphic` shell skin and the supporting token model used to apply restrained neumorphic depth across shell chrome and built-in app interiors without changing runtime behavior.

## Scope

The neumorphic skin covers:

- desktop wallpaper atmosphere and launcher icons
- window chrome, titlebar controls, and window bodies
- taskbar, start button, taskbar app buttons, tray, overflow, and menus
- built-in app primitives rendered through `system_ui` shell/content primitives and `data-ui-*` roots
- built-in app interiors for Explorer, Notepad, Terminal, Calculator, System Settings, and the lightweight Paint/Connection utility surfaces that replaced placeholder-grade windows

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

## Visual Rules

- Light source direction is fixed: top-left highlight and bottom-right shadow.
- Raised controls use low-amplitude outer shadows rather than dramatic extrusion.
- Pressed, toggled, and input-well states use inset treatments, not color changes alone.
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

## Related Files

- [`crates/desktop_runtime/src/model.rs`](../../crates/desktop_runtime/src/model.rs)
- [`crates/desktop_runtime/src/reducer/appearance.rs`](../../crates/desktop_runtime/src/reducer/appearance.rs)
- [`crates/site/src/theme_shell/01-primitives.css`](../../crates/site/src/theme_shell/01-primitives.css)
- [`crates/site/src/theme_shell/34-theme-soft-neumorphic.css`](../../crates/site/src/theme_shell/34-theme-soft-neumorphic.css)
