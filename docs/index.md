---
title: "Documentation System"
category: "explanation"
owner: "architecture-owner"
status: "active"
last_reviewed: "2026-02-25"
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

## Doctrine

- Documentation is a first-class system artifact.
- Documentation changes are reviewed alongside code changes.
- Documents must declare ownership, status, and invariants.
- Structure is constrained by category (`tutorial`, `how-to`, `reference`, `explanation`, `adr`, `sop`).

## Canonical Information Architecture

- `docs/tutorials`: learning-oriented paths with complete beginner wins
- `docs/how-to`: task-oriented operational procedures
- `docs/reference`: precise, mechanically accurate facts and contracts
- `docs/explanation`: rationale, tradeoffs, and architectural context
- `docs/adr`: architecture decision records
- `docs/sop`: controlled operational procedures and templates
- `docs/assets`: source-controlled diagrams and shared assets

## Start Here

- Read the operating model in [`docs/sop/documentation-system-sop.md`](sop/documentation-system-sop.md).
- Use [`docs/sop/sop-template.md`](sop/sop-template.md) for new SOPs.
- Use [`docs/adr/ADR-0000-template.md`](adr/ADR-0000-template.md) for structural decisions.
- Follow [`docs/how-to/update-documentation-in-a-pr.md`](how-to/update-documentation-in-a-pr.md) for day-to-day changes.

