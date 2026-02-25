use leptos::*;
use serde_json::Value;

#[component]
pub fn NotepadApp(launch_params: Value) -> impl IntoView {
    let slug = launch_params
        .get("slug")
        .and_then(Value::as_str)
        .unwrap_or("welcome");
    let wrap_lines = create_rw_signal(true);

    let content = sample_note(slug);
    let line_count = content.lines().count();
    let char_count = content.chars().count();

    view! {
        <div class="app-shell app-notepad-shell">
            <div class="app-menubar">
                <button type="button">"File"</button>
                <button type="button">"Edit"</button>
                <button type="button">"Search"</button>
                <button type="button">"Help"</button>
            </div>

            <div class="app-toolbar">
                <button type="button" on:click=move |_| wrap_lines.update(|v| *v = !*v)>
                    {move || if wrap_lines.get() { "Wrap: On" } else { "Wrap: Off" }}
                </button>
                <button type="button">"Copy Link"</button>
                <button type="button">"Prev"</button>
                <button type="button">"Next"</button>
            </div>

            <div class="notepad-ruler" aria-hidden="true">
                "1    10   20   30   40   50   60   70"
            </div>

            <div class="notepad-document">
                <div class="notepad-document-header">
                    <div class="doc-title">{format!("{}.txt", slug)}</div>
                    <div class="doc-meta">{format!("Slug: {slug}")}</div>
                </div>
                <pre
                    class=move || {
                        if wrap_lines.get() {
                            "notepad-page wrap"
                        } else {
                            "notepad-page nowrap"
                        }
                    }
                >
                    {content}
                </pre>
            </div>

            <div class="app-statusbar">
                <span>{format!("Lines: {line_count}")}</span>
                <span>{format!("Chars: {char_count}")}</span>
                <span>{move || if wrap_lines.get() { "Word Wrap" } else { "No Wrap" }}</span>
            </div>
        </div>
    }
}

fn sample_note(slug: &str) -> String {
    match slug {
        "about" => String::from(
            "Justin Short Personal Workstation\n\
             ===============================\n\n\
             This debranded desktop shell is the primary interface for the site.\n\
             It is built with Leptos + Rust/WASM and organized around a reusable\n\
             window manager runtime.\n\n\
             Goals:\n\
             - playful interaction\n\
             - durable architecture\n\
             - low-friction publishing\n",
        ),
        "terminal-cheatsheet" => String::from(
            "Terminal Commands\n\
             ----------------\n\
             help\n\
             open projects\n\
             open notes <slug>\n\
             search <query>\n\
             theme <name>\n\
             dial\n",
        ),
        _ => format!(
            "Welcome ({slug})\n\
             -----------------\n\
             This is the Notepad app shell. In the production version, this panel\n\
             displays build-time-rendered HTML/markdown content with deep-linking,\n\
             previous/next navigation, and tag views.\n\n\
             This placeholder is now a real mounted app crate, which means its UI\n\
             can evolve independently from the desktop runtime crate.\n"
        ),
    }
}
