---
title: "How to Update Documentation in a Pull Request"
category: "how-to"
owner: "site-owner"
status: "active"
last_reviewed: "2026-02-25"
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

Use this guide when code behavior, operational procedures, or architecture decisions change and documentation must be updated in the same pull request.

## Preconditions

- You know which behavior changed.
- You can identify the affected documentation category (`how-to`, `reference`, `explanation`, `adr`, or `sop`).
- Your branch contains the code change (if applicable).

## Procedure

1. Update the relevant document(s) under `docs/`.
2. Ensure frontmatter fields are present and accurate.
3. If architecture changed, create an ADR from [`docs/adr/ADR-0000-template.md`](../adr/ADR-0000-template.md).
4. If public API changed, update `docs/reference/openapi/`.
5. Run:

```bash
python3 scripts/docs/validate_docs.py all
```

6. If Mermaid diagrams changed and `mmdc` is installed, run:

```bash
python3 scripts/docs/validate_docs.py mermaid --require-renderer
```

7. Include the docs-related checklist items in the pull request template.

## Failure Handling

- Missing frontmatter fields: add required metadata and rerun validation.
- Stale `last_reviewed`: review the content and set a new date if still accurate.
- Broken internal links: correct paths or anchors before requesting review.

## Output

A pull request with code + documentation changes reviewed together.

