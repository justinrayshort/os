---
title: "Explanation: Documentation Architecture and Governance"
category: "explanation"
owner: "architecture-owner"
status: "active"
last_reviewed: "2026-02-25"
audience: ["engineering", "platform"]
invariants:
  - "Description of the documentation system remains separate from operational procedures."
  - "Architectural rationale is preserved independently from task execution steps."
tags: ["explanation", "governance", "diataxis"]
domain: "docs"
lifecycle: "ga"
---

# Explanation: Documentation Architecture and Governance

## Why Diataxis is enforced

Diataxis prevents category drift. Without hard boundaries, reference pages become tutorials, SOPs become explanations, and operational reliability degrades because users cannot tell whether a page is normative or informative.

## Why Documentation-as-Code is enforced

Documentation failures are production failures when they cause incorrect operations, invalid assumptions, or delayed recovery. Co-locating docs with code ensures behavioral changes and knowledge changes travel together.

## Why frontmatter contracts exist

Ownership, lifecycle status, and invariants are governance signals. They enable automated freshness checks, review routing, and long-horizon maintainability.

## Failure modes this system addresses

- Orphaned procedures with no owner
- Stale recovery docs that still pass manual review
- Mixed intent pages (tutorial + reference + explanation in one file)
- Broken internal links after refactors
- Diagram drift when screenshots replace source diagrams

## Evolution strategy

- Add generated references as the codebase grows (OpenAPI, CLI help, schema exports).
- Expand ownership vocabulary and CODEOWNERS mapping when teams formalize.
- Feed audit reports into dashboards for trend analysis.

## Architecture Diagram

```mermaid
flowchart LR
  A["Code Change"] --> B["Docs Change in Same PR"]
  B --> C["Contract Validation"]
  C --> D["Lint + Links + Diagrams + OpenAPI"]
  D --> E["MkDocs Build"]
  E --> F["Merge Gate"]
  F --> G["Quarterly Audit Report"]
```

