//! Terminal desktop app UI component backed by the browser-native shell session bridge.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

use std::{
    cell::Cell,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

use desktop_app_contract::AppServices;
use leptos::ev::KeyboardEvent;
use leptos::*;
use platform_storage::{self, TERMINAL_STATE_NAMESPACE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use system_shell_contract::{CompletionRequest, CompletionItem, ExecutionId, ShellRequest, ShellStreamEvent};

const TERMINAL_STATE_SCHEMA_VERSION: u32 = 2;
const MAX_TERMINAL_ENTRIES: usize = 200;
static NEXT_TERMINAL_INSTANCE_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct LegacyTerminalPersistedState {
    cwd: String,
    input: String,
    lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PersistedExecutionState {
    execution_id: ExecutionId,
    command: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum TerminalTranscriptEntry {
    Prompt {
        cwd: String,
        command: String,
        execution_id: Option<ExecutionId>,
    },
    Stdout {
        text: String,
        execution_id: ExecutionId,
    },
    Stderr {
        text: String,
        execution_id: ExecutionId,
    },
    Status {
        text: String,
        execution_id: ExecutionId,
    },
    Json {
        value: Value,
        execution_id: ExecutionId,
    },
    System {
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TerminalPersistedState {
    cwd: String,
    input: String,
    transcript: Vec<TerminalTranscriptEntry>,
    history_cursor: Option<usize>,
    active_execution: Option<PersistedExecutionState>,
}

fn migrate_terminal_state(
    schema_version: u32,
    envelope: &platform_storage::AppStateEnvelope,
) -> Result<Option<TerminalPersistedState>, String> {
    match schema_version {
        0 | 1 => {
            let legacy: LegacyTerminalPersistedState =
                platform_storage::migrate_envelope_payload(envelope)?;
            Ok(Some(TerminalPersistedState {
                cwd: legacy.cwd,
                input: legacy.input,
                transcript: legacy
                    .lines
                    .into_iter()
                    .map(|text| TerminalTranscriptEntry::System { text })
                    .collect(),
                history_cursor: None,
                active_execution: None,
            }))
        }
        _ => Ok(None),
    }
}

fn default_terminal_transcript() -> Vec<TerminalTranscriptEntry> {
    vec![
        TerminalTranscriptEntry::System {
            text: "RetroShell 0.2".to_string(),
        },
        TerminalTranscriptEntry::System {
            text: "Type `help` for commands.".to_string(),
        },
    ]
}

fn normalize_terminal_transcript(transcript: &mut Vec<TerminalTranscriptEntry>) {
    if transcript.is_empty() {
        *transcript = default_terminal_transcript();
        return;
    }

    if transcript.len() > MAX_TERMINAL_ENTRIES {
        let overflow = transcript.len() - MAX_TERMINAL_ENTRIES;
        transcript.drain(0..overflow);
    }
}

fn restore_terminal_state(
    mut restored: TerminalPersistedState,
    launch_cwd: &str,
) -> TerminalPersistedState {
    if restored.cwd.trim().is_empty() {
        restored.cwd = launch_cwd.to_string();
    }
    if restored.active_execution.is_some() {
        restored.active_execution = None;
        restored.transcript.push(TerminalTranscriptEntry::System {
            text: "Previous command interrupted during restore.".to_string(),
        });
    }
    normalize_terminal_transcript(&mut restored.transcript);
    restored
}

fn render_transcript_entry(entry: &TerminalTranscriptEntry) -> String {
    match entry {
        TerminalTranscriptEntry::Prompt { cwd, command, .. } => format!("{cwd}> {command}"),
        TerminalTranscriptEntry::Stdout { text, .. } => text.clone(),
        TerminalTranscriptEntry::Stderr { text, .. } => text.clone(),
        TerminalTranscriptEntry::Status { text, .. } => text.clone(),
        TerminalTranscriptEntry::Json { value, .. } => {
            serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
        }
        TerminalTranscriptEntry::System { text } => text.clone(),
    }
}

fn completion_request(cwd: &str, line: &str) -> CompletionRequest {
    CompletionRequest {
        cwd: cwd.to_string(),
        line: line.to_string(),
        argv: line
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>(),
        cursor: line.len(),
        source_window_id: None,
    }
}

#[component]
/// Terminal app window contents.
///
/// This component presents a browser-native shell backed by runtime-owned commands and persists
/// transcript state via [`platform_storage`].
pub fn TerminalApp(
    /// App launch parameters (for example, the initial working directory).
    launch_params: Value,
    /// Manager-restored app state payload for this window instance.
    restored_state: Option<Value>,
    /// Optional app-host bridge for manager-owned commands.
    services: Option<AppServices>,
) -> impl IntoView {
    let input_id = format!(
        "retro-shell-input-{}",
        NEXT_TERMINAL_INSTANCE_ID.fetch_add(1, Ordering::Relaxed)
    );
    let launch_cwd = launch_params
        .get("cwd")
        .and_then(Value::as_str)
        .unwrap_or("~/desktop")
        .to_string();
    let shell_session = services
        .as_ref()
        .and_then(|services| services.commands.create_session(launch_cwd.clone()).ok());
    let cwd = create_rw_signal(launch_cwd.clone());
    let input = create_rw_signal(String::new());
    let transcript = create_rw_signal(default_terminal_transcript());
    let suggestions = create_rw_signal(Vec::<CompletionItem>::new());
    let history_cursor = create_rw_signal::<Option<usize>>(None);
    let active_execution = create_rw_signal::<Option<PersistedExecutionState>>(None);
    let processed_events = create_rw_signal(0usize);
    let pending_command = create_rw_signal::<Option<String>>(None);
    let hydrated = create_rw_signal(false);
    let last_saved = create_rw_signal::<Option<String>>(None);
    let services_for_persist = services.clone();
    let hydrate_alive = Rc::new(Cell::new(true));
    on_cleanup({
        let hydrate_alive = hydrate_alive.clone();
        move || hydrate_alive.set(false)
    });

    if let Some(restored_state) = restored_state.as_ref() {
        if let Ok(restored) = serde_json::from_value::<TerminalPersistedState>(restored_state.clone())
        {
            let restored = restore_terminal_state(restored, &launch_cwd);
            let serialized = serde_json::to_string(&restored).ok();
            cwd.set(restored.cwd);
            input.set(restored.input);
            transcript.set(restored.transcript);
            history_cursor.set(restored.history_cursor);
            active_execution.set(restored.active_execution);
            last_saved.set(serialized);
            hydrated.set(true);
        }
    }

    create_effect(move |_| {
        let cwd = cwd;
        let input = input;
        let transcript = transcript;
        let history_cursor = history_cursor;
        let active_execution = active_execution;
        let hydrated = hydrated;
        let last_saved = last_saved;
        let hydrate_alive = hydrate_alive.clone();
        let launch_cwd = launch_cwd.clone();
        spawn_local(async move {
            match platform_storage::load_app_state_with_migration::<TerminalPersistedState, _>(
                TERMINAL_STATE_NAMESPACE,
                TERMINAL_STATE_SCHEMA_VERSION,
                migrate_terminal_state,
            )
            .await
            {
                Ok(Some(restored)) => {
                    if last_saved.get_untracked().is_none() {
                        let restored = restore_terminal_state(restored, &launch_cwd);
                        if !hydrate_alive.get() {
                            return;
                        }
                        let serialized = serde_json::to_string(&restored).ok();
                        cwd.set(restored.cwd);
                        input.set(restored.input);
                        transcript.set(restored.transcript);
                        history_cursor.set(restored.history_cursor);
                        active_execution.set(restored.active_execution);
                        last_saved.set(serialized);
                    }
                }
                Ok(None) => {}
                Err(err) => logging::warn!("terminal hydrate failed: {err}"),
            }
            if !hydrate_alive.get() {
                return;
            }
            hydrated.set(true);
        });
    });

    if let Some(shell_session) = shell_session.clone() {
        create_effect(move |_| {
            let events = shell_session.events.get();
            let already_processed = processed_events.get();
            if already_processed >= events.len() {
                return;
            }

            for event in events.iter().skip(already_processed) {
                match event {
                    ShellStreamEvent::Started { execution_id } => {
                        let command = pending_command.get_untracked().unwrap_or_default();
                        if !command.is_empty() {
                            active_execution.set(Some(PersistedExecutionState {
                                execution_id: *execution_id,
                                command,
                            }));
                            pending_command.set(None);
                        }
                    }
                    ShellStreamEvent::StdoutChunk { execution_id, text } => transcript.update(|entries| {
                        entries.push(TerminalTranscriptEntry::Stdout {
                            text: text.clone(),
                            execution_id: *execution_id,
                        });
                        normalize_terminal_transcript(entries);
                    }),
                    ShellStreamEvent::StderrChunk { execution_id, text } => transcript.update(|entries| {
                        entries.push(TerminalTranscriptEntry::Stderr {
                            text: text.clone(),
                            execution_id: *execution_id,
                        });
                        normalize_terminal_transcript(entries);
                    }),
                    ShellStreamEvent::Status { execution_id, text } => transcript.update(|entries| {
                        entries.push(TerminalTranscriptEntry::Status {
                            text: text.clone(),
                            execution_id: *execution_id,
                        });
                        normalize_terminal_transcript(entries);
                    }),
                    ShellStreamEvent::Json { execution_id, value } => transcript.update(|entries| {
                        entries.push(TerminalTranscriptEntry::Json {
                            value: value.clone(),
                            execution_id: *execution_id,
                        });
                        normalize_terminal_transcript(entries);
                    }),
                    ShellStreamEvent::Cancelled { .. } => {
                        active_execution.set(None);
                    }
                    ShellStreamEvent::Completed { .. } => {
                        active_execution.set(None);
                    }
                    ShellStreamEvent::Progress { .. } => {}
                }
            }

            processed_events.set(events.len());
            if let Some(session_cwd) = shell_session.cwd.get().split_whitespace().next() {
                cwd.set(shell_session.cwd.get());
                let _ = session_cwd;
            }
        });
    }

    create_effect(move |_| {
        if !hydrated.get() {
            return;
        }

        let mut snapshot = TerminalPersistedState {
            cwd: cwd.get(),
            input: input.get(),
            transcript: transcript.get(),
            history_cursor: history_cursor.get(),
            active_execution: active_execution.get(),
        };
        normalize_terminal_transcript(&mut snapshot.transcript);

        let serialized = match serde_json::to_string(&snapshot) {
            Ok(raw) => raw,
            Err(err) => {
                logging::warn!("terminal serialize failed: {err}");
                return;
            }
        };

        if last_saved.get().as_deref() == Some(serialized.as_str()) {
            return;
        }
        last_saved.set(Some(serialized));

        if let Some(services) = services_for_persist.clone() {
            if let Ok(value) = serde_json::to_value(&snapshot) {
                services.state.persist_window_state(value);
            }
        }

        spawn_local(async move {
            if let Err(err) = platform_storage::save_app_state(
                TERMINAL_STATE_NAMESPACE,
                TERMINAL_STATE_SCHEMA_VERSION,
                &snapshot,
            )
            .await
            {
                logging::warn!("terminal persist failed: {err}");
            }
        });
    });

    let submit_command: Rc<dyn Fn(String)> = Rc::new({
        let shell_session = shell_session.clone();
        move |command: String| {
            let command = command.trim().to_string();
            if command.is_empty() {
                return;
            }

            transcript.update(|entries| {
                entries.push(TerminalTranscriptEntry::Prompt {
                    cwd: cwd.get_untracked(),
                    command: command.clone(),
                    execution_id: None,
                });
                normalize_terminal_transcript(entries);
            });

            history_cursor.set(None);
            suggestions.set(Vec::new());
            input.set(String::new());

            if command.eq_ignore_ascii_case("clear") {
                transcript.set(default_terminal_transcript());
                active_execution.set(None);
                pending_command.set(None);
                return;
            }

            match shell_session.clone() {
                Some(shell_session) => {
                    pending_command.set(Some(command.clone()));
                    shell_session.submit(ShellRequest {
                        line: command,
                        cwd: cwd.get_untracked(),
                        source_window_id: None,
                    });
                }
                None => transcript.update(|entries| {
                    entries.push(TerminalTranscriptEntry::System {
                        text: "Shell session unavailable.".to_string(),
                    });
                    normalize_terminal_transcript(entries);
                }),
            }
        }
    });

    let try_history_navigation = {
        let services = services.clone();
        move |direction: i32| {
            let Some(services) = services.as_ref() else {
                return;
            };
            let history = services.commands.history.get();
            if history.is_empty() {
                return;
            }

            let next_index = match (history_cursor.get_untracked(), direction) {
                (None, -1) => Some(history.len().saturating_sub(1)),
                (Some(index), -1) if index > 0 => Some(index - 1),
                (Some(index), 1) if index + 1 < history.len() => Some(index + 1),
                (Some(_), 1) => None,
                (current, _) => current,
            };

            history_cursor.set(next_index);
            match next_index {
                Some(index) => input.set(history[index].clone()),
                None => input.set(String::new()),
            }
        }
    };

    let trigger_completion = {
        let shell_session = shell_session.clone();
        move || {
            let Some(shell_session) = shell_session.clone() else {
                return;
            };
            let current_input = input.get_untracked();
            spawn_local(async move {
                match shell_session
                    .complete(completion_request(&cwd.get_untracked(), &current_input))
                    .await
                {
                    Ok(items) => {
                        if items.len() == 1 {
                            let value = items[0].value.clone();
                            input.set(format!("{value} "));
                            suggestions.set(Vec::new());
                        } else {
                            suggestions.set(items);
                        }
                    }
                    Err(err) => {
                        transcript.update(|entries| {
                            entries.push(TerminalTranscriptEntry::System {
                                text: err.message,
                            });
                            normalize_terminal_transcript(entries);
                        });
                    }
                }
            });
        }
    };

    let indexed_entries = move || {
        transcript
            .get()
            .into_iter()
            .enumerate()
            .map(|(idx, entry)| (idx, render_transcript_entry(&entry)))
            .collect::<Vec<_>>()
    };
    let shell_session_for_status = shell_session.clone();
    let submit_help = submit_command.clone();
    let submit_open = submit_command.clone();
    let submit_on_enter = submit_command.clone();
    let submit_on_click = submit_command.clone();

    view! {
        <div class="app-shell app-terminal-shell">
            <div class="terminal-toolbar">
                <button type="button" class="app-action" on:click=move |_| submit_help("help".to_string())>"Help"</button>
                <button type="button" class="app-action" on:click=move |_| submit_open("open system.explorer".to_string())>"Open Explorer"</button>
                <button type="button" class="app-action" on:click=move |_| {
                    transcript.set(default_terminal_transcript());
                    input.set(String::new());
                    active_execution.set(None);
                }>"Clear"</button>
            </div>

            <div class="terminal-screen" role="log" aria-live="polite">
                <For each=indexed_entries key=|(idx, _)| *idx let:entry>
                    <div class="terminal-line">{entry.1}</div>
                </For>
            </div>

            <Show when=move || !suggestions.get().is_empty() fallback=|| ()>
                <div class="terminal-completions" role="listbox" aria-label="Completions">
                    <For each=move || suggestions.get() key=|item| item.value.clone() let:item>
                        <button
                            type="button"
                            class="terminal-completion"
                            on:click=move |_| {
                                input.set(format!("{} ", item.value));
                                suggestions.set(Vec::new());
                            }
                        >
                            {item.label}
                        </button>
                    </For>
                </div>
            </Show>

            <div class="terminal-input-row">
                <label class="terminal-prompt" for=input_id.clone()>
                    {move || format!("{}>", cwd.get())}
                </label>
                <input
                    id=input_id.clone()
                    class="terminal-input app-field"
                    type="text"
                    value=move || input.get()
                    on:input=move |ev| input.set(event_target_value(&ev))
                    on:keydown=move |ev: KeyboardEvent| {
                        match ev.key().as_str() {
                            "Enter" => submit_on_enter(input.get_untracked()),
                            "ArrowUp" => {
                                ev.prevent_default();
                                try_history_navigation(-1);
                            }
                            "ArrowDown" => {
                                ev.prevent_default();
                                try_history_navigation(1);
                            }
                            "Tab" => {
                                ev.prevent_default();
                                trigger_completion();
                            }
                            "Escape" => suggestions.set(Vec::new()),
                            "c" | "C" if ev.ctrl_key() => {
                                if let Some(shell_session) = shell_session.clone() {
                                    ev.prevent_default();
                                    shell_session.cancel();
                                }
                            }
                            "l" | "L" if ev.ctrl_key() => {
                                ev.prevent_default();
                                transcript.set(default_terminal_transcript());
                            }
                            _ => {}
                        }
                    }
                    placeholder="Try: apps.list"
                    autocomplete="off"
                    spellcheck="false"
                />
                <button
                    type="button"
                    class="terminal-run app-action"
                    on:click=move |_| submit_on_click(input.get_untracked())
                >
                    "Run"
                </button>
            </div>

            <div class="app-statusbar">
                <span>
                    {move || if !hydrated.get() {
                        "Hydrating shell session"
                    } else if shell_session_for_status.is_none() {
                        "Shell unavailable"
                    } else if active_execution.get().is_some() {
                        "Running command"
                    } else {
                        "Shell ready"
                    }}
                </span>
                <span>{move || format!("{} entrie(s)", transcript.get().len())}</span>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_namespace_migration_supports_legacy_lines() {
        let envelope = platform_storage::build_app_state_envelope(
            TERMINAL_STATE_NAMESPACE,
            1,
            &LegacyTerminalPersistedState {
                cwd: "/".to_string(),
                input: "help".to_string(),
                lines: vec!["RetroShell 0.1".to_string()],
            },
        )
        .expect("build envelope");

        let migrated =
            migrate_terminal_state(1, &envelope).expect("legacy migration should succeed");
        assert!(migrated.is_some(), "expected migrated terminal state");
        let transcript = migrated.expect("state").transcript;
        assert!(matches!(
            transcript.first(),
            Some(TerminalTranscriptEntry::System { text }) if text == "RetroShell 0.1"
        ));
    }

    #[test]
    fn restore_marks_interrupted_execution() {
        let restored = restore_terminal_state(
            TerminalPersistedState {
                cwd: "/".to_string(),
                input: String::new(),
                transcript: Vec::new(),
                history_cursor: None,
                active_execution: Some(PersistedExecutionState {
                    execution_id: ExecutionId(4),
                    command: "apps.list".to_string(),
                }),
            },
            "/",
        );

        assert!(restored.active_execution.is_none());
        assert!(restored
            .transcript
            .iter()
            .any(|entry| matches!(entry, TerminalTranscriptEntry::System { text } if text.contains("interrupted"))));
    }
}
