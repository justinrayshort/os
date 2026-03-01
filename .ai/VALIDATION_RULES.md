# Validation Rules & Enforcement

**Version:** 1.0  
**Last Updated:** 2026-03-01  
**Audience:** Code reviewers, linting/CI automation, validation frameworks

Codifies architectural constraints, documentation governance, UI primitive enforcement, and linting rules that must be validated.

## Architectural Constraints (Checked via Code Review)

### 1. Host Boundary Enforcement

**Rule:** All host-domain APIs (filesystem, cache, notifications, process management) must flow through platform_host contracts.

**Violations:**
- ❌ App directly accessing Tauri API (tauri::fs, tauri::notification)
- ❌ Runtime code calling web-sys APIs directly
- ✅ App requesting file read through HostContext (which delegates to platform_host impl)

**Validation:** Code review; search for direct tauri::* or web_sys::* imports in apps/ and desktop_runtime/

### 2. Dependency Rules

**Rule:** No forbidden dependencies allowed.

| From | Allowed Deps | Forbidden |
|---|---|---|
| crates/apps/* | desktop_app_contract, platform_host, system_ui, system_shell_contract, leptos | desktop_runtime |
| desktop_runtime | platform_host, system_ui, system_shell, desktop_app_contract, tokio, leptos | apps/*, desktop_tauri |
| platform_host | (pure contract; only std, serde, thiserror, maybe serde_json) | any platform-specific crate |
| platform_host_web | platform_host, web-sys, wasm-bindgen, gloo | tauri, any native crate |
| desktop_tauri | platform_host, tauri, tokio | web-sys, wasm-bindgen |

**Validation:** `cargo tree --duplicates` and direct import audit

### 3. Circular Dependency Prevention

**Rule:** No circular dependencies; acyclic dependency graph required.

**Validation:** `cargo tree` must not show cycles

### 4. No Direct Reducer Access from Apps

**Rule:** Apps must not import or call desktop_runtime::reducer or state directly.

**Violations:**
- ❌ `use desktop_runtime::reducer;` in app code
- ❌ `state.reducer(action)` from within app
- ✅ Effects triggered via host context or app effects

**Validation:** Code review; grep for imports of desktop_runtime reducer in crates/apps/

## Documentation Governance (Checked via `cargo xtask docs`)

### 1. Frontmatter Requirements

**Rule:** All docs/*.md files must include complete frontmatter.

Required fields:
```yaml
---
title: "Document Title"
category: "tutorial|how-to|reference|explanation|adr|sop"
owner: "username or team"
status: "active|deprecated|draft|review"
last_reviewed: "YYYY-MM-DD"
audience: "developers|contributors|users"
invariants: "comma-separated critical properties or constraints"
---
```

**Validation:** `cargo xtask docs frontmatter` enforces presence and format

### 2. Diataxis Separation

**Rule:** Each Wiki page must be authored as a single Diataxis intent (not mixed).

| Type | Purpose | Structure |
|---|---|---|
| Tutorial | Learning by doing | Outcome, Entry Criteria, Procedure, Validation, Next Steps |
| How-to | Solving specific problems | How-to steps with prerequisites |
| Reference | Technical documentation | Structured API/concept reference |
| Explanation | Understanding concepts | Narrative explanation and rationale |

**Violation:** A page that mixes "how to use X" with "explanation of why X" without clear separation

**Validation:** Manual review; `cargo xtask docs wiki` validates instructional template structure for tutorials/how-to

### 3. Instructional Template (Tutorials & How-To)

**Rule:** Every `wiki/Tutorial-*.md` and `wiki/How-to-*.md` must follow exact structure:

```markdown
## Outcome
(Observable end state)

## Entry Criteria
### Prior Knowledge
### Environment Setup
### Dependencies

## Procedure
(Step-by-step instructions)

## Validation
(Verification steps)

## Next Steps
(Routing to next pages)
```

**Validation:** `cargo xtask docs wiki` enforces exact section presence and order

### 4. Review Freshness

**Rule:** Documentation must be reviewed and updated within 180 days.

**Tracking:** `last_reviewed` field in frontmatter

**Validation:** `cargo xtask docs audit-report` generates freshness warnings

### 5. Rustdoc Coverage

**Rule:** All public APIs (functions, types, traits, modules) must have rustdoc.

**Minimum requirements:**
- Summary sentence (first line)
- Errors section (if fallible)
- Examples section (if complex)
- Cross-references (if relevant)

**Validation:** `cargo doc --workspace --no-deps` with `RUSTDOCFLAGS=-D warnings` (when enabled)

## UI & Design System Enforcement (Checked via `cargo xtask docs ui-conformance`)

### 1. Shared Primitive Requirement

**Rule:** All shell UI must use system_ui primitives exclusively.

**Allowed:**
- `FluentButton`, `FluentIcon`, `FluentMenu`, etc. from system_ui
- Token-based theming via system_ui::tokens

**Forbidden:**
- Raw `<button>`, `<div>`, `<input>` HTML
- Inline styles or local CSS classes (except layout-only classes in specific layers)
- Direct color values instead of tokens

**Validation:** `cargo xtask docs ui-conformance` audits system_ui imports and tokens usage

### 2. Icon Usage Centralization

**Rule:** All icons must be sourced from crates/system_ui/src/icon.rs (IconName enum).

**Violations:**
- ❌ Inline Fluent icon SVG
- ❌ App-local icon definitions
- ✅ Use IconName::MyIcon registered in system_ui

**Validation:** `cargo xtask docs ui-conformance` checks for direct Fluent icon imports

### 3. Token Layer Enforcement

**Rule:** No hardcoded colors, spacing, or typography values in component code.

**Violations:**
- ❌ `color: "#FF5733"` in component CSS
- ❌ `padding: 16px` without token reference
- ✅ `color: tokens.primary_color()`, `padding: tokens.spacing_standard()`

**Validation:** Automated scanning for hex colors and hardcoded values in component styles

### 4. No Direct Primitive Composition in App/Runtime

**Rule:** App and runtime layers must not compose system_ui primitives with raw `data-ui-kind` or direct class manipulation.

**Violations:**
- ❌ `<FluentButton class="my-custom-layout" />`
- ❌ Adding `data-ui-kind` attributes in app code

**Allowed:**
- App-local layout-only classes (grid, flex containers)
- Using primitives as provided by system_ui

**Validation:** Structural scanning in `cargo xtask docs ui-conformance`

## Linting & Code Quality

### 1. Rustfmt (Format)

**Rule:** All code must be formatted per rustfmt defaults.

**Validation:** `cargo fmt --all -- --check`

### 2. Clippy (Lint)

**Rule:** All clippy warnings must be addressed or explicitly allowed.

**Exceptions:** `#[allow(clippy::...)]` with justification comment

**Validation:** `cargo clippy --workspace --all-targets`

### 3. Unsafe Code

**Rule:** All `unsafe` blocks must include `// SAFETY:` comment explaining why it's safe.

**Violations:**
- ❌ `unsafe { ... }` without comment
- ✅ `unsafe { ptr.as_ref() } // SAFETY: pointer is guaranteed valid by caller contract`

**Validation:** Code review; grep for unsafe without SAFETY comment

### 4. Unused Code Detection

**Rule:** No `#[allow(dead_code)]` without clear justification.

**Exceptions:** Legitimate platform-specific or feature-gated code with comment

**Validation:** Code review and `cargo check --all-targets`

## Testing Enforcement

### 1. Public API Test Coverage

**Rule:** All public functions must have at least one test.

**Minimum:** Happy-path test + error-case test for fallible functions

**Validation:** Manual review; consider code coverage tools if available

### 2. Doctest Validity

**Rule:** All rustdoc examples must be valid (marked `ignore` if they require setup).

**Validation:** `cargo test --workspace --doc`

### 3. No Test Panics

**Rule:** Tests must assert, not panic, on unexpected conditions.

**Violations:**
- ❌ `result.unwrap()` in test
- ✅ `assert!(result.is_ok())` or explicit error matching

**Validation:** Code review

## Git Workflow Enforcement (Checked via Commit Hooks)

### 1. Commit Message Format

**Rule:** All commits must follow format defined in GIT_WORKFLOW.md

**Minimum:**
- Clear subject line (50 chars)
- Body explaining why (if non-obvious)
- Include `Co-authored-by: Copilot <...>` trailer

**Validation:** Pre-commit hook (if configured)

### 2. No Direct Main Edits

**Rule:** Never push directly to main; use branch + review workflow.

**Validation:** Branch protection rules on main

## Validation Checklist (Before Commit)

- [ ] `cargo fmt --all -- --check` passes (no formatting issues)
- [ ] `cargo clippy --workspace --all-targets` has no new warnings
- [ ] `cargo test --workspace` passes
- [ ] `cargo doc --workspace --no-deps` builds without warnings
- [ ] `cargo test --workspace --doc` (doctests pass)
- [ ] `cargo xtask docs all` (docs contracts pass)
- [ ] `cargo xtask docs ui-conformance` (if UI changed)
- [ ] No `unsafe` without `// SAFETY:` comment
- [ ] Rustdoc present for all public APIs
- [ ] Git commit message is clear and includes trailers
- [ ] No forbidden dependencies introduced
- [ ] Host boundary not violated

## Running Validations Locally

```bash
# Quick validation (format, tests, docs build)
cargo verify-fast

# Full validation (all checks, all profiles)
cargo verify

# Specific validation
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
cargo doc --workspace --no-deps
cargo test --workspace --doc
cargo xtask docs all
cargo xtask docs ui-conformance
```

## Authority & Overrides

Validation rules can only be overridden by:
1. Explicit approval from repository maintainer
2. Documented exception in AGENTS.md section 2 ("Operating Rules")
3. Feature flag or conditional compilation (with justification)

Don't weaken validation rules without explicit request and documented rationale.
