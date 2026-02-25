---
title: "Documentation Ownership and Lifecycle States"
category: "reference"
owner: "architecture-owner"
status: "active"
last_reviewed: "2026-02-25"
audience: ["engineering", "platform"]
invariants:
  - "Every document has a declared owner."
  - "Lifecycle state is explicit and validated."
tags: ["reference", "ownership", "lifecycle"]
domain: "docs"
lifecycle: "ga"
---

# Documentation Ownership and Lifecycle States

## Owners (Controlled Vocabulary)

- `site-owner`
- `platform-team`
- `architecture-owner`

Owner validation is enforced by [`tools/docs/doc_contracts.json`](../../tools/docs/doc_contracts.json).

## Lifecycle Status Definitions

- `draft`: Not yet approved for production reliance.
- `active`: Current and approved.
- `deprecated`: Still present but scheduled for removal or replacement.
- `superseded`: Replaced by another document or procedure.

## Status Rules

- `deprecated` and `superseded` documents should include `superseded_by` when applicable.
- `active` documents must pass freshness checks.
- `sop` and `reference` documents should not remain `draft` for production procedures.

