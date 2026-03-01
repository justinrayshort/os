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
  - "Canonical UI slices use deterministic scene seeding, viewport definitions, and artifact naming."
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
- `artifacts/style/`
- `artifacts/timing/`
- `artifacts/logs/`
- `artifacts/network/`
- `artifacts/traces/`
- `artifacts/diffs/`

The schema version is now `2`. Each slice entry records:

- `attempt`
- `failure_categories`
- `artifacts.style_snapshot`
- `artifacts.timing_snapshot`
- deterministic environment metadata
- timing metrics for navigation, readiness, artifact capture, and total slice duration

The run summary also records failure-category counts, flaky-slice counts, and retry-success counts.

## Canonical Browser Scene Contract

The browser-hosted shell now exposes a deterministic scene contract through query parameters:

- `?e2e-scene=<scene-id>`
- `e2e-skin=soft-neumorphic`
- `e2e-high-contrast=true|false`
- `e2e-reduced-motion=true|false`

Canonical scene ids for the blocking neumorphic workflow:

- `shell-default`
- `shell-context-menu-open`
- `settings-appearance`
- `settings-accessibility`
- `start-button-hover`
- `start-button-focus`
- `shell-high-contrast`
- `shell-reduced-motion`
- `ui-showcase-controls`
- `terminal-default`

Readiness is defined by both:

- DOM sentinel: `[data-ui-kind="desktop-root"][data-e2e-ready="true"]`
- performance mark: `os:e2e-ready`

Canonical validation runs wait on those readiness signals instead of arbitrary sleeps.

## Standardized Environment

Canonical neumorphic runs use:

- Chromium for the blocking browser path
- `colorScheme = light`
- `reducedMotion = reduce`
- fixed epoch `2026-01-01T12:00:00Z`
- deterministic `Math.random`
- motion freezing through an injected stylesheet
- single-worker execution for baseline-bearing profiles

Viewport definitions are explicit and stable:

- `desktop`: `1440x900`, `deviceScaleFactor=1`
- `tablet`: `1024x768`, `deviceScaleFactor=1`
- `mobile`: `390x844`, `deviceScaleFactor=1`

## Canonical Slice Naming

Canonical slice ids use stable dot-separated names:

- `shell.soft-neumorphic.default`
- `shell.soft-neumorphic.context-menu-open`
- `shell.soft-neumorphic.start-button-hover`
- `shell.soft-neumorphic.start-button-focus`
- `shell.soft-neumorphic.high-contrast`
- `shell.soft-neumorphic.reduced-motion`
- `settings.desktop.appearance-tab`
- `settings.desktop.accessibility-tab`
- `system.ui-showcase.controls`
- `terminal.desktop.default`

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
- `style.json`
- `manifest.json`

Do not edit baseline files manually. Generate candidate artifacts with `cargo e2e run`, review them with `cargo e2e inspect`, then promote them with `cargo e2e promote`.

## Diff Policy

The default browser workflow uses hybrid comparison:

- tolerance-based screenshot comparison using `pixelmatch`
- normalized DOM comparison
- normalized accessibility snapshot comparison
- deterministic layout metrics comparison
- deterministic style/token snapshot comparison

Canonical screenshot settings:

- `threshold = 0.10`
- `includeAA = false`

Pixel failure thresholds:

- `desktop`: fail above `0.0010`
- `tablet`: fail above `0.0010`
- `mobile`: fail above `0.0015`

Structured artifacts remain exact-match comparisons for DOM, accessibility, layout, and style snapshots.

## Failure Taxonomy

Every failure now records both a `code` and a `category`. Supported categories are:

- `environment-misconfiguration`
- `orchestration-startup`
- `readiness-timeout`
- `race-condition`
- `ui-contract-violation`
- `visual-regression`
- `javascript-runtime`
- `network-failure`
- `baseline-missing`
- `flaky`

`cargo e2e inspect` groups failures by category and prints the most relevant artifact path for each group.

## Iteration Loop

1. Implement the UI change.
2. Capture a focused slice without diff enforcement when you need a new candidate baseline:

   ```bash
   cargo e2e run --profile local-dev --scenario ui.neumorphic.layout --slice shell.soft-neumorphic.default --no-diff
   ```

3. Inspect the resulting manifest:

   ```bash
   cargo e2e inspect --run <run-id>
   ```

4. Promote the accepted baseline:

   ```bash
   cargo e2e promote --profile local-dev --scenario ui.neumorphic.layout --slice shell.soft-neumorphic.default --source-run <run-id>
   ```

5. Validate the promoted slice in headless mode:

   ```bash
   cargo e2e run --profile ci-headless --scenario ui.neumorphic.layout --slice shell.soft-neumorphic.default
   ```

6. If the change is broader than one slice, rerun the relevant blocking scenario family:

   - `ui.neumorphic.navigation`
   - `ui.neumorphic.interaction`
   - `ui.neumorphic.accessibility`
   - `ui.neumorphic.apps`

## Related Files

- [`xtask/src/commands/e2e.rs`](../../xtask/src/commands/e2e.rs)
- [`xtask/src/runtime/artifacts.rs`](../../xtask/src/runtime/artifacts.rs)
- [`tools/automation/e2e_profiles.toml`](../../tools/automation/e2e_profiles.toml)
- [`tools/automation/e2e_scenarios.toml`](../../tools/automation/e2e_scenarios.toml)
- [`tools/e2e/src/run.mjs`](../../tools/e2e/src/run.mjs)
