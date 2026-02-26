---
title: "Documentation System"
category: "explanation"
owner: "architecture-owner"
status: "active"
last_reviewed: "2026-02-26"
audience: ["engineering", "platform"]
invariants:
  - "Documentation is versioned in the same repository as source code."
  - "Operational and architectural knowledge must be reviewable in pull requests."
tags: ["documentation", "governance", "diataxis"]
domain: "docs"
lifecycle: "ga"
---

# Documentation System

This repository treats documentation as production infrastructure: versioned, review-gated, machine-validated, and classified by intent using Diataxis.

The project now uses a split documentation surface:

- `rustdoc` is authoritative for Rust API reference.
- The GitHub Wiki (`wiki/` submodule) is the primary surface for tutorials, how-to guides, and explanations.
- The repo-native `docs/` corpus is the governance/operations surface for contracts, ADRs, SOPs, and tooling reference.

## Doctrine

- Documentation is a first-class system artifact.
- Documentation changes are reviewed alongside code changes.
- Documents must declare ownership, status, and invariants.
- Structure is constrained by category (`tutorial`, `how-to`, `reference`, `explanation`, `adr`, `sop`).

## Canonical Information Architecture

- `rustdoc` (generated): Rust API reference (crate/module/item docs)
- `wiki/`: learning-oriented tutorials, task-oriented how-to guides, and explanatory content
- `docs/reference`: precise documentation-system contracts, tooling, and governance references
- `docs/tutorials`: maintainer onboarding pointers for the documentation workflow (canonical tutorials live in the wiki)
- `docs/how-to`: maintainer workflow pointers for the documentation system (canonical how-to content lives in the wiki)
- `docs/explanation`: documentation-system rationale (project/product explanations live in the wiki)
- `docs/adr`: architecture decision records
- `docs/sop`: controlled operational procedures and templates
- `docs/assets`: source-controlled diagrams and shared assets

## Start Here

- Read the operating model in [`docs/sop/documentation-system-sop.md`](sop/documentation-system-sop.md).
- Read the split-strategy policy in [`docs/reference/rustdoc-and-github-wiki-documentation-strategy.md`](reference/rustdoc-and-github-wiki-documentation-strategy.md).
- Use [`docs/sop/sop-template.md`](sop/sop-template.md) for new SOPs.
- Use [`docs/adr/ADR-0000-template.md`](adr/ADR-0000-template.md) for structural decisions.
- Follow [`docs/how-to/update-documentation-in-a-pr.md`](how-to/update-documentation-in-a-pr.md) for day-to-day changes.
