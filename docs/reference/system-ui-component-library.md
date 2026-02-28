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

- shared Leptos primitives for shell chrome, app shells, surfaces, navigation, controls, overlays, data-display, typography, icons, and layout
- centralized iconography (`Icon`, `IconName`, `IconSize`)
- stable `data-ui-*` DOM markers and slot/state contracts
- semantic `--sys-*` token consumption by primitive styles

The library does not own app logic, host contracts, reducer state, or runtime orchestration.

## Primitive Catalog

Layout primitives:

- `AppShell`
- `MenuBar`
- `ToolBar`
- `StatusBar`
- `Surface`
- `Panel`
- `ElevationLayer`
- `Stack`
- `Cluster`
- `Grid`
- `SplitLayout`

Shell primitives:

- `DesktopRoot`
- `DesktopBackdrop`
- `DesktopIconGrid`
- `DesktopIconButton`
- `WindowFrame`
- `WindowTitleBar`
- `WindowTitle`
- `WindowControls`
- `WindowControlButton`
- `WindowBody`
- `ResizeHandle`
- `Taskbar`
- `TaskbarSection`
- `TaskbarButton`
- `TaskbarOverflowButton`
- `TrayList`
- `TrayButton`
- `ClockButton`

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
- `OptionCard`
- `ToggleRow`
- `PreviewFrame`
- `Badge`
- `EmptyState`

Navigation and overlay primitives:

- `TabList`
- `Tab`
- `MenuSurface`
- `MenuItem`
- `MenuSeparator`
- `LauncherMenu`
- `CompletionList`
- `CompletionItem`

Data display and app-content primitives:

- `Pane`
- `PaneHeader`
- `ListSurface`
- `DataTable`
- `Tree`
- `TreeItem`
- `InspectorGrid`
- `TerminalSurface`
- `TerminalTranscript`
- `TerminalLine`
- `TerminalPrompt`

Typography primitives:

- `Text`
- `Heading`

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
- `--sys-type-*`
- `--sys-space-*`
- `--sys-size-*`
- `--sys-radius-*`
- `--sys-border-*`
- `--sys-font-*`
- `--sys-shadow-*`
- `--sys-motion-*`
- `--sys-focus-*`
- `--sys-elevation-*`
- `--sys-state-*`
- `--sys-opacity-*`
- `--sys-z-*`

Skin layers may derive values from legacy token families during migration, but primitive consumers only reference `--sys-*`. Skin switching is implemented by remapping `--sys-*` under `data-skin`, `data-high-contrast`, and `data-reduced-motion` scopes.

## DOM Contract

Shared primitives render stable root markers:

- `data-ui-primitive`
- `data-ui-kind`
- `data-ui-variant`
- `data-ui-size`
- `data-ui-tone`
- `data-ui-elevation`
- `data-ui-slot`

Shared primitives may also expose discrete state markers such as:

- `data-ui-selected`
- `data-ui-active`
- `data-ui-expanded`
- `data-ui-focused`
- `data-ui-pressed`
- `data-ui-disabled`
- `data-ui-minimized`

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

`cargo xtask docs ui-inventory` produces a machine-readable inventory of styling entry points across app crates, runtime shell markup, `system_ui`, and theme CSS.

`cargo xtask docs ui-conformance` enforces:

- skin selector scoping
- token/literal hygiene
- centralized icon usage
- shared primitive adoption via rejection of legacy primitive markup and old icon import paths
- token-only skin files
- restricted inline-style usage outside geometry/media positioning
- rejection of new app-specific or shell-bespoke visual selector contracts

## Accessibility Invariants

Shared primitives must preserve:

- visible `:focus-visible` affordances
- reduced-motion support
- high-contrast support
- stable keyboard behavior for shared button/field surfaces

## Related Files

- [`crates/system_ui/src/lib.rs`](../../crates/system_ui/src/lib.rs)
- [`crates/system_ui/src/icon.rs`](../../crates/system_ui/src/icon.rs)
- [`crates/system_ui/src/primitives/mod.rs`](../../crates/system_ui/src/primitives/mod.rs)
- [`crates/system_ui/src/primitives/shell.rs`](../../crates/system_ui/src/primitives/shell.rs)
- [`crates/system_ui/src/primitives/controls.rs`](../../crates/system_ui/src/primitives/controls.rs)
- [`crates/system_ui/src/primitives/navigation.rs`](../../crates/system_ui/src/primitives/navigation.rs)
- [`crates/system_ui/src/primitives/data_display.rs`](../../crates/system_ui/src/primitives/data_display.rs)
- [`crates/site/src/theme_shell/00-foundations.css`](../../crates/site/src/theme_shell/00-foundations.css)
- [`crates/site/src/theme_shell/01-primitives.css`](../../crates/site/src/theme_shell/01-primitives.css)
- [`xtask/src/docs.rs`](../../xtask/src/docs.rs)
