---
title: "How to Update Documentation in a Pull Request"
category: "how-to"
owner: "site-owner"
status: "active"
last_reviewed: "2026-02-26"
audience: ["engineering", "platform"]
invariants:
  - "Behavioral code changes require documentation updates in the same pull request."
  - "Frontmatter metadata must remain complete and valid."
tags: ["workflow", "pull-request"]
domain: "docs"
lifecycle: "ga"
---

# How to Update Documentation in a Pull Request

## Purpose

Use this guide when code behavior, operational procedures, or architecture decisions change and documentation must be updated in the same pull request across the rustdoc/wiki/MkDocs split.

## Preconditions

- You know which behavior changed.
- You can identify the affected documentation surface (`rustdoc`, GitHub Wiki, MkDocs docs/ADR/SOP pages).
- Your branch contains the code change (if applicable).

## Procedure

1. Update rustdoc comments (`//!`, `///`) for any changed public Rust APIs.
2. Update relevant GitHub Wiki pages under `wiki/` for tutorial/how-to/explanation changes.
3. Update relevant MkDocs pages under `docs/` when contracts, SOPs, ADRs, CI, or governance changed.
4. Ensure MkDocs frontmatter fields are present and accurate for edited `docs/*.md` pages.
5. If architecture changed, create an ADR from [`docs/adr/ADR-0000-template.md`](../adr/ADR-0000-template.md).
6. If OpenAPI surface changed, update `docs/reference/openapi/`.
7. Initialize/update the wiki submodule (if needed):

```bash
git submodule update --init --recursive
```

8. Run:

```bash
python3 scripts/docs/validate_docs.py all
cargo doc --workspace --no-deps
cargo test --workspace --doc
```

9. If Mermaid diagrams changed and `mmdc` is installed, run:

```bash
python3 scripts/docs/validate_docs.py mermaid --require-renderer
```

10. Include the rustdoc/wiki/docs checklist items in the pull request template (including the `wiki/` submodule pointer when wiki pages changed).

## Failure Handling

- Missing/invalid wiki structure: update `wiki/Home.md`, `wiki/_Sidebar.md`, or required category pages and rerun `python3 scripts/docs/validate_docs.py wiki`.
- Rustdoc warnings/errors: fix docs comments, intra-doc links, or doctest examples and rerun rustdoc commands.
- Missing frontmatter fields: add required metadata and rerun validation.
- Stale `last_reviewed`: review the content and set a new date if still accurate.
- Broken internal links: correct paths or anchors before requesting review.

## Output

A pull request with code + rustdoc + wiki/MkDocs documentation changes reviewed together.
