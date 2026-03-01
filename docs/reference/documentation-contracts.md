---
title: "Documentation Contracts"
category: "reference"
owner: "architecture-owner"
status: "active"
last_reviewed: "2026-02-26"
audience: ["engineering", "platform"]
invariants:
  - "Every markdown document under docs/ includes required frontmatter."
  - "Category metadata matches Diataxis folder placement."
tags: ["reference", "contracts", "frontmatter"]
domain: "docs"
lifecycle: "ga"
---

# Documentation Contracts

## Required Frontmatter Fields

- `title` (string)
- `category` (string)
- `owner` (string)
- `status` (string)
- `last_reviewed` (ISO date `YYYY-MM-DD`)
- `audience` (array of strings)
- `invariants` (array of strings)

## Allowed Categories

- `tutorial`
- `how-to`
- `reference`
- `explanation`
- `adr`
- `sop`

## Allowed Status Values

- `draft`
- `active`
- `deprecated`
- `superseded`

## Review Freshness Policy

- Documents fail docs validation when `last_reviewed` is older than `180` days (configurable via `DOCS_STALE_REVIEW_DAYS` or contract config).

## Diataxis Folder Mapping

- `docs/tutorials/**` -> `category: tutorial`
- `docs/how-to/**` -> `category: how-to`
- `docs/reference/**` -> `category: reference`
- `docs/explanation/**` -> `category: explanation`
- `docs/adr/**` -> `category: adr`
- `docs/sop/**` -> `category: sop`

## SOP Heading Contract

SOP documents in `docs/sop/` must contain these sections in order:

1. `Title & Purpose`
2. `Scope`
3. `Roles & Responsibilities`
4. `Prerequisites`
5. `Step-by-Step Procedure`
6. `Visual Aids`
7. `Invariants (Critical Section)`
8. `Validation Checklist`
9. `Version History`

## Source of Truth

- Docs page contracts (frontmatter/category/SOP structure) are defined in [`tools/docs/doc_contracts.json`](../../tools/docs/doc_contracts.json).
- External wiki structure contracts are enforced by the Rust `xtask` docs validator ([`xtask/src/docs.rs`](../../xtask/src/docs.rs), `cargo xtask docs wiki`).
- Rust API reference contract is enforced through rustdoc/lints in the Rust crates plus local rustdoc build/doctest steps.
- Wiki authoring and review require the external wiki repository branch and commit SHA to be recorded in PR or review notes whenever canonical wiki content changes.
