---
title: "Desktop Shell Neumorphic Design System"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-28"
audience: ["engineering", "design"]
invariants:
  - "Shell icon usage flows through the centralized `system_ui` icon abstraction instead of ad-hoc text glyphs or inline per-component SVG markup."
  - "Theme-specific visual changes are applied via `data-skin` scoped CSS overrides with shared shell structure to preserve behavior parity across skins."
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
- built-in app primitives rendered through `system_ui` (`AppShell`, `MenuBar`, `ToolBar`, `StatusBar`, `Button`, `TextField`, `TextArea`, `ProgressBar`) and `data-ui-*` roots
- built-in app interiors for Explorer, Notepad, Terminal, Calculator, System Settings, and placeholder apps

Wallpaper asset selection remains a separate subsystem. Skin files may style atmosphere and surface treatment, but they must not choose wallpapers or mutate wallpaper runtime state.

## Runtime Contract

The skin is exposed through [`DesktopSkin`](../../crates/desktop_runtime/src/model.rs) as:

- stable id: `soft-neumorphic`
- label: `Soft Neumorphic`
- default typed skin for fresh state and legacy theme payloads that omitted `skin`

The shell root renders the active value through `data-skin`, while high-contrast and reduced-motion remain driven by `data-high-contrast` and `data-reduced-motion`.

## Token Model

The neumorphic skin is implemented in:

- [`crates/site/src/theme_shell/34-theme-soft-neumorphic-tokens.css`](../../crates/site/src/theme_shell/34-theme-soft-neumorphic-tokens.css)
- [`crates/site/src/theme_shell/35-theme-soft-neumorphic-overrides.css`](../../crates/site/src/theme_shell/35-theme-soft-neumorphic-overrides.css)

Primary token families:

- `--neuro-space-*`
- `--neuro-radius-*`
- `--neuro-motion-*`
- `--neuro-surface-*`
- `--neuro-text-*`
- `--neuro-border-*`
- `--neuro-highlight-*`
- `--neuro-shadow-*`
- `--neuro-elevation-raised-*`
- `--neuro-elevation-pressed-*`
- `--neuro-elevation-inset-*`
- `--neuro-focus-*`
- `--neuro-app-*`

Terminal readability continues to use the shared `--terminal-*` token family, with the neumorphic skin remapping those tokens to match the shell while preserving transcript clarity.

## Visual Rules

- Light source direction is fixed: top-left highlight and bottom-right shadow.
- Raised controls use low-amplitude outer shadows rather than dramatic extrusion.
- Pressed, toggled, and input-well states use inset treatments, not color changes alone.
- Focus indication uses an explicit outline token and must remain visible independently of elevation styling.
- High-contrast mode may intentionally flatten surfaces to preserve separation and contrast.
- Dark mode uses dedicated neumorphic shadow/highlight recipes rather than simple light-theme inversion.

## Component Conventions

- Reuse `system_ui::Icon`, `IconName`, and `IconSize` for shell iconography.
- Preserve shared shell structure and reducer-driven state semantics.
- Prefer semantic token updates over one-off literals in overrides.
- Keep app-specific classes compatible with shared primitives instead of reintroducing bespoke controls.
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
- [`crates/site/src/theme_shell/01-components-shell.css`](../../crates/site/src/theme_shell/01-components-shell.css)
- [`crates/site/src/theme_shell/34-theme-soft-neumorphic-tokens.css`](../../crates/site/src/theme_shell/34-theme-soft-neumorphic-tokens.css)
- [`crates/site/src/theme_shell/35-theme-soft-neumorphic-overrides.css`](../../crates/site/src/theme_shell/35-theme-soft-neumorphic-overrides.css)
