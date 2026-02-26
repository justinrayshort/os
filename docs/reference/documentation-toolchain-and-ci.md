---
title: "Documentation Toolchain and CI Pipeline"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-26"
audience: ["platform", "engineering"]
invariants:
  - "CI fails on broken links, invalid contracts, or invalid diagrams."
  - "Docs site builds from repository sources only."
tags: ["reference", "ci", "tooling"]
domain: "docs"
lifecycle: "ga"
---

# Documentation Toolchain and CI Pipeline

## Authoring Formats

- Markdown (`.md`) for docs pages
- Markdown (`.md`) for GitHub Wiki pages (via `wiki/` submodule)
- Rust doc comments (`//!`, `///`) for API reference generation
- OpenAPI YAML/JSON under `docs/reference/openapi/`
- Mermaid blocks in Markdown and `.mmd` assets
- ADRs as Markdown with frontmatter

## Build System

- `mkdocs` + Material theme (`mkdocs.yml`)
- `rustdoc` (`cargo doc --workspace --no-deps`) for API reference output
- GitHub Wiki repository integrated as `wiki/` git submodule

## CI Validation Stages

1. Markdown lint
2. Vale prose lint (MkDocs docs)
3. Wiki submodule structure validation (`python3 scripts/docs/validate_docs.py wiki`)
4. Frontmatter + contract validation
5. OpenAPI validation
6. Mermaid validation
7. Broken internal reference detection
8. Rustdoc build (`cargo doc --workspace --no-deps`, `RUSTDOCFLAGS=-D warnings`)
9. Rustdoc doctests (`cargo test --workspace --doc`)
10. Docs site build (`mkdocs build --strict`)

## Entry Points

- Local full validation:

```bash
python3 scripts/docs/validate_docs.py all
cargo doc --workspace --no-deps
cargo test --workspace --doc
```

- Generate audit report:

```bash
python3 scripts/docs/validate_docs.py audit-report --output .artifacts/docs-audit.json
```

- Convenience wrappers for docs checks and project verification are documented in [Project Command Entry Points](project-command-entrypoints.md).

## CI Workflows

- Pull request / push docs validation: `.github/workflows/docs.yml`
- Quarterly audit artifact generation: `.github/workflows/docs-audit.yml`

Both workflows checkout git submodules so the GitHub Wiki repository is present during validation.
