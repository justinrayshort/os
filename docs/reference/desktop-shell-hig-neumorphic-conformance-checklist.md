---
title: "Desktop Shell HIG + Neumorphic Conformance Checklist"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-28"
audience: ["engineering", "design"]
invariants:
  - "Conformance status is assigned from explicit evidence and acceptance criteria, not visual preference alone."
  - "Apple HIG principles govern usability/accessibility fidelity while the neumorphic surface language remains tokenized and semantically integrated."
  - "Shell UI changes that affect visuals or interaction behavior must update this checklist in the same review workflow."
tags: ["reference", "design-system", "apple-hig", "neumorphic", "accessibility", "governance"]
domain: "frontend"
lifecycle: "ga"
---

# Desktop Shell HIG + Neumorphic Conformance Checklist

This reference is the authoritative acceptance checklist for the `soft-neumorphic` shell skin.

It evaluates the implementation against:

- Apple Human Interface Guidelines principles for clarity, hierarchy, feedback, motion discipline, and accessibility
- project-specific neumorphic constraints for tokenization, consistent depth language, explicit affordances, and controlled contrast
- existing icon governance that requires centralized `Icon` and `IconName` usage

## Scope and Assessment Basis

Current assessed scope:

- shell chrome and menus
- taskbar, tray, and launcher surfaces
- built-in app primitives and interiors
- adaptive light/dark and high-contrast variants
- responsive behavior across the documented screenshot matrix

Primary evidence sources:

- [`crates/desktop_runtime/src/model.rs`](../../crates/desktop_runtime/src/model.rs)
- [`crates/system_ui/src/icon.rs`](../../crates/system_ui/src/icon.rs)
- [`crates/site/src/theme_shell/01-primitives.css`](../../crates/site/src/theme_shell/01-primitives.css)
- [`crates/site/src/theme_shell/34-theme-soft-neumorphic.css`](../../crates/site/src/theme_shell/34-theme-soft-neumorphic.css)
- [`scripts/ui/capture-skin-matrix.sh`](../../scripts/ui/capture-skin-matrix.sh)
- [`scripts/ui/keyboard-flow-smoke.sh`](../../scripts/ui/keyboard-flow-smoke.sh)
- [`/.artifacts/ui-conformance/screenshots/`](../../.artifacts/ui-conformance/screenshots/)
- [`/.artifacts/ui-conformance/keyboard/keyboard-smoke-report.json`](../../.artifacts/ui-conformance/keyboard/keyboard-smoke-report.json)
- [`/.artifacts/ui-conformance/contrast/soft-neumorphic-contrast-report.json`](../../.artifacts/ui-conformance/contrast/soft-neumorphic-contrast-report.json)

## Status Legend

- `Complete`: implementation and validation evidence are present in the current review cycle
- `Partial`: implementation exists but evidence or coverage is incomplete
- `Outstanding`: implementation and/or acceptable validation evidence is missing

## Criteria-Based Checklist

### A. Tokens and Depth Language

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `TOK-01` | Complete | The neumorphic skin defines dedicated semantic token coverage for spacing, radius, motion, surfaces, text, borders, elevation, focus, and component roles. | `00-foundations.css` defines shared `--sys-*` families, including `--sys-light-*`, `--sys-depth-*`, `--sys-shadow-geometry-*`, and `--sys-surface-depth-*`, and `34-theme-soft-neumorphic.css` remaps them for the skin. |
| `TOK-02` | Complete | The skin is scoped via `.desktop-shell[data-skin="soft-neumorphic"]` and does not leak into other skins. | `34-theme-soft-neumorphic.css` and `cargo xtask docs ui-conformance`. |
| `TOK-03` | Complete | Raised, pressed, and inset states are represented through reusable elevation tokens rather than ad hoc per-component shadow geometry. | Shared primitive styling reads `--sys-elevation-*` tokens that are remapped in `34-theme-soft-neumorphic.css`. |
| `TOK-04` | Complete | Light and dark variants use dedicated token remapping rather than naive inversion. | `34-theme-soft-neumorphic.css` includes dark-scheme token remapping for surface, shadow, highlight, and accent values. |
| `TOK-05` | Complete | High-contrast mode flattens or strengthens borders/focus styles when subtle depth would reduce clarity. | `data-high-contrast="true"` token overrides replace subtle depth with stronger borders and focus emphasis. |

### B. Components and Interaction States

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `CMP-01` | Complete | Shell chrome surfaces share a consistent neumorphic treatment for radius, border softness, and elevation. | `01-primitives.css` applies the shared shell/app primitive contract, `34-theme-soft-neumorphic.css` remaps the relevant tokens, and runtime shell chrome now composes through `system_ui` shell primitives. |
| `CMP-02` | Complete | Shared built-in app primitives are styled consistently before app-specific extensions. | Shared primitive selectors in `01-primitives.css` style app shells, controls, panes, tabs, tables, terminal surfaces, and overlays before any app-specific semantic markers are applied. |
| `CMP-03` | Complete | Pressed, selected, focused, minimized, and active states are visually distinct without relying on shadow-only differences. | The skin uses inset treatments, accent borders, and explicit outlines for state changes. |
| `CMP-04` | Partial | All built-in app interiors use the shared depth language while preserving domain-specific readability. | Explorer and Settings now use state-driven setup/progressive-disclosure flows, while Notepad, Calculator, Terminal, and the lightweight Paint/Connection utilities use the same shared depth grammar; follow-up visual review should verify parity for every app state. |
| `CMP-05` | Complete | Built-in app and shell surfaces compose from the shared `system_ui` primitive library or equivalent `data-ui-*` roots instead of legacy `.app-*` primitive markup. | `crates/system_ui` now owns the shared primitive catalog, runtime window/taskbar/menu shell surfaces compose through those primitives, and `cargo xtask docs ui-conformance` rejects legacy primitive markup in app/runtime crates. |

### C. Accessibility and Usability

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `A11Y-01` | Complete | Keyboard navigation, dismissal, and reducer-driven interaction semantics remain unchanged. | Runtime behavior remains driven by `desktop_runtime` reducer/components; the skin is CSS-only. |
| `A11Y-02` | Complete | Focus indication is separate from elevation styling and visible on all primary interactive elements. | `35-theme-soft-neumorphic-overrides.css` defines shared `:focus-visible` outlines using dedicated focus tokens. |
| `A11Y-03` | Complete | Reduced-motion mode disables nonessential transitions and animations. | `data-reduced-motion="true"` suppression is present in the neumorphic overrides. |
| `A11Y-04` | Partial | Contrast targets for text, key icons, boundaries, and focus indicators are measured and recorded across light/dark/high-contrast variants. | [`soft-neumorphic-contrast-report.json`](../../.artifacts/ui-conformance/contrast/soft-neumorphic-contrast-report.json) now records a soft-neumorphic sample set. Current measurements show the taskbar start-button label at `1.2:1` against its control background and a window-frame boundary sample at `2.24:1`, both below target, while the start-button focus outline measures `5.2:1` and clears the `3:1` non-text threshold. `prefers-color-scheme: dark` emulation produced the same computed shell colors as light during this review, so dark-specific contrast evidence remains incomplete. |
| `A11Y-05` | Partial | Terminal transcript readability is preserved above the ambient shell softness in both light and dark modes. | [`soft-neumorphic-contrast-report.json`](../../.artifacts/ui-conformance/contrast/soft-neumorphic-contrast-report.json) measures a representative terminal transcript line at `3.15:1` against the terminal surface in the current browser-hosted shell. [`keyboard-smoke-report.json`](../../.artifacts/ui-conformance/keyboard/keyboard-smoke-report.json) confirms keyboard traversal still reaches and operates the Settings flow, but terminal transcript/completion contrast still needs follow-up to meet the documented `4.5:1` body-text goal if treated as standard reading text. |

### D. Responsive and Governance Coverage

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `RSP-01` | Complete | The screenshot matrix and keyboard smoke automation include `soft-neumorphic` alongside the other skins. | `scripts/ui/capture-skin-matrix.sh` and `scripts/ui/keyboard-flow-smoke.sh` enumerate the new skin, and [`keyboard-smoke-report.json`](../../.artifacts/ui-conformance/keyboard/keyboard-smoke-report.json) now records passing keyboard flows for all four skins. |
| `RSP-02` | Complete | Desktop, tablet, and mobile evidence is captured and reviewed for the neumorphic skin. | [`/.artifacts/ui-conformance/screenshots/`](../../.artifacts/ui-conformance/screenshots/) now contains the desktop, tablet, and mobile screenshot matrix for `soft-neumorphic` plus the comparison skins captured during this review cycle. |
| `DOC-01` | Complete | The canonical design-system reference documents the neumorphic token and component strategy. | `docs/reference/desktop-shell-neumorphic-design-system.md`. |
| `DOC-02` | Complete | Local machine-checkable validation enforces required skin scopes and literal hygiene for the new skin. | `cargo xtask docs ui-conformance` now validates the soft-neumorphic scope and override file hygiene. |

## Required Exit Criteria for Rigorous Claim

Before claiming rigorous neumorphic conformance rather than an aspirational implementation, all `Partial` items above must have review evidence attached for the current change set.

## Related Documents

- [`docs/reference/desktop-shell-neumorphic-design-system.md`](desktop-shell-neumorphic-design-system.md)
- [`docs/sop/ui-design-conformance-review-sop.md`](../sop/ui-design-conformance-review-sop.md)
