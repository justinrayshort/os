---
title: "Documentation Toolchain and Local Verification Pipeline"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-27"
audience: ["platform", "engineering"]
invariants:
  - "Local docs validation fails on broken links, invalid contracts, or invalid diagrams."
  - "Documentation automation runs through in-repo Rust tooling (`cargo xtask docs`)."
tags: ["reference", "ci", "tooling"]
domain: "docs"
lifecycle: "ga"
---

# Documentation Toolchain and Local Verification Pipeline

## Authoring Formats

- Markdown (`.md`) for docs pages
- Markdown (`.md`) for GitHub Wiki pages (via `wiki/` submodule)
- Rust doc comments (`//!`, `///`) for API reference generation
- OpenAPI YAML/JSON under `docs/reference/openapi/`
- Mermaid blocks in Markdown and `.mmd` assets
- ADRs as Markdown with frontmatter

## Build System

- `cargo xtask docs` (Rust-native docs contract/audit validator)
- `rustdoc` (`cargo doc --workspace --no-deps`) for API reference output
- GitHub Wiki repository integrated as `wiki/` git submodule

## Local Validation Stages

1. Wiki submodule structure validation (`cargo xtask docs wiki`)
2. Frontmatter + contract validation
3. OpenAPI parse/sanity validation
4. Mermaid structural validation
5. Broken internal reference detection
6. Typed app-state envelope boundary enforcement (`cargo xtask docs storage-boundary`) for
   `crates/apps`, `crates/desktop_runtime`, and `crates/site` (direct low-level envelope load calls are disallowed)
7. Rustdoc build (`cargo doc --workspace --no-deps`, `RUSTDOCFLAGS=-D warnings`)
8. Rustdoc doctests (`cargo test --workspace --doc`)
9. Audit report generation (`cargo xtask docs audit-report --output ...`) when needed

## Entry Points

- Local full validation:

```bash
cargo xtask docs all
cargo doc --workspace --no-deps
cargo test --workspace --doc
```

`cargo xtask docs all` already includes wiki structure checks and storage-boundary enforcement.

- Staged diagnostics (optional when you want isolated failures):

```bash
cargo xtask docs wiki
cargo xtask docs storage-boundary
```

- Full workspace verification (includes docs audit + optional clippy/trunk stages):

```bash
cargo verify
```

- Generate audit report:

```bash
cargo xtask docs audit-report --output .artifacts/docs-audit.json
```

- Convenience wrappers for docs checks and project verification are documented in [Project Command Entry Points](project-command-entrypoints.md).

## Hosted CI Status

GitHub Actions workflows for docs validation/audit are decommissioned in this repository. Documentation verification and audit generation are run locally via Cargo/`xtask`, and artifacts (for example `.artifacts/docs-audit.json`) can be attached to reviews or releases as needed.
