---
title: "UI Feedback Automation and Baseline Workflow"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-03-01"
audience: ["engineering", "design"]
invariants:
  - "Cargo-managed UI feedback runs write a single machine-readable manifest per run."
  - "Approved baselines are updated only through `cargo e2e promote`."
  - "Canonical UI slices use deterministic viewport definitions and artifact naming."
tags: ["reference", "e2e", "playwright", "ui", "baselines", "xtask"]
domain: "frontend"
lifecycle: "ga"
---

# UI Feedback Automation and Baseline Workflow

This reference describes the Cargo-managed Playwright workflow used to capture deterministic UI feedback artifacts, inspect them programmatically, and promote approved baselines for later validation runs.

## Commands

- `cargo e2e run --profile <name> [--scenario <id>] [--slice <id>] [--debug] [--no-diff]`
- `cargo e2e inspect --run <path|run-id>`
- `cargo e2e promote --profile <name> [--scenario <id>] [--slice <id>] --source-run <path|run-id>`
- `cargo e2e doctor`
- `cargo e2e list`

## Run Artifacts

Each run writes under:

- `.artifacts/e2e/runs/<run-id>/`

Required machine-readable entrypoint:

- `reports/ui-feedback-manifest.json`

Artifact subdirectories:

- `artifacts/screenshots/`
- `artifacts/dom/`
- `artifacts/a11y/`
- `artifacts/layout/`
- `artifacts/logs/`
- `artifacts/network/`
- `artifacts/traces/`
- `artifacts/diffs/`

The manifest indexes every scenario/slice/browser/viewport result plus the artifact paths for that tuple.

## Canonical Slice Naming

Canonical slice ids use stable dot-separated names:

- `shell.soft-neumorphic.default`
- `shell.soft-neumorphic.context-menu-open`
- `shell.soft-neumorphic.start-button-hover`
- `settings.desktop.appearance-tab`

Artifact filenames normalize those ids into the safe stem:

- `<browser>--<scenario-id>--<slice-id>--<viewport-id>`

## Baselines

Approved baselines live under:

- `tools/e2e/baselines/<scenario-id>/<slice-id>/<browser>/<viewport-id>/`

Each baseline directory contains:

- `screenshot.png`
- `dom.json`
- `a11y.json`
- `layout.json`
- `manifest.json`

Do not edit baseline files manually. Generate candidate artifacts with `cargo e2e run`, review them with `cargo e2e inspect`, then promote them with `cargo e2e promote`.

## Diff Policy

The default browser workflow uses hybrid comparison:

- screenshot hash comparison for same-profile pixel validation
- normalized DOM comparison
- normalized accessibility snapshot comparison
- deterministic layout metrics comparison

When a baseline was promoted from a different profile, the workflow still records screenshot diffs for review but only fails on the structured DOM/accessibility/layout comparisons. This preserves a stable headed-debug loop while keeping CI validation deterministic.

## Iteration Loop

1. Implement the UI change.
2. Capture a focused slice without diff enforcement when you need a new candidate baseline:

   ```bash
   cargo e2e run --profile local-dev --scenario ui.shell.layout-baseline --slice shell.soft-neumorphic.default --no-diff
   ```

3. Inspect the resulting manifest:

   ```bash
   cargo e2e inspect --run <run-id>
   ```

4. Promote the accepted baseline:

   ```bash
   cargo e2e promote --profile local-dev --scenario ui.shell.layout-baseline --slice shell.soft-neumorphic.default --source-run <run-id>
   ```

5. Validate the promoted slice in headless mode:

   ```bash
   cargo e2e run --profile ci-headless --scenario ui.shell.layout-baseline --slice shell.soft-neumorphic.default
   ```

## Related Files

- [`xtask/src/commands/e2e.rs`](../../xtask/src/commands/e2e.rs)
- [`xtask/src/runtime/artifacts.rs`](../../xtask/src/runtime/artifacts.rs)
- [`tools/automation/e2e_profiles.toml`](../../tools/automation/e2e_profiles.toml)
- [`tools/automation/e2e_scenarios.toml`](../../tools/automation/e2e_scenarios.toml)
- [`tools/e2e/src/run.mjs`](../../tools/e2e/src/run.mjs)
