---
title: "Tutorial: Make Your First Documentation Change"
category: "tutorial"
owner: "site-owner"
status: "active"
last_reviewed: "2026-02-26"
audience: ["engineering"]
invariants:
  - "Tutorials remain linear and finishable in 30 minutes or less."
  - "This tutorial does not replace reference or policy documentation."
tags: ["tutorial", "onboarding"]
domain: "docs"
lifecycle: "ga"
---

# Tutorial: Make Your First Documentation Change

Goal: make a small documentation update across the new documentation split (rustdoc and/or wiki), validate it locally, and prepare a pull request.

## What you will do

1. Update one rustdoc comment or one wiki page.
2. Run documentation validation (wiki + rustdoc doctests + docs contract checks).
3. Confirm links/examples resolve.
4. Prepare a PR with the documentation change.

## Prerequisites

- A local clone of this repository
- Rust toolchain available (`cargo`)

## Steps

1. Open [`docs/how-to/update-documentation-in-a-pr.md`](../how-to/update-documentation-in-a-pr.md).
2. Run `git submodule update --init --recursive` if `wiki/` is not present.
3. Make one small change in either:
   - a rustdoc comment in `crates/**.rs`, or
   - a wiki page under `wiki/`
4. Run:

```bash
cargo xtask docs all
cargo doc --workspace --no-deps
cargo test --workspace --doc
```

5. Review the output. Fix any validation failures.
6. If you changed `wiki/`, ensure the submodule pointer is included in the main repo PR.
7. Commit your change with the documentation update in the same commit or PR as related code changes.

## What just happened

You used the same validation and review path as production changes. This is the core documentation-as-code workflow for the rustdoc + GitHub Wiki strategy.
