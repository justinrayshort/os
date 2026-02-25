use leptos::ev::KeyboardEvent;
use leptos::*;
use serde_json::Value;

#[component]
pub fn TerminalApp(launch_params: Value) -> impl IntoView {
    let cwd = launch_params
        .get("cwd")
        .and_then(Value::as_str)
        .unwrap_or("~/desktop")
        .to_string();
    let input = create_rw_signal(String::new());
    let lines = create_rw_signal(vec![
        "RetroShell 0.1".to_string(),
        "Type `help` for commands.".to_string(),
    ]);
    let prompt_cwd = cwd.clone();
    let keydown_cwd = cwd.clone();
    let click_cwd = cwd.clone();
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
                }>"Clear"</button>
            </div>

            <div class="terminal-screen" role="log" aria-live="polite">
                <For each=move || lines.get() key=|line| line.clone() let:line>
                    <div class="terminal-line">{line}</div>
                </For>
            </div>

            <div class="terminal-input-row">
                <label class="terminal-prompt" for="retro-shell-input">{format!("{prompt_cwd}>")}</label>
                <input
                    id="retro-shell-input"
                    class="terminal-input"
                    type="text"
                    value=move || input.get()
                    on:input=move |ev| input.set(event_target_value(&ev))
                    on:keydown=move |ev: KeyboardEvent| {
                        if ev.key() == "Enter" {
                            submit_input_command(input, lines, &keydown_cwd);
                        }
                    }
                    placeholder="Try: help"
                    autocomplete="off"
                    spellcheck="false"
                />
                <button
                    type="button"
                    class="terminal-run"
                    on:click=move |_| submit_input_command(input, lines, &click_cwd)
                >
                    "Run"
                </button>
            </div>

            <div class="app-statusbar">
                <span>"Local terminal sandbox (UI only)"</span>
                <span>{move || format!("{} line(s)", lines.get().len())}</span>
            </div>
        </div>
    }
}

fn submit_input_command(input: RwSignal<String>, lines: RwSignal<Vec<String>>, cwd: &str) {
    let command = input.get_untracked().trim().to_string();
    if command.is_empty() {
        return;
    }

    lines.update(|out| {
        out.push(format!("{cwd}> {command}"));
        out.extend(simulate_command(&command));
        if out.len() > 200 {
            let overflow = out.len() - 200;
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
