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

- `crates/system_shell_contract`
- `crates/system_shell`
- `crates/desktop_runtime/src/shell.rs`
- `crates/apps/terminal`

## Architecture Split

- `system_shell_contract`: defines segmented command paths, parser payloads, structured command data, completion payloads, errors, and stream events.
- `system_shell`: owns command registration, parsing, hierarchical lookup, pipeline execution, cancellation, and per-session event streams.
- `desktop_runtime`: owns built-in system commands and the runtime bridge to reducer actions and platform storage.
- `desktop_app_terminal`: renders typed transcript state and forwards browser input into a runtime-created shell session.

## Command Naming and Grammar

The terminal now uses a mixed command model:

- hierarchical commands use space-delimited namespace/action paths (`theme show`, `windows focus`, `data where`)
- familiar shell verbs remain single-token where expected (`pwd`, `cd`, `ls`, `open`, `clear`)

Shell input is parsed as:

- one or more pipeline stages separated by `|`
- each stage resolved against the longest matching registered command path
- remaining tokens parsed as typed literals and options

Namespace prefixes are discoverable. Entering `theme` or `windows` without a leaf command returns structured subcommand help instead of a plain "command not found" error.

## Built-in Commands

The current runtime-owned command pack includes:

- `help list`
- `help show <command...>`
- `terminal clear` (alias: `clear`)
- `history list`
- `open`
- `apps list`
- `apps open`
- `windows list`
- `windows focus`
- `windows close`
- `windows minimize`
- `windows restore`
- `theme show`
- `theme set skin`
- `theme set high-contrast`
- `theme set reduced-motion`
- `config get`
- `config set`
- `inspect runtime`
- `inspect windows`
- `inspect storage`
- `pwd`
- `cd`
- `ls`
- `data select`
- `data where`
- `data sort`
- `data first`
- `data get`

`ls`, `windows list`, and `apps list` return table-shaped data. `theme show` and `inspect runtime` return record-shaped data. `pwd` returns a scalar string value. `data *` commands accept structured piped input and transform it.

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
- `Notice`
- `Progress`
- `Data`
- `Completed`
- `Cancelled`

Command handlers return `CommandResult` values with typed `StructuredData` payloads, optional notices, and an explicit display preference. The terminal app converts those events into persisted transcript entries rather than rendering directly from command handlers.

## Structured Data Model

Terminal command results are data-first rather than text-first.

Supported top-level result shapes:

- scalar values
- records
- lists
- tables
- empty results with notices only

The shell stack uses:

- project-owned serializable `StructuredData` contracts at the runtime/UI boundary
- Nushell `nu-protocol` values as the typed-value reference model inside the shell/runtime adapters
- `nu-table` fallback rendering for deterministic plain-text table output
- `nu-ansi-term` styling utilities for consistent fallback formatting

## UI Structure

The terminal UI now uses a single terminal-surface layout:

- transcript viewport and active prompt share the same `.terminal-screen`
- the active input line is rendered as the final row inside `.terminal-transcript`

Persistent toolbar buttons, run buttons, and status chrome are intentionally absent. Command discovery and utility actions remain command- and shortcut-driven (`help`, `clear`, `Ctrl+L`, `Ctrl+C`).

Transcript rendering is semantic rather than decorative:

- `Prompt` -> `.terminal-line-prompt`
- `Notice` -> `.terminal-line-status` / `.terminal-line-stderr`
- `Value` -> `.terminal-line-value`
- `Record` -> `.terminal-data-record`
- `Table` -> `.terminal-data-table` / `.terminal-table`
- `System` -> `.terminal-line-system`

The prompt is rendered inline as part of the terminal buffer rather than as a separate form composer. It includes structured context segments such as the current working directory and current prompt mode (`ready` / `running`) plus a separator glyph and a native text input, preserving browser caret/input behavior without placeholder-driven hints.

The command input uses `desktop_app_contract::window_primary_input_dom_id(window_id)` as its DOM id so the runtime host can restore keyboard focus when the terminal window opens or regains focus.

## Completion and Scroll Behavior

- `Tab` requests completions from the existing shell session contract.
- Single matches fill the input immediately.
- Multiple matches render in a compact overlay (`.terminal-completions`) inside the terminal surface so they visually read as part of the buffer.
- `Escape` dismisses the completion overlay.
- `ls` and `cd` completions prefer path-like candidates from the explorer backend.
- Transcript scrolling auto-follows new output only while the viewport is already at or near the bottom; manual review scroll position is preserved when the user scrolls upward.

## Persistence

Terminal state is persisted under `app.terminal` (`TERMINAL_STATE_NAMESPACE`) with schema version `3`.

Persisted fields:

- `cwd`
- `input`
- `transcript`
- `history_cursor`
- `active_execution`

Legacy schema `0`/`1` terminal transcripts using plain `lines: Vec<String>` are migrated into typed `System` transcript entries.
Legacy schema `2` transcript lines using `Stdout`/`Stderr`/`Status`/`Json` entries are migrated into typed `Notice` and `Data` entries.

## Host Boundary

The terminal does not call Tauri or browser host APIs directly.

Host-backed commands route through:

- `platform_host::load_pref_with` / `save_pref_with` via the configured prefs store
- `platform_host::AppStateStore` helpers via the configured app-state store
- `platform_host::ExplorerFsService` via the configured explorer service
- existing runtime reducer actions for app/window/theme control

This keeps browser and Tauri behavior aligned through the shared `platform_host` contract surface.
