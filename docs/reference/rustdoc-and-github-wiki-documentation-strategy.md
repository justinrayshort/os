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
git submodule update --init --recursive
```
