---
title: "System UI Component Library"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-28"
audience: ["engineering", "design"]
invariants:
  - "Shell and built-in system applications consume shared `system_ui` primitives instead of redefining legacy `.app-*` visual contracts in local markup."
  - "Shared primitives render stable `data-ui-*` roots and consume semantic `--sys-*` tokens rather than skin-specific literals."
  - "UI conformance validation rejects legacy primitive class usage and old icon import paths outside approved primitive surfaces."
tags: ["reference", "design-system", "system-ui", "components", "tokens"]
domain: "frontend"
lifecycle: "ga"
---

# System UI Component Library

This reference defines the shared `crates/system_ui` component library used by the desktop shell and built-in system applications.

## Scope

The library owns:

- shared Leptos primitives for app shells, surfaces, panels, buttons, fields, progress, typography, icons, and layout
- centralized iconography (`Icon`, `IconName`, `IconSize`)
- stable `data-ui-*` DOM markers and `ui-*` compatibility classes
- semantic `--sys-*` token consumption by primitive styles

The library does not own app logic, host contracts, reducer state, or runtime orchestration.

## Primitive Catalog

Structural primitives:

- `AppShell`
- `MenuBar`
- `ToolBar`
- `StatusBar`
- `Surface`
- `Panel`
- `ElevationLayer`

Control primitives:

- `Button`
- `TextField`
- `TextArea`
- `SelectField`
- `RangeField`
- `ColorField`
- `ProgressBar`
- `DisclosurePanel`
- `StepFlow`
- `StepFlowHeader`
- `StepFlowStep`
- `StepFlowActions`

Typography and layout primitives:

- `Text`
- `Heading`
- `Stack`
- `Cluster`
- `Grid`

Icon primitives:

- `Icon`
- `IconName`
- `IconSize`

## Guided Flow Contract

The component library now includes a narrow guided-flow API for setup and onboarding surfaces that would otherwise expose too much complexity up front.

Shared guided-flow primitives:

- `DisclosurePanel`
- `StepFlow`
- `StepFlowHeader`
- `StepFlowStep`
- `StepFlowActions`
- `StepStatus`

Usage rules:

- app crates own step state and validation
- shared primitives own badges, spacing, action-row layout, and reduced-motion-compatible transitions
- advanced controls should prefer `DisclosurePanel` before introducing a bespoke local wrapper

## Token Contract

Primitive styling consumes the semantic token families below:

- `--sys-color-*`
- `--sys-space-*`
- `--sys-radius-*`
- `--sys-font-*`
- `--sys-shadow-*`
- `--sys-motion-*`
- `--sys-focus-*`
- `--sys-elevation-*`
- `--sys-state-*`

Skin layers map existing `--ui-*`, `--fluent-*`, and `--neuro-*` values into `--sys-*` under `data-skin` scopes.

## DOM Contract

Shared primitives render stable root markers:

- `data-ui-primitive`
- `data-ui-kind`
- `data-ui-variant`
- `data-ui-size`
- `data-ui-tone`
- `data-ui-elevation`
- `data-ui-state`

New work should prefer `data-ui-*` roots and shared components over direct legacy `.app-*` class usage.

## Usage Restrictions

Built-in apps and runtime-owned shell surfaces must not:

- emit legacy primitive classes (`app-shell`, `app-menubar`, `app-toolbar`, `app-statusbar`, `app-action`, `app-field`, `app-editor`, `app-progress`) in Rust markup
- import icons from old runtime-local paths
- restyle primitive roots with ad hoc app-local visual overrides

App-local classes remain acceptable for:

- layout placement
- semantic region naming
- nonvisual DOM targeting

## Enforcement

`cargo xtask docs ui-conformance` enforces:

- skin selector scoping
- token/literal hygiene
- centralized icon usage
- shared primitive adoption via rejection of legacy primitive markup and old icon import paths

## Accessibility Invariants

Shared primitives must preserve:

- visible `:focus-visible` affordances
- reduced-motion support
- high-contrast support
- stable keyboard behavior for shared button/field surfaces

## Related Files

- [`crates/system_ui/src/lib.rs`](../../crates/system_ui/src/lib.rs)
- [`crates/system_ui/src/icon.rs`](../../crates/system_ui/src/icon.rs)
- [`crates/system_ui/src/primitives.rs`](../../crates/system_ui/src/primitives.rs)
- [`crates/site/src/theme_shell/00-tokens-reset.css`](../../crates/site/src/theme_shell/00-tokens-reset.css)
- [`crates/site/src/theme_shell/01-components-shell.css`](../../crates/site/src/theme_shell/01-components-shell.css)
- [`xtask/src/docs.rs`](../../xtask/src/docs.rs)
