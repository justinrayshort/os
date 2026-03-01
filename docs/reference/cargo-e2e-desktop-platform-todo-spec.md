---
title: "Cargo E2E Desktop Platform TODO Spec"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-28"
audience: ["engineering", "platform"]
invariants:
  - "macOS remains a browser-E2E-first development host until Linux and Windows desktop-driver execution is validated end to end."
  - "Desktop E2E backend work for Linux and Windows must stay behind the Cargo-managed xtask surface and reuse the shared automation runtime."
tags: ["reference", "e2e", "xtask", "desktop", "webdriver", "platform-roadmap"]
domain: "docs"
lifecycle: "draft"
---

# Cargo E2E Desktop Platform TODO Spec

This page inventories the remaining work needed to take the Cargo-managed desktop E2E backend from staged implementation to validated Linux and Windows support, while keeping macOS as the stable browser-first development host.

## Current Stable Contract

- macOS is the supported day-to-day development host for Cargo-managed browser E2E:
  - `cargo e2e doctor`
  - `cargo e2e run --profile local-dev --scenario shell.boot`
  - `cargo e2e run --profile ci-headless`
  - `cargo e2e run --profile cross-browser`
- Desktop profiles (`tauri-linux`, `tauri-windows`) are versioned and parsed on macOS, but they must fail immediately with a clear unsupported-platform error.
- `cargo verify` and `cargo verify --profile ...` must remain green on macOS without requiring desktop-driver tooling.

## In-Scope Future Desktop Targets

- Linux:
  - `cargo e2e run --profile tauri-linux`
  - desktop smoke coverage via `tauri-driver` plus a native WebDriver bridge
- Windows:
  - `cargo e2e run --profile tauri-windows`
  - desktop smoke coverage via `tauri-driver` plus a native WebDriver bridge

## Linux TODOs

- Validate the current `tauri-driver` launch arguments and native WebDriver selection against a real Linux host.
- Confirm which native bridge is the supported default:
  - `geckodriver`
  - `chromedriver`
- Verify the desktop binary path and runtime dependencies for `cargo build -p desktop_tauri`.
- Exercise both staged desktop scenarios:
  - `desktop.boot`
  - `desktop.settings-navigation`
- Capture the desktop artifact contract:
  - driver logs
  - frontend-server logs
  - screenshots
  - report JSON
- Add Linux-host acceptance evidence to the docs and wiki.

## Windows TODOs

- Validate `tauri-driver` execution and native driver selection on Windows.
- Confirm path handling for:
  - `desktop_tauri.exe`
  - `where`-based native driver lookup
  - artifact/log paths
- Confirm the correct preferred native bridge on Windows:
  - `msedgedriver`
  - `chromedriver`
- Exercise both staged desktop scenarios:
  - `desktop.boot`
  - `desktop.settings-navigation`
- Add Windows-host acceptance evidence to the docs and wiki.

## Shared Backend TODOs

- Harden desktop-driver prerequisite checks in `cargo e2e doctor` with host-specific remediation text.
- Add xtask tests for desktop backend argument shaping where possible without depending on host drivers.
- Decide whether the desktop harness should remain Selenium-based or move behind a more structured Node runner abstraction.
- Verify deterministic teardown for:
  - frontend server
  - `tauri-driver`
  - desktop app process
- Decide whether desktop runs should emit trace-equivalent debugging artifacts or remain screenshot/report only.

## Acceptance Criteria For Desktop GA

- Linux desktop profile passes on a supported Linux host.
- Windows desktop profile passes on a supported Windows host.
- `cargo e2e doctor` clearly distinguishes:
  - browser readiness
  - desktop-driver readiness
  - unsupported-host state
- Desktop artifacts are written under `.artifacts/e2e/runs/<run-id>/` with deterministic paths.
- `cargo verify`, browser E2E, and the macOS unsupported-path behavior remain unchanged and green.

## Non-Goals

- macOS desktop-driver support in the current phase.
- Full screenshot-matrix parity for desktop before smoke coverage is stable.
- Replacing the browser E2E path with the desktop path as the default development workflow.
