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

- The GitHub Wiki (`wiki/` submodule) is the canonical documentation hub and primary navigation surface for project documentation (Diataxis pages plus project reference/index pages).
- `rustdoc` is authoritative for Rust API reference.
- The repo-native `docs/` corpus is the canonical storage for formal governance/operations artifacts (contracts, ADRs, SOPs, tooling reference, and supporting assets).

## Doctrine

- Documentation is a first-class system artifact.
- Documentation changes are reviewed alongside code changes.
- Documents must declare ownership, status, and invariants.
- Structure is constrained by category (`tutorial`, `how-to`, `reference`, `explanation`, `adr`, `sop`).

## Canonical Hub-and-Artifact Model

- The GitHub Wiki is the canonical reader-facing entry point and navigation hub.
- Canonical artifact storage remains type-specific (`rustdoc` for Rust APIs, `docs/` for ADR/SOP/governance/assets).
- Wiki reference pages index and cross-link formal artifacts instead of duplicating them.
- Changes to formal artifacts should update both the canonical artifact and the relevant wiki registry/index page in the same review cycle.

## Canonical Information Architecture

- `rustdoc` (generated): Rust API reference (crate/module/item docs)
- `wiki/`: canonical documentation hub with Diataxis tutorials/how-to/explanations plus project reference indexes (architecture maps, diagrams, ADR/SOP registries, command and artifact catalogs)
- `docs/reference`: precise repo-native contracts, tooling, OpenAPI, and governance references that back wiki reference indexes
- `docs/tutorials`: maintainer onboarding pointers for the documentation workflow (canonical tutorials live in the wiki)
- `docs/how-to`: maintainer workflow pointers for the documentation system (canonical how-to content lives in the wiki)
- `docs/explanation`: documentation-system rationale (project/product explanations live in the wiki)
- `docs/adr`: architecture decision records
- `docs/sop`: controlled operational procedures and templates
- `docs/assets`: source-controlled diagrams and shared assets

## Start Here

- Start in `wiki/Home.md` for the canonical documentation hub and project-wide navigation.
- Read the operating model in [`docs/sop/documentation-system-sop.md`](sop/documentation-system-sop.md).
- Read the split-strategy policy in [`docs/reference/rustdoc-and-github-wiki-documentation-strategy.md`](reference/rustdoc-and-github-wiki-documentation-strategy.md).
- Use [`docs/sop/sop-template.md`](sop/sop-template.md) for new SOPs.
- Use [`docs/adr/ADR-0000-template.md`](adr/ADR-0000-template.md) for structural decisions.
- Follow [`docs/how-to/update-documentation-in-a-pr.md`](how-to/update-documentation-in-a-pr.md) for day-to-day changes.
