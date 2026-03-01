---
title: "Rustdoc and GitHub Wiki Documentation Strategy"
category: "reference"
owner: "architecture-owner"
status: "active"
last_reviewed: "2026-02-28"
audience: ["engineering", "platform"]
invariants:
  - "Rust API reference is generated from rustdoc and not maintained manually in Markdown."
  - "The GitHub Wiki is the canonical documentation hub and navigation layer; formal artifacts remain in their canonical storage surfaces and are indexed from the wiki."
tags: ["reference", "diataxis", "rustdoc", "wiki", "governance"]
domain: "docs"
lifecycle: "ga"
---

# Rustdoc and GitHub Wiki Documentation Strategy

This repository uses a split documentation model aligned with Diataxis and a hub-and-artifact strategy:

- The GitHub Wiki is the canonical documentation hub and primary navigation surface.
- `rustdoc` is the authoritative source for code-level API reference.
- `docs/` remains the repo-native canonical storage surface for documentation contracts, SOPs, ADRs, tooling reference, and supporting assets (validated by `cargo xtask docs`).

## Canonical Hub Model (Required)

The term "canonical" is used in two distinct ways:

- **Canonical hub**: the GitHub Wiki is the definitive reader-facing entry point and navigation system for project documentation.
- **Canonical artifact storage**: the underlying source-of-truth files remain type-specific (`rustdoc` for Rust APIs, `docs/` for ADR/SOP/governance/assets).

This avoids duplication drift while preserving a single place for discovery and cross-linking.

## Diataxis Mapping

| Content type | Primary surface | Source of truth |
| --- | --- | --- |
| Reference (Rust APIs) | Generated rustdoc HTML | Rust source comments (`///`, `//!`) |
| Reference (project architecture/operations/artifact indexes) | GitHub Wiki reference pages | external wiki repository pages + linked canonical artifacts in `docs/` / `rustdoc` |
| Tutorials | GitHub Wiki | external wiki repository pages |
| How-to guides | GitHub Wiki | external wiki repository pages |
| Explanations | GitHub Wiki | external wiki repository pages |
| ADR / SOP / docs governance | `docs/` (repo-native Markdown) | `docs/*.md` |

## Rustdoc Authoring Conventions (Required)

- Document crates and modules with `//!`.
- Document public items with `///` using clear intent-first summaries.
- Prefer describing behavior and invariants over repeating type signatures.
- Include runnable examples when the API is user-facing and executable in doctests.
- Use intra-doc links (for example, ``[`DesktopAction`]`` and ``[`reduce_desktop`]``) instead of raw text references.
- Keep docs aligned with visibility: public API docs are required; internal-only details are documented when they materially affect maintainability.

## GitHub Wiki Conventions (Required)

- Wiki repository is maintained as an external git-backed repository, not as a nested folder in the main source tree.
- `Home.md` is the canonical project documentation hub landing page and links to all Diataxis category indexes plus major reference registries.
- `_Sidebar.md` is maintained as the canonical navigation entrypoint.
- Tutorial/how-to/explanation/reference pages are separated by intent (no mixed-purpose pages).
- `API-Reference-(rustdoc).md` functions as the wiki reference entry point (rustdoc access + project reference index links).
- Wiki reference pages maintain structured registries/indexes for ADRs, SOPs/runbooks, diagrams, commands, and documentation artifacts.
- Wiki pages link to rustdoc for API signatures and item details instead of duplicating reference content.
- Wiki reference pages link to repo-native ADR/SOP/reference/assets instead of duplicating formal artifacts.

## Synchronization Rules

When code changes:

1. Update rustdoc for changed public APIs (crate/module/item docs, links, examples).
2. Update affected wiki pages when workflows, usage guidance, or rationale changed.
3. Update repo governance/SOP/reference pages under `docs/` when process, tooling, contracts, or formal artifacts changed.
4. Update the relevant wiki reference/index pages when formal artifacts are added or materially changed (for example ADR, SOP, diagram, command catalog, artifact registry entries).

When repo-native formal artifacts in `docs/` change:

1. Update the canonical artifact in `docs/`.
2. Update the corresponding wiki registry/index page so the hub remains complete.
3. Run docs validation (`cargo xtask docs all`) and any targeted checks (OpenAPI/Mermaid) if relevant.

When wiki content changes:

1. Confirm rustdoc and `docs/` links still point to valid canonical artifacts.
2. Commit the change in the external wiki repository.
3. Record the wiki branch and commit SHA in the main repo PR or review notes when the code/doc change depends on that wiki update.
4. Keep repo-native pointer pages in `docs/tutorials`, `docs/how-to`, and `docs/explanation` in sync only when the canonical wiki page URL changes.

## Wiki Maintenance Workflow (Required)

Use a consistent external-checkout flow before authoring or validating wiki content:

```bash
cargo wiki clone
```

Before editing wiki pages, inspect the configured checkout:

```bash
cargo wiki status
```

- The recommended checkout path is the sibling repository `../os.wiki` unless `OS_WIKI_PATH` overrides it.
- `cargo wiki verify` checks the external checkout remote URL, branch, dirty state, and upstream synchronization.
- Commit wiki changes in the external checkout itself; there is no parent-repo pointer update.

Commit and review workflow for wiki edits:

1. Commit changes inside the external wiki repository.
2. Include related main-repo code and rustdoc/docs changes in the normal PR flow.
3. Record the wiki branch and commit SHA in the PR description or review notes whenever the wiki changed.

## Wiki Hub Surface (Current)

The wiki is the canonical hub for:

- Diataxis learning and guidance content (tutorials, how-to guides, explanations)
- Project reference/index pages (architecture maps, diagrams, ADR/SOP registries, command catalogs, artifact registry)

Maintainer-facing documentation workflow pages migrated into the wiki remain canonical there:

- Tutorial: <https://github.com/justinrayshort/os/wiki/Tutorial-First-Documentation-Change>
- How-to: <https://github.com/justinrayshort/os/wiki/How-to-Update-Documentation-in-a-Pull-Request>
- Explanation: <https://github.com/justinrayshort/os/wiki/Explanation-Documentation-Architecture-and-Governance>

Repo-native pages under `docs/tutorials`, `docs/how-to`, and `docs/explanation` remain as machine-validated pointers for discoverability and governance navigation.

## Review and Pull Request Requirements

- Documentation changes are reviewed in the same PR as related code changes.
- PR authors record which rustdoc/wiki/docs surfaces changed, which validation commands were run, and the wiki branch/SHA whenever wiki content changed.
- Reviewers verify:
  - API changes are reflected in rustdoc
  - Wiki pages are updated or explicitly marked N/A
  - Wiki reference registries/indexes are updated when ADR/SOP/formal artifact inventories change
  - wiki branch/SHA is recorded when wiki content changes

## Validation

The local Rust toolchain validates both documentation layers:

- `cargo xtask docs wiki` (external checkout + wiki structure checks)
- `cargo doc --workspace --no-deps` (`RUSTDOCFLAGS=-D warnings`)
- `cargo test --workspace --doc` (runnable rustdoc examples)
- `cargo xtask docs all` (docs contracts + links + OpenAPI + Mermaid + wiki checks)

Local bootstrap for the external wiki checkout:

```bash
cargo wiki clone
```
