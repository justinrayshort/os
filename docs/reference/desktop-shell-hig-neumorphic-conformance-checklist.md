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
- [`crates/site/src/theme_shell/34-theme-soft-neumorphic-tokens.css`](../../crates/site/src/theme_shell/34-theme-soft-neumorphic-tokens.css)
- [`crates/site/src/theme_shell/35-theme-soft-neumorphic-overrides.css`](../../crates/site/src/theme_shell/35-theme-soft-neumorphic-overrides.css)
- [`scripts/ui/capture-skin-matrix.sh`](../../scripts/ui/capture-skin-matrix.sh)
- [`scripts/ui/keyboard-flow-smoke.sh`](../../scripts/ui/keyboard-flow-smoke.sh)

## Status Legend

- `Complete`: implementation and validation evidence are present in the current review cycle
- `Partial`: implementation exists but evidence or coverage is incomplete
- `Outstanding`: implementation and/or acceptable validation evidence is missing

## Criteria-Based Checklist

### A. Tokens and Depth Language

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `TOK-01` | Complete | The neumorphic skin defines dedicated semantic token families for spacing, radius, motion, surfaces, text, borders, elevation, highlights, shadows, focus, and app primitives. | `34-theme-soft-neumorphic-tokens.css` defines the `--neuro-*` token families plus shared `--terminal-*` mappings. |
| `TOK-02` | Complete | The skin is scoped via `.desktop-shell[data-skin="soft-neumorphic"]` and does not leak into other skins. | `34-theme-soft-neumorphic-tokens.css`, `35-theme-soft-neumorphic-overrides.css`, and `cargo xtask docs ui-conformance`. |
| `TOK-03` | Complete | Raised, pressed, and inset states are represented through reusable elevation tokens rather than ad hoc per-component shadow geometry. | `--neuro-elevation-raised-*`, `--neuro-elevation-pressed-*`, and `--neuro-elevation-inset-*` are reused across shell chrome and app primitives. |
| `TOK-04` | Complete | Light and dark variants use dedicated token remapping rather than naive inversion. | `34-theme-soft-neumorphic-tokens.css` includes `prefers-color-scheme: dark` remapping for surface, shadow, highlight, and accent tokens. |
| `TOK-05` | Complete | High-contrast mode flattens or strengthens borders/focus styles when subtle depth would reduce clarity. | `data-high-contrast="true"` token overrides replace subtle depth with stronger borders and focus emphasis. |

### B. Components and Interaction States

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `CMP-01` | Complete | Shell chrome surfaces share a consistent neumorphic treatment for radius, border softness, and elevation. | `35-theme-soft-neumorphic-overrides.css` applies a shared visual contract to windows, taskbar, menus, dialogs, and tray surfaces. |
| `CMP-02` | Complete | Shared built-in app primitives are styled consistently before app-specific extensions. | `app-shell`, `app-menubar`, `app-toolbar`, `app-statusbar`, `app-action`, `app-field`, `app-editor`, and `app-progress` receive skin-scoped overrides. |
| `CMP-03` | Complete | Pressed, selected, focused, minimized, and active states are visually distinct without relying on shadow-only differences. | The skin uses inset treatments, accent borders, and explicit outlines for state changes. |
| `CMP-04` | Partial | All built-in app interiors use the shared depth language while preserving domain-specific readability. | Explorer, Notepad, Calculator, Settings, Terminal, and placeholder surfaces are covered; follow-up visual review should verify parity for every app state. |
| `CMP-05` | Complete | Built-in app and shell surfaces compose from the shared `system_ui` primitive library or equivalent `data-ui-*` roots instead of legacy `.app-*` primitive markup. | `crates/system_ui` now owns the shared primitive catalog and `cargo xtask docs ui-conformance` rejects legacy primitive markup in app/runtime crates. |

### C. Accessibility and Usability

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `A11Y-01` | Complete | Keyboard navigation, dismissal, and reducer-driven interaction semantics remain unchanged. | Runtime behavior remains driven by `desktop_runtime` reducer/components; the skin is CSS-only. |
| `A11Y-02` | Complete | Focus indication is separate from elevation styling and visible on all primary interactive elements. | `35-theme-soft-neumorphic-overrides.css` defines shared `:focus-visible` outlines using dedicated focus tokens. |
| `A11Y-03` | Complete | Reduced-motion mode disables nonessential transitions and animations. | `data-reduced-motion="true"` suppression is present in the neumorphic overrides. |
| `A11Y-04` | Partial | Contrast targets for text, key icons, boundaries, and focus indicators are measured and recorded across light/dark/high-contrast variants. | Design targets are documented, but measured reports remain a required evidence artifact for each material revision. |
| `A11Y-05` | Partial | Terminal transcript readability is preserved above the ambient shell softness in both light and dark modes. | Terminal-specific token mapping exists; review evidence should confirm transcript contrast and completion clarity. |

### D. Responsive and Governance Coverage

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `RSP-01` | Complete | The screenshot matrix and keyboard smoke automation include `soft-neumorphic` alongside the other skins. | `scripts/ui/capture-skin-matrix.sh` and `scripts/ui/keyboard-flow-smoke.sh` enumerate the new skin. |
| `RSP-02` | Partial | Desktop, tablet, and mobile evidence is captured and reviewed for the neumorphic skin. | Scripts exist; generated evidence must still be captured as part of the review cycle. |
| `DOC-01` | Complete | The canonical design-system reference documents the neumorphic token and component strategy. | `docs/reference/desktop-shell-neumorphic-design-system.md`. |
| `DOC-02` | Complete | Local machine-checkable validation enforces required skin scopes and literal hygiene for the new skin. | `cargo xtask docs ui-conformance` now validates the soft-neumorphic scope and override file hygiene. |

## Required Exit Criteria for Rigorous Claim

Before claiming rigorous neumorphic conformance rather than an aspirational implementation, all `Partial` items above must have review evidence attached for the current change set.

## Related Documents

- [`docs/reference/desktop-shell-neumorphic-design-system.md`](desktop-shell-neumorphic-design-system.md)
- [`docs/sop/ui-design-conformance-review-sop.md`](../sop/ui-design-conformance-review-sop.md)
