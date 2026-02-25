---
title: "Tutorial: Make Your First Documentation Change"
category: "tutorial"
owner: "site-owner"
status: "active"
last_reviewed: "2026-02-25"
audience: ["engineering"]
invariants:
  - "Tutorials remain linear and finishable in 30 minutes or less."
  - "This tutorial does not replace reference or policy documentation."
tags: ["tutorial", "onboarding"]
domain: "docs"
lifecycle: "ga"
---

# Tutorial: Make Your First Documentation Change

Goal: make a small docs update, validate it locally, and prepare a pull request.

## What you will do

1. Edit a single markdown file.
2. Add or update frontmatter metadata.
3. Run documentation validation.
4. Confirm links resolve.

## Prerequisites

- A local clone of this repository
- `python3` available

## Steps

1. Open [`docs/how-to/update-documentation-in-a-pr.md`](../how-to/update-documentation-in-a-pr.md).
2. Change one sentence or fix one typo.
3. Run:

```bash
python3 scripts/docs/validate_docs.py all
```

4. Review the output. Fix any validation failures.
5. Commit your change with the documentation update in the same commit or PR as related code changes.

## What just happened

You used the same validation and review path as production changes. This is the core documentation-as-code workflow.

