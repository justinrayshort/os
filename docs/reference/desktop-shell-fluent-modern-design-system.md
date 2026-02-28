---
title: "Desktop Shell Modern Adaptive Design System"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-28"
audience: ["engineering", "design"]
invariants:
  - "Shell icon usage flows through the centralized `system_ui` icon abstraction instead of ad-hoc text glyphs or inline per-component SVG markup."
  - "Theme-specific visual changes are applied via `data-skin` scoped CSS overrides with shared shell structure to preserve behavior parity across skins."
  - "Accessibility behaviors (focus visibility, keyboard navigation, high contrast, reduced motion) remain functional during visual refinements."
tags: ["reference", "design-system", "desktop-shell", "fluent", "icons", "accessibility"]
domain: "frontend"
lifecycle: "ga"
---

# Desktop Shell Modern Adaptive Design System

This reference documents the `modern-adaptive` skin and the tokenized shell/app primitive layer that delivers a Fluent-inspired desktop while preserving runtime behavior, layout logic, and keyboard interaction semantics.

## Scope

The `modern-adaptive` skin covers the desktop shell chrome and shared shell controls:

- desktop launcher icons
- window chrome (titlebars and window controls)
- taskbar (start button, pinned apps, running windows, overflow)
- tray widgets and clock
- launcher/start menu and taskbar menus
- desktop context menu wallpaper selection affordances
- System Settings app (display and accessibility surfaces)

Wallpaper rendering is a separate shell subsystem, not a skin extension. Skin/theme layers own token remapping only; wallpaper rendering and wallpaper asset selection remain reducer/runtime concerns.

App content areas (Calculator, Explorer, Notepad, Terminal, System Settings) now inherit shared primitive styling from `01-primitives.css` and switch appearance exclusively through `--sys-*` token remapping.

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

The modern-adaptive skin is scoped by the shell root skin attribute:

- `data-skin="modern-adaptive"` on `.desktop-shell`

Shared defaults and modern-adaptive remaps live in:

- [`crates/site/src/theme_shell/00-foundations.css`](../../crates/site/src/theme_shell/00-foundations.css)
- [`crates/site/src/theme_shell/01-primitives.css`](../../crates/site/src/theme_shell/01-primitives.css)
- [`crates/site/src/theme_shell/02-shell-layout.css`](../../crates/site/src/theme_shell/02-shell-layout.css)
- [`crates/site/src/theme_shell/03-responsive.css`](../../crates/site/src/theme_shell/03-responsive.css)
- [`crates/site/src/theme_shell/04-accessibility-motion.css`](../../crates/site/src/theme_shell/04-accessibility-motion.css)
- [`crates/site/src/theme_shell/30-theme-modern-adaptive.css`](../../crates/site/src/theme_shell/30-theme-modern-adaptive.css)

Primitive consumers rely on these shared token families:

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

The skin file itself is token-only. Base primitive styling lives in `01-primitives.css`, responsive behavior in `03-responsive.css`, and reduced-motion/high-contrast remaps in `04-accessibility-motion.css`.

Wallpaper boundary rules:

- skin files may provide atmosphere overlays, but they do not choose wallpaper assets
- wallpaper selection, display mode, and animation policy are driven by runtime wallpaper state
- theme changes must not implicitly change wallpaper selection

## Component Primitives and Styling Conventions

The redesign standardizes these shell primitives through `crates/system_ui` and `data-ui-*` roots:

- `Icon` for SVG icon rendering
- `DesktopRoot`, `DesktopBackdrop`, `DesktopIconGrid`, `DesktopIconButton`
- `WindowFrame`, `WindowTitleBar`, `WindowTitle`, `WindowControls`, `WindowControlButton`, `WindowBody`, `ResizeHandle`
- `Taskbar`, `TaskbarSection`, `TaskbarButton`, `TaskbarOverflowButton`, `TrayList`, `TrayButton`, `ClockButton`
- `MenuSurface`, `MenuItem`, `MenuSeparator`, `LauncherMenu`
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
- `TabList`, `Tab`, `Pane`, `PaneHeader`, `SplitLayout`
- `ListSurface`, `DataTable`, `Tree`, `TreeItem`, `InspectorGrid`
- `OptionCard`, `ToggleRow`, `PreviewFrame`, `Badge`, `EmptyState`
- `TerminalSurface`, `TerminalTranscript`, `TerminalLine`, `TerminalPrompt`, `CompletionList`, `CompletionItem`

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
- App-specific classes may extend semantics for testing or behavior hooks, but visual styling must remain on shared primitive selectors and `data-ui-*` contracts.
- Terminal remains transcript-first: no persistent toolbar, run button, or status chrome; one minimal startup hint; command-driven utility actions; and auto-follow output that disengages while the user reviews older transcript lines.

Conventions:

- Icons are decorative unless otherwise specified (`aria-hidden="true"` by default).
- Labels remain text-based for discoverability and assistive tech compatibility.
- Icon color is inherited from component foreground color (`currentColor`) to preserve state styling.
- Layout and component metrics should use spacing/radius/component metric tokens.
- App interaction states are standardized through shared `system_ui` primitives and `data-ui-*` states instead of per-app ad hoc state rules.
- Touch/hybrid ergonomics are expressed through shared `--sys-comp-*` and responsive token adjustments rather than skin-specific app override blocks.

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

Modern-adaptive supports adaptive theming through `prefers-color-scheme` token overrides while preserving the explicit runtime skin preset (`data-skin`).

Performance safeguards:

- appearance changes are implemented through token remapping and shared primitive selectors
- expensive effects are limited to key surfaces and guarded with capability checks
- reduced-motion mode disables transitions/animations at the shell scope

## Incremental Rollout Guidance

When extending the redesign to more shell or app surfaces:

1. Reuse `IconName` and `Icon` instead of introducing new icon markup per component.
2. Prefer semantic `--sys-*` or `--sys-comp-*` additions over one-off literals in component rules.
3. Scope visual changes under `.desktop-shell[data-skin="modern-adaptive"]` unless the change is intended for all skins.
4. Preserve keyboard, focus, and screen-reader semantics before adjusting appearance.
5. Validate on standard and high-DPI displays (especially icon alignment and small text in tray/taskbar regions).

## Related Files

- [`crates/desktop_runtime/src/components.rs`](../../crates/desktop_runtime/src/components.rs)
- [`crates/desktop_runtime/src/model.rs`](../../crates/desktop_runtime/src/model.rs)
- [`crates/desktop_runtime/src/reducer.rs`](../../crates/desktop_runtime/src/reducer.rs)
- [`crates/site/src/theme_shell/01-primitives.css`](../../crates/site/src/theme_shell/01-primitives.css)
- [`crates/site/src/theme_shell/30-theme-modern-adaptive.css`](../../crates/site/src/theme_shell/30-theme-modern-adaptive.css)
- [`docs/reference/desktop-shell-hig-fluent-conformance-checklist.md`](desktop-shell-hig-fluent-conformance-checklist.md)
- [`docs/sop/ui-design-conformance-review-sop.md`](../sop/ui-design-conformance-review-sop.md)
- [`wiki/Reference-Design-Materials-and-Artifacts.md`](../../wiki/Reference-Design-Materials-and-Artifacts.md)
