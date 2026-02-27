---
title: "System Terminal Command Interface"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-27"
audience: ["engineering", "platform"]
invariants:
  - "The terminal UI remains separate from the headless shell engine and from desktop/runtime host APIs."
  - "Terminal command execution routes through the runtime-owned shell registry and existing platform storage/host abstractions."
  - "Command metadata is the single source of truth for help output, completion labels, and shell registration identity."
tags: ["reference", "terminal", "shell", "desktop-runtime", "wasm"]
domain: "desktop"
lifecycle: "ga"
---

# System Terminal Command Interface

This page documents the browser-native terminal command surface implemented by the shared shell stack:

- `crates/shrs_core_headless`
- `crates/system_shell_contract`
- `crates/system_shell`
- `crates/desktop_runtime/src/shell.rs`
- `crates/apps/terminal`

## Architecture Split

- `shrs_core_headless`: parses shell input into argv without native TTY dependencies.
- `system_shell_contract`: defines command metadata, shell requests, completion payloads, errors, and stream events.
- `system_shell`: owns command registration, command lookup, async execution, cancellation, and per-session event streams.
- `desktop_runtime`: owns built-in system commands and the runtime bridge to reducer actions and platform storage.
- `desktop_app_terminal`: renders transcript state and forwards browser input into a runtime-created shell session.

## Built-in Commands

The initial runtime-owned command pack includes:

- `help`
- `clear`
- `history`
- `open`
- `apps.list`
- `apps.open`
- `windows.list`
- `windows.focus`
- `windows.close`
- `windows.minimize`
- `windows.restore`
- `theme.show`
- `theme.set.skin`
- `theme.set.high-contrast`
- `theme.set.reduced-motion`
- `config.get`
- `config.set`
- `inspect.runtime`
- `inspect.windows`
- `inspect.storage`
- `fs.pwd`
- `fs.cd`
- `fs.ls`

## Command Registration

Apps can register commands dynamically through `desktop_app_contract::CommandService`.

Rules:

- apps must request `AppCapability::Commands` unless they are privileged runtime apps
- app-scoped registrations must use the registering app's canonical id
- window-scoped registrations must use the current window id
- only privileged apps may register `Global` commands
- registration handles unregister on drop

## Stream Events

Command output is streamed into the terminal UI using `system_shell_contract::ShellStreamEvent`:

- `Started`
- `StdoutChunk`
- `StderrChunk`
- `Status`
- `Progress`
- `Json`
- `Completed`
- `Cancelled`

The terminal app converts those events into persisted transcript entries rather than rendering directly from command handlers.

## Persistence

Terminal state is persisted under `app.terminal` (`TERMINAL_STATE_NAMESPACE`) with schema version `2`.

Persisted fields:

- `cwd`
- `input`
- `transcript`
- `history_cursor`
- `active_execution`

Legacy schema `0`/`1` terminal transcripts using plain `lines: Vec<String>` are migrated into typed `System` transcript entries.

## Host Boundary

The terminal does not call Tauri or browser host APIs directly.

Host-backed commands route through:

- `platform_storage::load_pref_typed` / `save_pref_typed`
- `platform_storage::list_app_state_namespaces`
- `platform_storage::explorer_*`
- existing runtime reducer actions for app/window/theme control

This keeps browser and Tauri behavior aligned with the existing host adapter strategy.
