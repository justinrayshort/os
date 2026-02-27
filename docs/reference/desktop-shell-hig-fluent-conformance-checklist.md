---
title: "Desktop Shell HIG + Fluent Conformance Checklist"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-27"
audience: ["engineering", "design"]
invariants:
  - "Conformance status is assigned from explicit evidence and acceptance criteria, not visual preference alone."
  - "Apple HIG principles govern usability/accessibility fidelity while Fluent UI assets remain tokenized and semantically integrated."
  - "Shell UI changes that affect visuals or interaction behavior must update this checklist in the same review workflow."
tags: ["reference", "design-system", "apple-hig", "fluent-ui", "accessibility", "governance"]
domain: "frontend"
lifecycle: "ga"
---

# Desktop Shell HIG + Fluent Conformance Checklist

This reference is the authoritative progress assessment and acceptance checklist for desktop-shell UI conformance.

It evaluates the current implementation against:

- Apple Human Interface Guidelines principles (clarity, hierarchy, feedback, motion discipline, accessibility, consistency)
- project-specific Fluent UI integration rules (semantic icon usage, tokenized styling, consistent asset provenance)

## Scope and Assessment Basis

Current scope (assessed): desktop shell chrome plus inbuilt app UI surfaces rendered by `desktop_runtime` app mounts and themed by `crates/site/src/theme_shell/*`.

Included surfaces:

- desktop icons / launcher affordances
- window chrome and controls
- taskbar, tray widgets, overflow, clock menu
- start menu and taskbar/context menus
- System Settings app shell controls (display + accessibility)
- inbuilt app surfaces (Explorer, Notepad, Terminal, Calculator, System Settings)

Evidence snapshot used for this assessment (code inspection, 2026-02-27):

- `crates/desktop_runtime/src/components.rs`
- `crates/desktop_runtime/src/components/{a11y.rs,taskbar.rs,window.rs,menus.rs}`
- `crates/desktop_runtime/src/{icons.rs,model.rs,reducer.rs,persistence.rs}`
- `crates/site/src/theme_shell/{00,01,02,03,04,30,31,32,33}-*.css`
- `docs/reference/desktop-shell-fluent-modern-design-system.md`

Assessment notes:

- The working tree currently contains local edits in Fluent theme token/override files (`00`, `32`, `33`). This checklist reflects the current checked-out implementation state, including those local changes.
- No automated contrast auditing, axe/pa11y checks, or visual regression snapshots were found in the repository at assessment time. Items requiring those proofs are marked `Outstanding` or `Partial` even if implementation intent exists.

## Interpretation Rule (HIG + Fluent)

This project is not attempting a literal macOS clone. Conformance means:

- HIG principles are applied to hierarchy, legibility, feedback, motion, and accessibility behavior.
- Fluent UI assets/resources are used consistently as a designed visual system (currently iconography + token language), not mixed ad hoc with unrelated icon sets or one-off glyphs.
- When HIG principles and Fluent visual treatments conflict, preserve usability/accessibility first and document the deviation.

## Status Legend

- `Complete`: implementation exists and acceptance criteria are met with inspectable evidence in the current review cycle.
- `Partial`: implementation direction exists, but criteria are only partly met or validation evidence is incomplete.
- `Outstanding`: criteria are not yet implemented or no acceptable validation evidence exists.

## Criteria-Based Checklist

### A. Design Tokens (Typography, Spacing, Color, Motion)

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `TOK-01` | Complete | Fluent-modern theme defines named token layers for typography, spacing, radius, icon sizes, and motion durations in dedicated token files. | `crates/site/src/theme_shell/30-theme-fluent-modern-tokens-core.css` and `crates/site/src/theme_shell/32-theme-fluent-modern-theme-tokens.css` define `--fluent-font-*`, `--fluent-space-*`, `--fluent-radius-*`, `--fluent-icon-*`, `--fluent-motion-*`, and shell semantic tokens. |
| `TOK-02` | Complete | Skin token layers are scoped via typed `data-skin` attributes and do not replace other skins globally. | Skin scopes are applied under `.desktop-shell[data-skin="modern-adaptive"]`, `.desktop-shell[data-skin="classic-xp"]`, and `.desktop-shell[data-skin="classic-95"]` across split theme files; `cargo xtask docs ui-conformance` rejects deprecated `data-theme` selectors and unscoped skin overrides. |
| `TOK-03` | Complete | Interactive state colors (hover/active/focus/selection) use semantic tokens rather than repeated color literals, except documented visual-only overlays/gradients. | Fluent interactive and component surface colors are tokenized across shell/taskbar/menu/tray/clock/titlebar/settings/desktop-icon surfaces. Remaining raw color literals in `33-theme-fluent-modern-overrides.css` are limited to transparent gradient stops (`rgba(..., 0)`) used in decorative/overlay gradients, which are accepted visual-only exceptions. |
| `TOK-04` | Complete | Layout spacing/radius values in Fluent overrides use spacing/radius tokens for new work; raw `px` values require documented exceptions. | Fluent shell/taskbar/menu/tray/titlebar/settings/desktop-icon layout metrics and radii now use shared spacing/radius and component metric tokens. Remaining raw `px` literals in `33-theme-fluent-modern-overrides.css` are limited to documented effect geometry exceptions (hairline borders, shadow kernels, outline widths/offsets, transform nudges, and decorative gradient dimensions). |
| `TOK-05` | Complete | Typography hierarchy is tokenized beyond font family (minimum: body, caption/dense, title/chrome weights/sizes) or an explicit exception policy documents remaining fixed sizes. | Fluent shell typography tokens now define body/chrome/title/caption sizes plus body/chrome/title/caption emphasis weights (and body line-height) and are applied across shell root text, titlebar chrome, taskbar/start/menu labels, and tray/clock text in Fluent overrides. |
| `TOK-06` | Complete | Motion timings for Fluent shell transitions use shared motion tokens and support reduced-motion override suppression. | `--fluent-motion-*` tokens are used in Fluent overrides; runtime `data-reduced-motion="true"` CSS disables transitions/animations in `33-theme-fluent-modern-overrides.css`. |

### B. Component Primitives and Consistency

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `CMP-01` | Complete | A single reusable icon primitive renders Fluent icons for shell components. | `FluentIcon` component in `crates/desktop_runtime/src/icons.rs`; used across window/taskbar/menu/dialog components. |
| `CMP-02` | Complete | Shell chrome surfaces (windows, taskbar, menus, dialogs) share consistent visual rules for radius, borders, and elevation within the Fluent theme scope. | Fluent overrides apply common surface/radius/shadow treatment to `.desktop-window`, `.taskbar`, `.taskbar-menu`, `.start-menu`, `.desktop-context-menu`, `.settings-content`, and inbuilt app shells/panes (`.app-*`, explorer/notepad/terminal/calculator/settings surfaces). |
| `CMP-03` | Complete | Component states (hover, active, focused, minimized, pressed, selected) are represented via semantic classes/attributes and styled consistently. | Examples: `.taskbar-app.focused`, `.taskbar-app.minimized`, `.tray-widget.pressed`, `.wallpaper-picker-item.selected`, `.desktop-window.focused`. |
| `CMP-04` | Complete | Shared primitives/components are documented with explicit contracts (purpose, invariants, usage boundaries) in reference docs and code comments. | The design-system reference now includes an explicit app primitive catalog (`app-shell`, `app-menubar`, `app-toolbar`, `app-statusbar`, `app-action`, `app-field`, `app-editor`, `app-progress`) plus usage invariants; shell icon primitives remain documented in `icons.rs`. |
| `CMP-05` | Complete | Fluent icon integration preserves text labels for discoverability/accessibility on controls where labels are meaningful. | Start menu/taskbar labels remain text; icons are decorative (`aria-hidden`) while labels/`aria-label` provide meaning. |

### C. State Management Patterns and Theming Strategy

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `STA-01` | Complete | Theme-affecting shell preferences are modeled in runtime state and serialized for persistence. | `DesktopTheme` (`skin`, `wallpaper_id`, `high_contrast`, `reduced_motion`, `audio_enabled`) in `crates/desktop_runtime/src/model.rs`; persisted via `persist_theme()` in `persistence.rs`. |
| `STA-02` | Complete | Shell root exposes theme state to CSS through stable `data-*` attributes. | `DesktopShell` sets `data-skin`, `data-high-contrast`, `data-reduced-motion` in `crates/desktop_runtime/src/components.rs`. |
| `STA-03` | Complete | Every user-visible theme accessibility toggle (at minimum reduced motion + high contrast) has a reducer action, UI trigger, persistence, and CSS effect. | Reduced motion and high contrast now both have reducer actions (`SetReducedMotion`, `SetHighContrast`), taskbar tray toggle triggers, persisted theme writes, and CSS/data-attribute effects. |
| `STA-04` | Complete | Theme changes persist without bypassing reducer/runtime effect flow. | Reducer emits `RuntimeEffect::PersistTheme`; host persists theme via `DesktopHostContext` effect handler. |
| `STA-05` | Partial | Theme variant behavior (light/dark/high-contrast) has documented validation coverage criteria and recorded results per release/review. | Light/dark/high-contrast CSS branches exist, but no repeatable validation evidence workflow existed prior to this checklist/SOP. |

### D. Accessibility Compliance (Implementation and Validation)

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `A11Y-01` | Complete | Menus implement keyboard navigation (Arrow Up/Down, Home/End, Escape) with roving focus behavior and disabled-item filtering. | `handle_menu_roving_keydown()` and menu helpers in `crates/desktop_runtime/src/components/a11y.rs`; wired into menus in `menus.rs`. |
| `A11Y-02` | Partial | Settings and menu workflows provide predictable keyboard focus order and return focus to invoking surfaces after dismiss/escape actions. | Context menu/start/taskbar focus restoration is implemented in `crates/desktop_runtime/src/components.rs` and `a11y.rs`; old display-properties modal focus trap was removed when settings moved into a managed app window, and app-level tab-order validation remains pending. |
| `A11Y-03` | Complete | Interactive shell regions and menus use explicit roles and ARIA attributes appropriate to interaction patterns. | `role="toolbar"`, `group`, `menu`, `menuitem*`, `dialog`, `tablist`, `tab`, `tabpanel`, `listbox`, `option`, `aria-pressed`, `aria-haspopup`, `aria-expanded`, etc. |
| `A11Y-04` | Complete | Focus indicators are visible and theme-consistent for keyboard navigation using `:focus-visible`. | Fluent overrides define focus outlines with `--fluent-shell-focus`; keyboard-selected taskbar state is visually distinct. |
| `A11Y-05` | Partial | Reduced-motion support includes both system preference fallback and runtime override. | System fallback exists in `04-motion-base.css`; runtime override exists in Fluent theme via `data-reduced-motion`. Manual verification matrix not yet codified before this change. |
| `A11Y-06` | Complete | High-contrast rendering support includes token overrides and a reachable user-facing control path. | Fluent CSS token overrides remain in place and a taskbar tray widget now toggles high contrast through reducer state (`SetHighContrast`) and persisted theme updates. |
| `A11Y-07` | Outstanding | Contrast compliance is measured and recorded for text, icons, focus rings, and non-text boundaries across light/dark/high-contrast variants (`>=4.5:1` text, `>=3:1` UI boundaries/focus). | Design docs describe contrast goals, but no stored measurement reports or automated checks were found. |
| `A11Y-08` | Outstanding | Automated accessibility validation (e.g., axe/pa11y or equivalent) runs on key shell flows and is review-gated. | No a11y automation harness or results were found in repo tooling/docs. |
| `A11Y-09` | Outstanding | Manual screen-reader and keyboard test matrix results are documented per material UI change. | ARIA semantics are strong, but no formal evidence log/template existed before this SOP/checklist. |

### E. Interaction Patterns, Motion, and Behavioral Fidelity

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `INT-01` | Complete | Core shell interactions preserve predictable behavior (focus, minimize/restore/maximize, menu dismissal, keyboard escape handling). | Reducer-backed window actions in `reducer.rs`; interaction wiring in `window.rs`, `taskbar.rs`, and `menus.rs`. |
| `INT-02` | Complete | Motion is purposeful, brief, and limited to reinforcing state changes (not ornamental animation). | Fluent overrides primarily use short transitions on hover/focus/window emphasis; no heavy animated sequences in shell chrome. |
| `INT-03` | Complete | Backdrop and blur effects are capability-gated and degrade gracefully. | `@supports (backdrop-filter: blur(8px))` guard for taskbar/menus/dialog surfaces in `33-theme-fluent-modern-overrides.css`. |
| `INT-04` | Complete | Interaction behavior is state-driven in reducer/runtime, not encoded only in CSS/DOM event side effects. | Window/taskbar/start-menu behavior dispatches `DesktopAction` values; reducer remains authoritative for shell state changes. |
| `INT-05` | Partial | HIG-aligned pointer target sizing and coarse-pointer/touch ergonomics are explicitly validated (target size thresholds + test matrix). | Coarse-pointer ergonomics are now tokenized and implemented across app surfaces and taskbar controls (`--fluent-app-touch-*` plus coarse-pointer responsive rules in `03-responsive-base.css` and Fluent overrides), and runtime taskbar/window heuristics now adapt layout pressure via viewport-aware planning. A documented target-size test matrix and evidence pack remain outstanding. |

### F. Iconography and Fluent UI Asset Integration

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `ICO-01` | Complete | Shell icon usage is centralized through a semantic icon catalog and renderer (no ad hoc per-component SVG snippets). | `IconName`, `IconSize`, `FluentIcon`, and `app_icon_name()` in `crates/desktop_runtime/src/icons.rs`; used across shell components. |
| `ICO-02` | Complete | Icon sizing is standardized through named size tokens/enum values and reused consistently. | `IconSize::{Xs,Sm,Md,Lg}` maps to standard sizes; shell components use those enum values. |
| `ICO-03` | Complete | Decorative icons are hidden from assistive tech when labels/ARIA carry meaning. | `FluentIcon` renders `aria-hidden="true"`; components provide labels via text or `aria-label`. |
| `ICO-04` | Partial | Fluent asset provenance, subset policy, and update expectations are documented and linked from design-system governance docs. | Asset subset/provenance is documented in `desktop-shell-fluent-modern-design-system.md`; formal change-control/update procedure is introduced by this SOP but version pin/update checklist remains lightweight. |
| `ICO-05` | Complete | Static enforcement prevents regressions to legacy text glyphs/inline icon markup in shell components (lint/check/test). | `cargo xtask docs ui-conformance` (and `cargo xtask docs all`) now scans core shell component files for inline SVG/icon markup (`<svg`, `inner_html=`, path data) and legacy text glyph markers (`DIR`, `TXT`, `56K`) and fails on regressions, reinforcing `FluentIcon`/`IconName` usage. |

### G. Responsive and Adaptive Behavior

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `RSP-01` | Complete | Shell has explicit responsive breakpoints for taskbar density, icon layout, menu sizing, and window sizing on narrow viewports. | `03-responsive-base.css` defines `900px` and `640px` breakpoints affecting icons/taskbar/windows/start menu/calculator layout. |
| `RSP-02` | Complete | Small-screen window behavior prevents off-screen unusable windows (viewport clamping/resizing rules). | `03-responsive-base.css` forces `.desktop-window` bounds/size on `max-width: 640px`. |
| `RSP-03` | Partial | Adaptive theming covers light/dark variants with semantic token remapping and component override parity. | `prefers-color-scheme: dark` Fluent token + component overrides exist; validation evidence matrix is not yet recorded consistently. |
| `RSP-04` | Outstanding | Responsive behavior is validated against a documented viewport matrix (desktop/tablet/mobile) with screenshot or scripted evidence. | Breakpoint rules exist, but no formal viewport evidence package/process existed before this change. |

### H. Documentation Coverage and Governance

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `DOC-01` | Complete | Design-system reference documents Fluent shell scope, token strategy, iconography, and accessibility invariants. | `docs/reference/desktop-shell-fluent-modern-design-system.md`. |
| `DOC-02` | Complete | A criteria-based conformance checklist exists with objective acceptance criteria and explicit statuses (`Complete`/`Partial`/`Outstanding`). | This document (`docs/reference/desktop-shell-hig-fluent-conformance-checklist.md`). |
| `DOC-03` | Complete | A repeatable SOP defines review steps, evidence requirements, invariants, and change-control for UI conformance reviews. | `docs/sop/ui-design-conformance-review-sop.md`. |
| `DOC-04` | Complete | `AGENTS.md` codifies enforceable UI conformance expectations for future agent-driven UI changes. | `AGENTS.md` (UI conformance sections added in this change). |
| `DOC-05` | Complete | Wiki artifact/SOP registries index the conformance checklist and review SOP as part of canonical navigation. | Wiki reference registry pages updated in this change. |
| `DOC-06` | Complete | CI/local tooling includes machine-checkable UI conformance gates beyond documentation validation. | `cargo xtask docs ui-conformance` (and `cargo xtask docs all`) now enforces Fluent shell token/literal hygiene rules for `33-theme-fluent-modern-overrides.css` (with documented exception boundaries such as transparent gradient stops/effect geometry) and static icon-standardization regressions in shell component files. |

## Current Assessment Summary (2026-02-27)

The desktop shell implementation demonstrates a principled and technically coherent foundation for HIG-quality behavior with Fluent-style assets:

- strong semantic icon standardization (`FluentIcon`, `IconName`, `IconSize`)
- solid keyboard/focus/menu/dialog accessibility mechanics
- state-driven theming hooks (`data-skin`, `data-reduced-motion`, `data-high-contrast`)
- responsive shell adaptations and adaptive light/dark variants
- tokenized Fluent theme layers with scoped overrides and compatibility strategy

However, the project should not yet claim rigorous conformance at a high bar of refinement because key validation and governance gaps remain:

- no automated accessibility or contrast validation pipeline
- no formal viewport/screenshot evidence matrix for responsive/polish review
## Required Exit Criteria for "Rigorous Conformance" Claim

Before declaring the desktop shell rigorously conformant (rather than aspirational/partial), all `Outstanding` items in sections `A11Y`, `ICO`, and `RSP` must be closed.

## Related Documents

- [`docs/reference/desktop-shell-fluent-modern-design-system.md`](desktop-shell-fluent-modern-design-system.md)
- [`docs/sop/ui-design-conformance-review-sop.md`](../sop/ui-design-conformance-review-sop.md)
