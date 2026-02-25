---
title: "Documentation Toolchain and CI Pipeline"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-25"
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
- OpenAPI YAML/JSON under `docs/reference/openapi/`
- Mermaid blocks in Markdown and `.mmd` assets
- ADRs as Markdown with frontmatter

## Build System

- `mkdocs` + Material theme (`mkdocs.yml`)

## CI Validation Stages

1. Markdown lint
2. Vale prose lint
3. Frontmatter + contract validation
4. OpenAPI validation
5. Mermaid validation
6. Broken internal reference detection
7. Docs site build (`mkdocs build --strict`)

## Entry Points

- Local full validation:

```bash
python3 scripts/docs/validate_docs.py all
```

- Generate audit report:

```bash
python3 scripts/docs/validate_docs.py audit-report --output .artifacts/docs-audit.json
```

- Convenience wrappers for docs checks and project verification are documented in [Project Command Entry Points](project-command-entrypoints.md).

## CI Workflows

- Pull request / push docs validation: `.github/workflows/docs.yml`
- Quarterly audit artifact generation: `.github/workflows/docs-audit.yml`
