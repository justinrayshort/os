---
title: "ADR-0001 Documentation System Adoption"
category: "adr"
owner: "architecture-owner"
status: "active"
last_reviewed: "2026-02-25"
audience: ["engineering", "platform"]
invariants:
  - "Repository documentation is classified by intent using Diataxis."
  - "Documentation governance is enforced by local Rust-native validation and review."
tags: ["adr", "documentation", "governance"]
domain: "architecture"
lifecycle: "ga"
---

# ADR-0001 Documentation System Adoption

## Status

Accepted

## Context

The project requires durable documentation that remains accurate over long horizons, survives personnel turnover, and scales with system complexity. Unstructured markdown and ad hoc review do not provide sufficient control for operational procedures and architecture rationale.

## Decision

Adopt a documentation system that combines Diataxis structure, documentation-as-code workflows, and machine-enforced governance contracts (owner, status, invariants, review freshness).

## Consequences

- Documentation changes become part of routine engineering review.
- Local validation workflow complexity increases slightly.
- Authors must classify content correctly and maintain frontmatter.
- Audit reports can be generated and tracked as artifacts.

## Alternatives Considered

- Wiki-based documentation outside the repository (rejected: version drift)
- Unstructured markdown without category enforcement (rejected: intent drift)
- Manual review only (rejected: insufficient reliability at scale)
