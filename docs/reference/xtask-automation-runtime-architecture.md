---
title: "xtask Automation Runtime Architecture"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-28"
audience: ["platform", "engineering"]
invariants:
  - "The xtask binary remains a thin CLI layer over library-backed command modules and shared runtime services."
  - "Workflow-specific policy lives under xtask/src/commands/, while cross-cutting orchestration services live under xtask/src/runtime/."
tags: ["reference", "xtask", "automation", "tooling"]
domain: "docs"
lifecycle: "ga"
---

# xtask Automation Runtime Architecture

This page documents the internal architecture of the `xtask` crate after the automation-runtime refactor.

## Module Boundaries

### CLI Surface

- [`xtask/src/bin/xtask.rs`](../../xtask/src/bin/xtask.rs): process entrypoint
- [`xtask/src/cli/mod.rs`](../../xtask/src/cli/mod.rs): top-level command parsing and help text

Responsibilities:

- top-level command selection
- stable usage text
- delegation into command families

Non-responsibilities:

- process spawning
- artifact layout
- workflow execution
- config loading

### Shared Runtime

- [`xtask/src/runtime/context.rs`](../../xtask/src/runtime/context.rs): shared `CommandContext`
- [`xtask/src/runtime/process.rs`](../../xtask/src/runtime/process.rs): `ProcessRunner`
- [`xtask/src/runtime/lifecycle.rs`](../../xtask/src/runtime/lifecycle.rs): shared process lifecycle, signaling, and port-readiness helpers
- [`xtask/src/runtime/workspace.rs`](../../xtask/src/runtime/workspace.rs): `WorkspaceState` for git/cargo workspace inspection
- [`xtask/src/runtime/workflow.rs`](../../xtask/src/runtime/workflow.rs): `WorkflowRecorder`, stage timing, structured automation manifests
- [`xtask/src/runtime/artifacts.rs`](../../xtask/src/runtime/artifacts.rs): `ArtifactManager`
- [`xtask/src/runtime/env.rs`](../../xtask/src/runtime/env.rs): environment normalization (`NO_COLOR`)
- [`xtask/src/runtime/config.rs`](../../xtask/src/runtime/config.rs): generic TOML-backed `ConfigLoader<T>`
- [`xtask/src/runtime/error.rs`](../../xtask/src/runtime/error.rs): structured `XtaskError`

Responsibilities:

- workspace-root resolution
- child-process execution
- managed process lifecycle and port readiness
- git/cargo workspace inspection
- environment normalization
- workflow manifests and stage/event recording
- artifact path policy
- typed configuration loading
- consistent error categories

### Command Domains

- [`xtask/src/commands/dev.rs`](../../xtask/src/commands/dev.rs): dev command-family façade and shared command structs
- [`xtask/src/commands/dev/`](../../xtask/src/commands/dev/): cohesive dev submodules for typed config, doctor checks, managed dev-server lifecycle, and web/Tauri entrypoints
- [`xtask/src/commands/verify.rs`](../../xtask/src/commands/verify.rs): verify command-family façade and shared command structs
- [`xtask/src/commands/verify/`](../../xtask/src/commands/verify/): cohesive verify submodules for profile/config handling, changed-scope detection, flow execution, and verification stage orchestration
- [`xtask/src/commands/perf/`](../../xtask/src/commands/perf/): cohesive perf submodules for CLI args, tooling checks, benchmark/profiling execution, and report generation
- [`xtask/src/commands/wiki.rs`](../../xtask/src/commands/wiki.rs): wiki submodule status/sync workflow management
- [`xtask/src/commands/docs/`](../../xtask/src/commands/docs/): docs command family façade and runtime integration
- [`xtask/src/docs.rs`](../../xtask/src/docs.rs): docs validator module root and command dispatch
- [`xtask/src/docs/`](../../xtask/src/docs/): split docs validation surfaces (`structure`, `wiki`, `frontmatter`, `sop`, `links`, `mermaid`, `openapi`, `storage_boundary`, `app_contract`, `ui_conformance`, `audit`)

Responsibilities:

- typed option parsing for a workflow family
- workflow-specific validation and sequencing
- use of shared runtime services instead of ad hoc helpers

## Stable Internal APIs

The refactor established these extension points for future workflows:

- `xtask::XtaskCommand`
- `CommandContext`
- `ProcessRunner`
- `WorkspaceState`
- `WorkflowRecorder`
- `ArtifactManager`
- `ConfigLoader<T>`
- `XtaskError`

New workflow families should build on these APIs instead of introducing command-specific process or artifact helpers.

## Configuration Ownership

Versioned automation configuration lives under [`tools/automation/`](../../tools/automation/):

- [`tools/automation/dev_server.toml`](../../tools/automation/dev_server.toml): managed dev-server defaults and artifact paths
- [`tools/automation/verify_profiles.toml`](../../tools/automation/verify_profiles.toml): verification profile definitions

Guidelines:

- use typed TOML-backed config for versioned workflow policy
- validate semantics after deserialization
- keep per-workflow defaults explicit and reviewable

## Workflow Recording and Artifacts

Structured workflow artifacts live under:

- [`.artifacts/automation/runs/`](../../.artifacts/automation/runs/)

Each workflow run writes:

- `manifest.json`
- `events.jsonl`

Event vocabulary:

- `workflow_started`
- `stage_started`
- `stage_finished`
- `workflow_finished`

Use this artifact stream for postmortem debugging, future automation dashboards, and consistency across new workflow families.

## Error Handling Standard

All new xtask-facing errors should use `XtaskError` categories:

- `config`
- `environment`
- `process launch`
- `process exit`
- `validation`
- `io`
- `unsupported platform`

Errors should include operation/path/hint metadata when possible so CLI failures remain actionable.

## How To Extend xtask

1. Add or extend a command family under `xtask/src/commands/`.
2. Parse arguments into a typed options struct.
3. Load versioned config from `tools/automation/` when workflow policy is repo-managed.
4. Execute through `CommandContext` services rather than spawning commands ad hoc.
5. Use `WorkflowRecorder` for multi-stage workflows.
6. Add focused unit tests under the owning command/runtime module.
7. Update [Project Command Entry Points](project-command-entrypoints.md) when operator-visible behavior changes.
