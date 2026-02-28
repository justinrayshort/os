# AGENTS.md

This repository is maintained with help from automated agents. Use this file as the repo-specific operating guide.

## 1) Project Scope

- Rust workspace with multiple crates and clear architectural boundaries, including:
  - `crates/site` (Leptos web entrypoints, routes, deep-link parsing, browser mount)
  - `crates/desktop_runtime` (desktop state model, reducer/effects, shell UI, app registry)
  - `crates/system_ui` (shared shell/app visual primitives, semantic icons, `data-ui-*` component contract)
  - `crates/platform_host` (typed host-domain contracts and shared models)
  - `crates/platform_host_web` (browser/wasm implementations of `platform_host` services)
  - `crates/apps/*` (desktop app crates such as calculator, explorer, notepad, terminal)
  - `xtask` (local workflow orchestration for docs, perf, verification, dev-server tasks)
- Documentation system split across:
  - Rust source comments (`//!`, `///`) -> generated `rustdoc` API reference (authoritative code-level Reference documentation)
  - GitHub Wiki repository as submodule under `wiki/` (canonical documentation hub and narrative/architectural record, organized by Diataxis)
  - Repo-native Markdown under `docs/` for formal artifact source files (contracts, SOPs, ADRs, tooling reference, diagrams/assets) indexed and cross-linked from the Wiki
  - Validation/audit CLI implemented in Rust via `cargo xtask docs` (`xtask/src/docs.rs`)
- Wiki instructional content (`Tutorial-*`, `How-to-*`) uses a shared structural template and is now validated by `cargo xtask docs wiki`.

## 2) Operating Rules

- Make minimal, reviewable changes that match existing patterns.
- Treat project documentation (`rustdoc` + Wiki + repo-native `docs/` artifacts) as the authoritative human-readable source of truth for system behavior, architecture, design decisions, and operations; keep it synchronized with implementation.
- If behavior, API shape, architecture, or procedures change, update docs in the same change/review workflow.
- Material code changes must update both:
  - `rustdoc` reference documentation for affected code
  - relevant Wiki pages (tutorial/how-to/explanation/reference registries) for changed behavior, interfaces, boundaries, or operational guidance
- All Rust source code must be documented using idiomatic `rustdoc` conventions at the crate, module, type, trait, and function levels.
- Rustdoc updates must include accurate behavior descriptions, invariants/constraints, error semantics, and examples/cross-references where appropriate.
- All documentation must follow Diataxis intent separation:
  - `rustdoc` content is Reference
  - Wiki pages must be explicitly authored as Tutorial / How-to Guide / Reference / Explanation and must not mix intents
- Preserve the host-boundary layering:
  - `platform_host` defines typed contracts/models
  - `platform_host_web` provides browser/wasm implementations
  - `desktop_tauri` owns native transport/bootstrap integration
- When editing Wiki tutorial/how-to pages, preserve the shared instructional template headings and order (validated by `cargo xtask docs wiki`).
- Material shell/UI design-system changes (theme tokens, shell component visuals, interaction patterns, iconography, responsive behavior, accessibility-affecting UI) must be reviewed against Apple HIG principles and the project neumorphic shell standards using:
  - `docs/reference/desktop-shell-hig-neumorphic-conformance-checklist.md`
  - `docs/sop/ui-design-conformance-review-sop.md`
- UI conformance claims must be evidence-based (checklist status updates plus keyboard/focus/motion/responsive validation and contrast measurements when colors/focus/borders change), not subjective visual approval alone.
- Preserve centralized shared icon usage (`crates/system_ui/src/icon.rs`), the `system_ui` primitive library, theme-scoped tokenization, and accessibility behavior during visual refinements; document any intentional deviations.
- Preserve documentation contracts enforced by `tools/docs/doc_contracts.json` and `cargo xtask docs`.
- Do not weaken validation rules or local verification workflows unless explicitly requested.
- Avoid destructive git commands unless explicitly requested.
- Do not casually edit generated web build artifacts under `crates/site/dist/` or `crates/site/target/trunk-dev-dist/` unless the task explicitly requires generated output updates.

## 3) Documentation Contracts (Required)

### 3.1 Validator-enforced (`cargo xtask docs`)

The docs validator enforces:

- Frontmatter required fields on docs pages:
  - `title`
  - `category`
  - `owner`
  - `status`
  - `last_reviewed`
  - `audience`
  - `invariants`
- Allowed categories:
  - `tutorial`, `how-to`, `reference`, `explanation`, `adr`, `sop`
- Folder/category mapping (Diataxis) under `docs/` must remain consistent.
- SOP docs must include the required SOP headings (validated by `sop` check).
- Review freshness threshold is tracked (currently 180 days) in audit reporting.
- Wiki submodule wiring and required Wiki pages (including `Home.md`, `_Sidebar.md`, category landing pages) via `cargo xtask docs wiki`.
- Wiki navigation expectations in `Home.md`, `OS-Wiki.md`, and `_Sidebar.md` via `cargo xtask docs wiki`.
- Wiki instructional template structure for `wiki/Tutorial-*.md` and `wiki/How-to-*.md`:
  - exact level-2 section sequence: `Outcome`, `Entry Criteria`, `Procedure`, `Validation`, `Next Steps`
  - exact `Entry Criteria` level-3 subsection sequence: `Prior Knowledge`, `Environment Setup`, `Dependencies`

### 3.2 Agent-enforced Documentation Requirements (Required)

- Wiki is the canonical documentation hub and canonical narrative/architectural record for the project, including:
  - architecture overviews and explanations
  - design rationale and decision context
  - ADR/SOP/diagram/tutorial/operational guidance indexes and cross-links
  - contributor-facing workflows and maintainership guidance
- Formal ADR/SOP/reference/asset source files may remain canonically stored in `docs/`, but the Wiki must be updated as the canonical navigation and narrative layer when those artifacts are added or materially changed.
- Rustdoc is the authoritative code-level Reference surface and must be kept current for all affected crates/modules/public types/traits/functions.
- Rustdoc should use idiomatic conventions:
  - crate/module overviews with `//!`
  - item-level docs with `///`
  - clear summaries, invariants, and error behavior
  - runnable examples for user-facing APIs when practical
  - intra-doc links/cross-references to related components
- Documentation changes must preserve strict Diataxis separation by user intent.
- Wiki explanations should preserve a coherent narrative sequence for contributors (architecture -> host/storage boundary -> technology/tooling -> performance -> documentation governance) unless the change intentionally revises the narrative structure.

### 3.3 Instructional Page Authoring Contract (Wiki Tutorials + How-To)

For every `wiki/Tutorial-*.md` and `wiki/How-to-*.md` page:

- Use the shared instructional template headings in order:
  - `## Outcome`
  - `## Entry Criteria`
  - `## Procedure`
  - `## Validation`
  - `## Next Steps`
- `## Entry Criteria` must include these `###` subsections in order:
  - `### Prior Knowledge`
  - `### Environment Setup`
  - `### Dependencies`
- `## Outcome` must define an observable/verifiable end state.
- `## Validation` must provide concrete confirmation steps and expected results.
- `## Next Steps` must intentionally route to the next Tutorial / How-to / Explanation / Reference page.

## 4) Local Verification Workflows (Current)

### 4.1 Documentation Verification (Local Rust Toolchain)

Primary entry points:

- `cargo xtask docs wiki`
- `cargo xtask docs all`
- `cargo doc --workspace --no-deps`
- `cargo test --workspace --doc`
- `cargo verify-fast`
- `cargo verify-fast --with-desktop`
- `cargo verify-fast --without-desktop`
- `cargo verify --profile <name>`
- `cargo verify`
- `cargo doctor`
- `cargo perf doctor`
- `cargo perf check`
- `cargo perf bench`
- `cargo perf dev-loop-baseline --output .artifacts/perf/reports/dev-loop-baseline.json`

Stages (automation-backed verification order):

1. Rust format + default test matrix (`cargo verify-fast` / `cargo verify`)
2. Rust all-features matrix (`cargo verify-fast` / `cargo verify`)
3. Rustdoc build + doctests (`cargo verify-fast` / `cargo verify`)
4. Documentation validation + docs audit artifact (`cargo verify-fast` / `cargo verify`)
5. Prototype compile checks (`cargo verify` only)
6. Clippy lint checks (`cargo verify` only; skipped with warning when unavailable)

Docs-focused verification order (`cargo xtask docs all` internals):

1. Docs + wiki validation (`cargo xtask docs all`, which includes wiki structure/template checks)
2. Docs contract validation (`structure`, `frontmatter`, `sop`)
3. OpenAPI validation (`cargo xtask docs openapi`)
4. Mermaid validation (`cargo xtask docs mermaid`)
5. Broken internal reference detection (`cargo xtask docs links`)
6. Typed app-state boundary enforcement (`cargo xtask docs storage-boundary`) to prevent direct envelope-load usage in app/runtime crates
7. UI conformance token/literal + icon/primitive-standardization audit (`cargo xtask docs ui-conformance`) for shared shell design-system hygiene
8. Rustdoc build (`cargo doc --workspace --no-deps`, `RUSTDOCFLAGS=-D warnings` when tightening docs quality)
9. Rustdoc doctests (`cargo test --workspace --doc`)
10. Audit artifact generation (`cargo xtask docs audit-report --output ...`) when needed

### 4.2 Quarterly Documentation Audit (Manual / Local)

Behavior:

- Run locally on a quarterly cadence (or before governance reviews)
- Validates wiki submodule structure and docs contracts (via `audit-report`)
- Generates `.artifacts/docs-audit.json` via `audit-report`
- Fails locally if audit validation fails
- Preserve/share the audit artifact through your normal review process (no hosted CI dependency)

### 4.3 UI Design Conformance Review (Local / Per Material UI Change)

Use the formal checklist and SOP for shell/UI design changes:

- `docs/reference/desktop-shell-hig-neumorphic-conformance-checklist.md`
- `docs/sop/ui-design-conformance-review-sop.md`

Minimum local review expectations for material UI changes:

1. Classify affected surfaces (`tokens`, `primitives`, `interaction`, `a11y`, `iconography`, `responsive`, `docs-governance`).
2. Gather evidence for the affected checklist IDs (keyboard/focus behavior, reduced motion, adaptive theming, responsive behavior, contrast if colors/focus/borders changed).
3. Run correctness/docs validation:
   - `cargo check --workspace`
   - `cargo test --workspace`
   - `cargo xtask docs all`
4. Update the checklist status entries and related design-system docs in the same review workflow.
5. If a formal docs artifact is added/changed, update the relevant Wiki registry page(s) in `wiki/`.

### 4.4 `xtask` / Validator Changes (Local)

When changing `xtask/src/docs.rs`, `xtask/src/perf.rs`, or command/workflow semantics:

1. Run `cargo fmt --all`.
2. Run `cargo test -p xtask`.
3. Run the affected workflow commands (for example `cargo xtask docs all`, `cargo xtask docs wiki` for isolated wiki diagnostics, `cargo perf doctor`).
4. Update `AGENTS.md`, wiki/reference docs, and command catalogs when behavior/contracts changed.

## 5) Local Commands

### 5.1 Docs Tooling Setup (Rust-only)

```bash
cargo build -p xtask
```

### 5.2 Docs Validation (fast path)

Run the standard local docs validation entry point:

```bash
git submodule update --init --recursive
cargo xtask docs all
cargo doc --workspace --no-deps
cargo test --workspace --doc
```

`cargo xtask docs all` includes wiki validation. Add `cargo xtask docs wiki` when you want staged or isolated wiki diagnostics.

### 5.3 Docs Commands (explicit)

```bash
git submodule update --init --recursive
cargo xtask docs structure
cargo xtask docs wiki
cargo xtask docs frontmatter
cargo xtask docs sop
cargo xtask docs openapi
cargo xtask docs mermaid
cargo xtask docs links
cargo xtask docs storage-boundary
cargo xtask docs ui-conformance
cargo doc --workspace --no-deps
cargo test --workspace --doc
```

### 5.4 Audit Report Command

```bash
cargo xtask docs audit-report --output .artifacts/docs-audit.json
```

### 5.5 Rust Workspace Commands

Prefer direct Cargo commands for clarity (there is no `package.json` script wrapper in this repo). A root `Makefile` remains only as a small compatibility shell over the primary Cargo aliases:

```bash
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
```

### 5.6 Performance Engineering Commands (Local / Controlled Benchmarking)

Standardized performance workflow entry points are exposed via `cargo xtask perf` and the `cargo perf` alias. Use these commands to keep profiling and benchmark runs repeatable, with artifacts under `.artifacts/perf/`.

```bash
cargo perf doctor
cargo perf check
cargo perf bench
cargo perf baseline local-main
cargo perf compare local-main
cargo perf dev-loop-baseline --output .artifacts/perf/reports/dev-loop-baseline.json
cargo perf flamegraph --bench <bench_name>
cargo perf heaptrack -- cargo bench --workspace
```

Notes:

- `cargo perf check` runs tests/doctests and compiles benchmark targets before optimization work.
- `cargo perf baseline` / `cargo perf compare` assume Criterion-style benchmark flags and append `-- --save-baseline/--baseline` automatically.
- `cargo perf flamegraph` and `cargo perf heaptrack` require optional local tooling and may be platform-specific.
- `cargo perf doctor` reports `sccache` availability and active `RUSTC_WRAPPER` status.
- Optional local cache setup helper: `source scripts/dev/setup-sccache.sh`.

### 5.7 Cargo Aliases / Convenience Wrappers (Current)

From `.cargo/config.toml`:

```bash
cargo dev
cargo setup-web
cargo web-check
cargo web-build
cargo flow
cargo doctor
cargo docs-check
cargo docs-audit
cargo perf <subcommand>
cargo verify-fast
cargo verify
cargo check-all
cargo test-all
```

Compatibility `make` wrappers:

```bash
make verify-fast
make verify
make wiki-init
make rustdoc-check
make docs-check
make proto-serve
make proto-stop
make proto-status
```

### 5.8 `xtask` / Validator Development Checks

```bash
cargo fmt --all
cargo test -p xtask
cargo xtask docs all
```

Run `cargo xtask docs wiki` in addition when validating staged wiki-only diagnostics.

## 6) Change Workflows for Agents

### 6.1 Docs-only changes

1. Classify the change surface: rustdoc (`crates/**` comments), wiki (`wiki/*.md`), or repo docs (`docs/`).
   - For Wiki changes, classify the page explicitly as Tutorial / How-to Guide / Reference / Explanation and keep content scoped to that intent.
   - For rustdoc changes, treat the content as Reference documentation.
2. Initialize/update the wiki submodule if touching wiki content (`git submodule update --init --recursive`).
3. Keep docs frontmatter complete and valid for `docs/*.md` changes.
4. If editing `wiki/Tutorial-*.md` or `wiki/How-to-*.md`, preserve the shared instructional template (`Outcome`, `Entry Criteria`, `Procedure`, `Validation`, `Next Steps` + required `Entry Criteria` subsections).
5. If ADRs, SOPs, diagrams, or other formal artifacts changed in `docs/`, update the relevant Wiki reference/index pages in the same change.
6. Run `cargo xtask docs all`, `cargo xtask docs storage-boundary`, `cargo xtask docs ui-conformance` (when Fluent shell UI/token conformance surfaces changed), and `cargo xtask docs wiki` when you need isolated wiki diagnostics.
7. Run `cargo doc --workspace --no-deps` and `cargo test --workspace --doc` when rustdoc changed (recommended for all docs changes that mention APIs).
8. If Mermaid or OpenAPI changed, run targeted checks (`cargo xtask docs mermaid`, `cargo xtask docs openapi`) in addition to `all`.
9. Generate an audit artifact (`cargo xtask docs audit-report --output .artifacts/docs-audit.json`) when the change affects governance/reporting flows.
10. For performance-sensitive changes, run `cargo perf check` and the relevant benchmark/profile commands (for example `cargo perf bench`, `cargo perf compare <baseline>`) and document measured deltas/tradeoffs in code review plus the relevant wiki/docs pages.

### 6.2 Code + docs changes

1. Update Rust code in the relevant crate(s).
2. Update rustdoc in the same change for affected crates/modules/types/traits/functions (behavior, invariants, errors, examples, and cross-references as applicable).
3. Update relevant Wiki content in the same change/review workflow whenever interfaces, behavior, user workflows, operational guidance, or system boundaries changed.
   - Choose the correct Diataxis page type (Tutorial / How-to Guide / Reference / Explanation).
   - If editing a Wiki tutorial/how-to page, preserve the enforced instructional template headings.
   - Update Wiki reference/index pages when formal artifacts (ADR/SOP/diagram/command catalogs) are added or changed.
4. Update affected governance docs/ADR/SOPs/diagrams/specs in `docs/` when process/contracts/architecture/operations changed.
5. Keep rustdoc and Wiki descriptions synchronized with each other and with the implementation before requesting review.
6. Run targeted Cargo checks (`cargo test --workspace` if behavior changed).
7. Run rustdoc checks (`cargo doc --workspace --no-deps`, `cargo test --workspace --doc`).
8. Run docs validation (`cargo xtask docs all`; add `cargo xtask docs wiki` for isolated wiki diagnostics).
9. If `xtask` docs/perf behavior changed, run `cargo test -p xtask` and update command/docs guidance (`AGENTS.md`, Wiki, `docs/reference/*`) as needed.
10. If wiki content changed, commit `wiki/` changes and include the updated `wiki/` submodule pointer in the same PR/change set.

### 6.3 UI design-system changes (code + visuals + docs)

1. Classify the UI change surface (tokens, primitives/components, interaction behavior, accessibility, iconography, responsive/adaptive theming).
2. Reuse existing shell primitives and conventions first (`FluentIcon`, semantic `IconName` mapping, theme-scoped token overrides, reducer-driven UI state).
3. Update implementation and preserve behavioral/accessibility invariants (keyboard navigation, focus visibility, reduced motion support, dialog/menu semantics).
4. Update `docs/reference/desktop-shell-hig-neumorphic-conformance-checklist.md` with evidence-based status changes for affected checklist items.
5. Update `docs/reference/desktop-shell-neumorphic-design-system.md` when token sets, primitives, invariants, or scope materially change.
6. Follow `docs/sop/ui-design-conformance-review-sop.md` for evidence collection, validation, and deviation handling.
7. Run `cargo check --workspace`, `cargo test --workspace`, `cargo xtask docs ui-conformance`, and `cargo xtask docs all` (plus rustdoc checks when rustdoc changed).
8. If formal docs artifacts or registries changed, update the relevant wiki reference pages and include the `wiki/` submodule pointer update in the same PR/change set.

### 6.4 Host Boundary / Storage Migration Changes (Code + Docs)

When changing `platform_host`, `platform_host_web`, or `desktop_tauri` contracts/behavior:

1. Preserve documented compatibility invariants unless an explicit migration plan is part of the change (namespaces, envelope semantics, explorer cache/prefs conventions, IndexedDB/object-store names where applicable).
2. Update rustdoc for affected contracts and adapters (traits, models, error semantics, examples if user-facing).
3. Update Wiki explanations/reference pages:
   - `Explanation-System-Architecture-Overview`
   - `Explanation-Browser-Host-Boundary-and-Storage-Model`
   - `Reference-System-Architecture-Map`
4. Add/update ADRs in `docs/adr/*` when boundary decisions or migration contracts change materially.
5. Run targeted tests/checks for affected crates plus `cargo xtask docs all` (`cargo xtask docs wiki` optional for isolated wiki diagnostics).

## 7) Key Files

- `AGENTS.md` (this repository-specific operating guide for agents)
- `xtask/src/docs.rs` (docs contract/integrity/audit CLI implementation; includes wiki instructional template validation)
- `xtask/src/perf.rs` (performance benchmarking/profiling workflow CLI implementation)
- `tools/docs/doc_contracts.json` (docs schema/contract rules)
- `.gitmodules` (wiki submodule declaration)
- `wiki/` (GitHub Wiki submodule checkout; canonical navigation + Diataxis narrative)
- `docs/reference/rustdoc-and-github-wiki-documentation-strategy.md` (documentation surface split policy)
- `docs/reference/documentation-toolchain-and-ci.md` (docs tooling/validation pipeline reference)
- `docs/reference/project-command-entrypoints.md` (command catalog / entrypoint reference)
- `tools/automation/verify_profiles.toml` (profile-driven verification policy surface used by `cargo verify --profile`)
- `docs/reference/performance-engineering-and-benchmarking.md` (performance workflow reference)
- `docs/reference/desktop-shell-neumorphic-design-system.md` (shell neumorphic design-system reference and invariants)
- `docs/reference/desktop-shell-hig-neumorphic-conformance-checklist.md` (objective HIG + neumorphic conformance status checklist)
- `docs/sop/ui-design-conformance-review-sop.md` (repeatable UI conformance review and change-control procedure)
- `.cargo/config.toml` (Cargo aliases for local workflows)
- `Makefile` (minimal compatibility wrappers delegating to Cargo aliases)
- `crates/platform_host/src/` (typed host contracts and shared models)
- `crates/platform_host_web/src/` (browser/wasm implementations of `platform_host` services)

## 8) Final Response Expectations (for agents)

In completion summaries:

- State what changed.
- List commands run (and whether they passed).
- Call out any checks not run (and why).
- If wiki content changed, state whether the `wiki/` submodule pointer changed and whether wiki commits/pushes were performed.
