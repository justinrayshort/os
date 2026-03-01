---
title: "OpenAPI Reference Assets"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-25"
audience: ["platform", "engineering"]
invariants:
  - "Public API contracts are source-controlled under this directory."
  - "OpenAPI specs are validated through local Rust-native documentation tooling."
tags: ["reference", "openapi"]
domain: "docs"
lifecycle: "ga"
---

# OpenAPI Reference Assets

Place OpenAPI specifications (`.yaml`, `.yml`, `.json`) in this directory.

- Machine validation runs locally through `cargo xtask docs openapi` and `cargo xtask docs all`.
- Reference pages should link to generated or human-written endpoint documentation.

Current placeholder spec: [`site-api.openapi.yaml`](site-api.openapi.yaml)
