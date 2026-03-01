---
title: "Desktop Shell HIG + Neumorphic Conformance Checklist"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-03-01"
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
- [`tools/automation/e2e_scenarios.toml`](../../tools/automation/e2e_scenarios.toml)
- [`/.artifacts/e2e/runs/`](../../.artifacts/e2e/runs/)
- [`tools/e2e/baselines/`](../../tools/e2e/baselines/)
- [`/.artifacts/ui-conformance/contrast/soft-neumorphic-contrast-report.json`](../../.artifacts/ui-conformance/contrast/soft-neumorphic-contrast-report.json)

Required canonical evidence expectations for the current workflow:

- `ui.neumorphic.layout`: desktop/tablet/mobile shell baseline
- `ui.neumorphic.navigation`: context menu + settings appearance view
- `ui.neumorphic.interaction`: start-button hover + focus-visible
- `ui.neumorphic.accessibility`: high-contrast, reduced-motion, settings accessibility view
- `ui.neumorphic.apps`: UI showcase controls + terminal default view

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
| `CMP-04` | Partial | All built-in app interiors use the shared depth language while preserving domain-specific readability. | The rebuilt soft-neumorphic primitive layer now covers segmented controls, icon buttons, switches, circular progress, and showcase-ready dials, and the `ui.neumorphic.apps` canonical scenario captures both the shared showcase surface and the terminal default surface. Existing built-in apps continue to inherit the shared shell/app token grammar, but broader per-app evidence beyond the canonical set remains pending. |
| `CMP-05` | Complete | Built-in app and shell surfaces compose from the shared `system_ui` primitive library or equivalent `data-ui-*` roots instead of legacy `.app-*` primitive markup. | `crates/system_ui` now owns the shared primitive catalog, runtime window/taskbar/menu shell surfaces compose through those primitives, and `cargo xtask docs ui-conformance` rejects legacy primitive markup in app/runtime crates. |

### C. Accessibility and Usability

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `A11Y-01` | Complete | Keyboard navigation, dismissal, and reducer-driven interaction semantics remain unchanged. | Runtime behavior remains driven by `desktop_runtime` reducer/components; the skin is CSS-only. |
| `A11Y-02` | Complete | Focus indication is separate from elevation styling and visible on all primary interactive elements. | `35-theme-soft-neumorphic-overrides.css` defines shared `:focus-visible` outlines using dedicated focus tokens, and `ui.neumorphic.interaction` now captures a deterministic start-button focus-visible evidence slice. |
| `A11Y-03` | Complete | Reduced-motion mode disables nonessential transitions and animations. | `data-reduced-motion="true"` suppression is present in the neumorphic overrides. |
| `A11Y-04` | Partial | Contrast targets for text, key icons, boundaries, and focus indicators are measured and recorded across light/dark/high-contrast variants. | The rebuilt skin raises shared text, placeholder, and focus tokens in code and high-contrast mode now overrides the new track/ring/icon-button token families as well. The previously checked-in contrast artifact still reflects the pre-rebuild sample set, so fresh measurements are still required before this item can be marked complete. |
| `A11Y-05` | Partial | Terminal transcript readability is preserved above the ambient shell softness in both light and dark modes. | The shell rebuild does not change terminal interaction semantics and retains the terminal-specific token family, but transcript/completion contrast should be remeasured after the new neumorphic palette landed because the existing artifact predates this rebuild. |

### D. Responsive and Governance Coverage

| ID | Status | Acceptance criteria (objective) | Current evidence / gap |
| --- | --- | --- | --- |
| `RSP-01` | Complete | The canonical automation set includes deterministic neumorphic shell layout, navigation, interaction, accessibility, and representative app-view coverage. | `tools/automation/e2e_scenarios.toml` defines `ui.neumorphic.layout`, `ui.neumorphic.navigation`, `ui.neumorphic.interaction`, `ui.neumorphic.accessibility`, and `ui.neumorphic.apps` as the blocking browser-backed workflow. |
| `RSP-02` | Complete | Desktop, tablet, and mobile evidence is captured and reviewed for the neumorphic shell baseline. | `ui.neumorphic.layout` captures the `shell.soft-neumorphic.default` slice at `1440x900`, `1024x768`, and `390x844`, and the resulting artifacts are indexed through the schema-v2 E2E manifest. |
| `DOC-01` | Complete | The canonical design-system reference documents the neumorphic token and component strategy. | `docs/reference/desktop-shell-neumorphic-design-system.md`. |
| `DOC-02` | Complete | Local machine-checkable validation enforces required skin scopes and literal hygiene for the new skin. | `cargo xtask docs ui-conformance` now validates the soft-neumorphic scope and override file hygiene. |

## Required Exit Criteria for Rigorous Claim

Before claiming rigorous neumorphic conformance rather than an aspirational implementation, all `Partial` items above must have review evidence attached for the current change set.

## Related Documents

- [`docs/reference/desktop-shell-neumorphic-design-system.md`](desktop-shell-neumorphic-design-system.md)
- [`docs/sop/ui-design-conformance-review-sop.md`](../sop/ui-design-conformance-review-sop.md)
