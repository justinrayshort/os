//! Terminal desktop app UI component and simulated command transcript persistence.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

use std::{
    cell::Cell,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

use leptos::ev::KeyboardEvent;
use leptos::*;
use platform_storage::{self, AppStateEnvelope, TERMINAL_STATE_NAMESPACE};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const TERMINAL_STATE_SCHEMA_VERSION: u32 = 1;
const MAX_TERMINAL_LINES: usize = 200;
static NEXT_TERMINAL_INSTANCE_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TerminalPersistedState {
    cwd: String,
    input: String,
    lines: Vec<String>,
}

fn default_terminal_lines() -> Vec<String> {
    vec![
        "RetroShell 0.1".to_string(),
        "Type `help` for commands.".to_string(),
    ]
}

fn normalize_terminal_lines(lines: &mut Vec<String>) {
    if lines.is_empty() {
        *lines = default_terminal_lines();
        return;
    }

    if lines.len() > MAX_TERMINAL_LINES {
        let overflow = lines.len() - MAX_TERMINAL_LINES;
        lines.drain(0..overflow);
    }
}

fn restore_terminal_state(
    envelope: AppStateEnvelope,
    launch_cwd: &str,
) -> Option<TerminalPersistedState> {
    if envelope.envelope_version != platform_storage::APP_STATE_ENVELOPE_VERSION {
        return None;
    }

    if envelope.schema_version > TERMINAL_STATE_SCHEMA_VERSION {
        return None;
    }

    let mut restored = serde_json::from_value::<TerminalPersistedState>(envelope.payload).ok()?;
    if restored.cwd.trim().is_empty() {
        restored.cwd = launch_cwd.to_string();
    }
    normalize_terminal_lines(&mut restored.lines);
    Some(restored)
}

#[component]
/// Terminal app window contents.
///
/// This component presents a UI-only shell simulation and persists the transcript via
/// [`platform_storage`].
pub fn TerminalApp(
    /// App launch parameters (for example, the initial working directory).
    launch_params: Value,
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
    let cwd = create_rw_signal(launch_cwd.clone());
    let input = create_rw_signal(String::new());
    let lines = create_rw_signal(default_terminal_lines());
    let hydrated = create_rw_signal(false);
    let last_saved = create_rw_signal::<Option<String>>(None);
    let hydrate_alive = Rc::new(Cell::new(true));
    on_cleanup({
        let hydrate_alive = hydrate_alive.clone();
        move || hydrate_alive.set(false)
    });

    create_effect(move |_| {
        let cwd = cwd;
        let input = input;
        let lines = lines;
        let hydrated = hydrated;
        let last_saved = last_saved;
        let hydrate_alive = hydrate_alive.clone();
        let launch_cwd = launch_cwd.clone();
        spawn_local(async move {
            match platform_storage::load_app_state_envelope(TERMINAL_STATE_NAMESPACE).await {
                Ok(Some(envelope)) => {
                    if let Some(restored) = restore_terminal_state(envelope, &launch_cwd) {
                        if !hydrate_alive.get() {
                            return;
                        }
                        let serialized = serde_json::to_string(&restored).ok();
                        cwd.set(restored.cwd);
                        input.set(restored.input);
                        lines.set(restored.lines);
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

    create_effect(move |_| {
        if !hydrated.get() {
            return;
        }

        let mut snapshot = TerminalPersistedState {
            cwd: cwd.get(),
            input: input.get(),
            lines: lines.get(),
        };
        normalize_terminal_lines(&mut snapshot.lines);

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

        let envelope = match platform_storage::build_app_state_envelope(
            TERMINAL_STATE_NAMESPACE,
            TERMINAL_STATE_SCHEMA_VERSION,
            &snapshot,
        ) {
            Ok(envelope) => envelope,
            Err(err) => {
                logging::warn!("terminal envelope build failed: {err}");
                return;
            }
        };

        spawn_local(async move {
            if let Err(err) = platform_storage::save_app_state_envelope(&envelope).await {
                logging::warn!("terminal persist failed: {err}");
            }
        });
    });

    let indexed_lines = move || {
        lines
            .get()
            .into_iter()
            .enumerate()
            .collect::<Vec<(usize, String)>>()
    };

    view! {
        <div class="app-shell app-terminal-shell">
            <div class="terminal-toolbar">
                <button type="button" on:click=move |_| {
                    lines.update(|out| out.push("help".to_string()));
                    lines.update(|out| out.extend(simulate_command("help")));
                }>"Help"</button>
                <button type="button" on:click=move |_| {
                    lines.update(|out| out.push("open projects".to_string()));
                    lines.update(|out| out.extend(simulate_command("open projects")));
                }>"Open Projects"</button>
                <button type="button" on:click=move |_| {
                    lines.set(vec!["RetroShell 0.1".to_string(), "Screen cleared.".to_string()]);
                    input.set(String::new());
                }>"Clear"</button>
            </div>

            <div class="terminal-screen" role="log" aria-live="polite">
                <For
                    each=indexed_lines
                    key=|(idx, _)| *idx
                    let:entry
                >
                    <div class="terminal-line">{entry.1}</div>
                </For>
            </div>

            <div class="terminal-input-row">
                <label class="terminal-prompt" for=input_id.clone()>
                    {move || format!("{}>", cwd.get())}
                </label>
                <input
                    id=input_id.clone()
                    class="terminal-input"
                    type="text"
                    value=move || input.get()
                    on:input=move |ev| input.set(event_target_value(&ev))
                    on:keydown=move |ev: KeyboardEvent| {
                        if ev.key() == "Enter" {
                            submit_input_command(input, lines, cwd);
                        }
                    }
                    placeholder="Try: help"
                    autocomplete="off"
                    spellcheck="false"
                />
                <button
                    type="button"
                    class="terminal-run"
                    on:click=move |_| submit_input_command(input, lines, cwd)
                >
                    "Run"
                </button>
            </div>

            <div class="app-statusbar">
                <span>
                    {move || if hydrated.get() {
                        "Local terminal sandbox (UI only, hydrated)"
                    } else {
                        "Local terminal sandbox (UI only, hydrating)"
                    }}
                </span>
                <span>{move || format!("{} line(s)", lines.get().len())}</span>
            </div>
        </div>
    }
}

fn submit_input_command(
    input: RwSignal<String>,
    lines: RwSignal<Vec<String>>,
    cwd: RwSignal<String>,
) {
    let command = input.get_untracked().trim().to_string();
    if command.is_empty() {
        return;
    }

    let prompt_cwd = cwd.get_untracked();
    lines.update(|out| {
        out.push(format!("{prompt_cwd}> {command}"));
        out.extend(simulate_command(&command));
        if out.len() > MAX_TERMINAL_LINES {
            let overflow = out.len() - MAX_TERMINAL_LINES;
            out.drain(0..overflow);
        }
    });
    input.set(String::new());
}

fn simulate_command(command: &str) -> Vec<String> {
    let normalized = command.trim();
    if normalized.is_empty() {
        return vec![];
    }

    let lower = normalized.to_ascii_lowercase();
    match lower.as_str() {
        "help" => vec![
            "Commands: help, open projects, open notes <slug>, search <q>, theme classic, dial, clear".to_string(),
        ],
        "open projects" | "open explorer" => {
            vec!["Queued: open Explorer window (runtime command routing comes next).".to_string()]
        }
        "dial" | "connect" => vec!["Queued: open Dial-up modal.".to_string()],
        "clear" => vec!["Tip: use the Clear toolbar button for now.".to_string()],
        _ if lower.starts_with("open notes ") => {
            let slug = normalized.split_whitespace().skip(2).collect::<Vec<_>>().join("-");
            vec![format!("Queued: open Notepad note `{slug}`.")]
        }
        _ if lower.starts_with("search ") => {
            let query = normalized.split_whitespace().skip(1).collect::<Vec<_>>().join(" ");
            vec![
                format!("Searching index for `{query}` ..."),
                "No indexed backend connected yet; using demo UI results.".to_string(),
            ]
        }
        _ if lower.starts_with("theme ") => {
            let theme = normalized.split_whitespace().skip(1).collect::<Vec<_>>().join(" ");
            vec![format!("Theme request `{theme}` captured (runtime theme command hook pending).")]
        }
        _ => vec![format!("Unrecognized command: `{normalized}`")],
    }
}
