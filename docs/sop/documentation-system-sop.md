---
title: "SOP: Documentation System Operation and Governance"
category: "sop"
owner: "architecture-owner"
status: "active"
last_reviewed: "2026-02-26"
audience: ["engineering", "platform"]
invariants:
  - "Documentation is reviewed and versioned with code."
  - "Critical procedures preserve explicit invariants and validation checklists."
tags: ["sop", "documentation", "governance"]
domain: "docs"
lifecycle: "ga"
---

# SOP: Documentation System Operation and Governance

## 1. Title & Purpose

This SOP defines the procedure for authoring, validating, reviewing, and auditing project documentation across rustdoc, the GitHub Wiki submodule, and the MkDocs governance docs without violating documentation governance invariants.

## 2. Scope

- Covers: rustdoc API reference comments, GitHub Wiki tutorials/how-to/explanations, MkDocs governance docs, ADRs, SOPs, OpenAPI assets, Mermaid diagrams, and CI validation
- Does not cover: runtime application behavior itself, external third-party documentation sites, or non-versioned documentation channels

## 3. Roles & Responsibilities

| Role | Responsibility |
| --- | --- |
| Author | Creates or updates rustdoc/wiki/MkDocs documentation in the same review cycle as related changes |
| Reviewer | Verifies correctness, surface placement (rustdoc vs wiki vs MkDocs), and postconditions |
| Architecture Owner | Approves structural changes, ADRs, and governance deviations |
| Platform Team | Maintains CI/documentation tooling and audit workflows |

## 4. Prerequisites

- Repository checkout with `python3`
- Repository checkout with git submodules initialized (`wiki/`)
- Ability to run repository validation scripts
- Required ownership and status values known
- Related code/API/architecture changes identified (if applicable)

## 5. Step-by-Step Procedure

1. Classify the change by intent and documentation surface (`rustdoc`, GitHub Wiki, MkDocs docs/ADR/SOP).
   - Command:

   ```bash
   git submodule update --init --recursive
   ```

   - Expected output: `wiki/` submodule is present and up to date
   - Failure condition: wiki submodule missing or detached workflow not updated
2. Create or update the documentation in the correct place.
   - Command:

   ```bash
   $EDITOR crates/<crate>/src/<file>.rs   # rustdoc
   $EDITOR wiki/<Page>.md                 # GitHub Wiki
   $EDITOR docs/<category>/<file>.md      # MkDocs governance/ADR/SOP
   ```

   - Expected output: API reference docs are in rustdoc; tutorials/how-to/explanations are in wiki; governance docs remain in `docs/`
   - Failure condition: wrong surface, missing rustdoc update, or mixed intent page
3. Add/update related contracts (ADR, OpenAPI, diagrams) when architecture/API/process changes.
   - Command:

   ```bash
   python3 scripts/docs/validate_docs.py openapi
   ```

   - Expected output: OpenAPI assets validate or no specs are present
   - Failure condition: schema invalid or spec missing for a changed public API
4. Run documentation validation locally.
   - Command:

   ```bash
   python3 scripts/docs/validate_docs.py all
   cargo doc --workspace --no-deps
   cargo test --workspace --doc
   ```

   - Expected output: wiki/MkDocs validation and rustdoc build/doctests report success
   - Failure condition: wiki structure, frontmatter/link/SOP/diagram validation, rustdoc warnings, or doctests fail
5. Submit a pull request and complete documentation checklist items.
   - Command:

   ```bash
   git status --short
   ```

   - Expected output: code changes plus rustdoc/wiki/MkDocs updates (and `wiki/` submodule pointer when applicable) are included
   - Failure condition: behavioral code change ships without rustdoc/wiki updates or ADR requirement is skipped

## 6. Visual Aids

```mermaid
sequenceDiagram
  participant Author
  participant CI
  participant Reviewer
  participant Repo
  Author->>Repo: Push PR (code + rustdoc + wiki/MkDocs)
  Repo->>CI: Trigger docs workflow
  CI->>CI: Validate wiki + rustdoc + contracts/lints/links/OpenAPI/Mermaid
  CI-->>Reviewer: Pass/Fail status
  Reviewer->>Repo: Approve or request changes
```

## 7. Invariants (Critical Section)

- Documentation remains in the same repository as source code.
- Rust API reference remains generated from rustdoc, not duplicated as hand-maintained Markdown reference.
- Tutorials/how-to/explanations are maintained in the GitHub Wiki (`wiki/` submodule).
- Critical procedures include explicit invariants and validation checklists.
- `category` matches folder placement.
- `owner`, `status`, and `last_reviewed` remain machine-validated.
- Diagrams are source-controlled (Mermaid) rather than screenshots.

## 8. Validation Checklist

- [ ] Frontmatter contract passes
- [ ] Wiki submodule and structure checks pass
- [ ] Rustdoc builds with no warnings
- [ ] Rustdoc examples/doctests pass
- [ ] Internal links resolve
- [ ] Mermaid blocks validate
- [ ] OpenAPI specs validate (if present/changed)
- [ ] MkDocs site builds
- [ ] PR checklist completed (docs updated / ADR if required)

## 9. Version History

| Version | Date | Author | Change |
| --- | --- | --- | --- |
| 1.1.0 | 2026-02-26 | Codex | Added rustdoc + GitHub Wiki split workflow, validation, and review requirements |
| 1.0.0 | 2026-02-25 | Codex | Initial documentation system SOP |
