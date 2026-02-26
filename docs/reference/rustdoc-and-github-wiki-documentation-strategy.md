---
title: "Rustdoc and GitHub Wiki Documentation Strategy"
category: "reference"
owner: "architecture-owner"
status: "active"
last_reviewed: "2026-02-26"
audience: ["engineering", "platform"]
invariants:
  - "Rust API reference is generated from rustdoc and not maintained manually in Markdown."
  - "Tutorial, how-to, and explanation content is maintained in the GitHub Wiki and reviewed alongside code."
tags: ["reference", "diataxis", "rustdoc", "wiki", "governance"]
domain: "docs"
lifecycle: "ga"
---

# Rustdoc and GitHub Wiki Documentation Strategy

This repository uses a split documentation model aligned with Diataxis:

- `rustdoc` is the authoritative source for code-level API reference.
- The GitHub Wiki (`wiki/` submodule) is the high-level documentation surface for tutorials, how-to guides, and explanations.
- `docs/` remains the repo-native governance/operations surface for documentation contracts, SOPs, ADRs, and tooling reference (validated by `cargo xtask docs`).

## Diataxis Mapping

| Content type | Primary surface | Source of truth |
| --- | --- | --- |
| Reference (Rust APIs) | Generated rustdoc HTML | Rust source comments (`///`, `//!`) |
| Tutorials | GitHub Wiki | `wiki/*.md` |
| How-to guides | GitHub Wiki | `wiki/*.md` |
| Explanations | GitHub Wiki | `wiki/*.md` |
| ADR / SOP / docs governance | `docs/` (repo-native Markdown) | `docs/*.md` |

## Rustdoc Authoring Conventions (Required)

- Document crates and modules with `//!`.
- Document public items with `///` using clear intent-first summaries.
- Prefer describing behavior and invariants over repeating type signatures.
- Include runnable examples when the API is user-facing and executable in doctests.
- Use intra-doc links (for example, ``[`DesktopAction`]`` and ``[`reduce_desktop`]``) instead of raw text references.
- Keep docs aligned with visibility: public API docs are required; internal-only details are documented when they materially affect maintainability.

## GitHub Wiki Conventions (Required)

- Wiki repository is integrated as the `wiki/` git submodule.
- `Home.md` describes the Diataxis split and links to category indexes.
- `_Sidebar.md` is maintained as the canonical navigation entrypoint.
- Tutorial/how-to/explanation pages are separated by intent (no mixed-purpose pages).
- Wiki pages link to rustdoc for API signatures and item details instead of duplicating reference content.

## Synchronization Rules

When code changes:

1. Update rustdoc for changed public APIs (crate/module/item docs, links, examples).
2. Update affected wiki pages when workflows, usage guidance, or rationale changed.
3. Update repo governance/SOP/reference pages under `docs/` when process, tooling, or contracts changed.

When wiki content changes:

1. Confirm rustdoc links still point to valid crates/modules/items.
2. Update the `wiki/` submodule pointer in the main repo PR.
3. Keep repo-native pointer pages in `docs/tutorials`, `docs/how-to`, and `docs/explanation` in sync only when the canonical wiki page name/path changes (to avoid duplicate procedural content).

## Wiki Submodule Maintenance Workflow (Required)

Use a consistent submodule refresh flow before authoring or validating wiki content:

```bash
git submodule sync --recursive
git submodule update --init --recursive
```

Before editing wiki pages:

```bash
git -C wiki status --short
git -C wiki fetch origin
```

- If `wiki/` is on a local branch, fast-forward with `git -C wiki pull --ff-only`.
- If `wiki/` is detached (common after submodule updates), switch to a local branch tracking the wiki default branch before committing wiki edits.

Commit and review workflow for wiki edits:

1. Commit changes inside `wiki/`.
2. Stage the `wiki/` submodule pointer in the main repo.
3. Include both in the same PR as related code and rustdoc/docs changes.

## Migrated Documentation Surface

The existing maintainer-facing documentation workflow pages have been migrated into the wiki and are now canonical there:

- Tutorial: `wiki/Tutorial-First-Documentation-Change.md`
- How-to: `wiki/How-to-Update-Documentation-in-a-Pull-Request.md`
- Explanation: `wiki/Explanation-Documentation-Architecture-and-Governance.md`

Repo-native pages under `docs/tutorials`, `docs/how-to`, and `docs/explanation` remain as machine-validated pointers for discoverability and governance navigation.

## Review and Pull Request Requirements

- Documentation changes are reviewed in the same PR as related code changes.
- PR authors complete rustdoc/wiki checklist items in `.github/pull_request_template.md`.
- Reviewers verify:
  - API changes are reflected in rustdoc
  - Wiki pages are updated or explicitly marked N/A
  - `wiki/` submodule pointer is included when wiki content changes

## Validation

The local Rust toolchain validates both documentation layers:

- `cargo xtask docs wiki` (submodule + wiki structure checks)
- `cargo doc --workspace --no-deps` (`RUSTDOCFLAGS=-D warnings`)
- `cargo test --workspace --doc` (runnable rustdoc examples)
- `cargo xtask docs all` (docs contracts + links + OpenAPI + Mermaid + wiki checks)

Local bootstrap for the wiki submodule:

```bash
git submodule sync --recursive
git submodule update --init --recursive
```
