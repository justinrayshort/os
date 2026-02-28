//! Notepad desktop app UI component and multi-document workspace persistence.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

use std::collections::BTreeMap;

use desktop_app_contract::AppServices;
use leptos::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use system_ui::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NotepadWorkspaceState {
    wrap_lines: bool,
    active_slug: String,
    open_order: Vec<String>,
    documents: BTreeMap<String, String>,
}

impl NotepadWorkspaceState {
    fn new(slug: &str) -> Self {
        let slug = normalized_slug(slug);
        let mut documents = BTreeMap::new();
        documents.insert(slug.clone(), sample_note(&slug));
        Self {
            wrap_lines: true,
            active_slug: slug.clone(),
            open_order: vec![slug],
            documents,
        }
    }

    fn ensure_document(&mut self, slug: &str) {
        let slug = normalized_slug(slug);
        self.documents
            .entry(slug.clone())
            .or_insert_with(|| sample_note(&slug));
        if !self.open_order.iter().any(|s| s == &slug) {
            self.open_order.push(slug.clone());
        }
        self.active_slug = slug;
        self.normalize();
    }

    fn active_text(&self) -> String {
        self.documents
            .get(&self.active_slug)
            .cloned()
            .unwrap_or_default()
    }

    fn set_active_text(&mut self, text: String) {
        self.documents.insert(self.active_slug.clone(), text);
        self.normalize();
    }

    fn select_index(&mut self, idx: usize) {
        if let Some(slug) = self.open_order.get(idx).cloned() {
            self.active_slug = slug;
        }
        self.normalize();
    }

    fn move_active_by(&mut self, delta: isize) {
        self.normalize();
        if self.open_order.is_empty() {
            return;
        }
        let current = self
            .open_order
            .iter()
            .position(|slug| slug == &self.active_slug)
            .unwrap_or(0);
        let len = self.open_order.len() as isize;
        let next = ((current as isize + delta).rem_euclid(len)) as usize;
        self.select_index(next);
    }

    fn add_scratch(&mut self) {
        let mut index = 1usize;
        loop {
            let slug = if index == 1 {
                "scratch".to_string()
            } else {
                format!("scratch-{index}")
            };
            if !self.documents.contains_key(&slug) {
                self.documents.insert(slug.clone(), String::new());
                self.open_order.push(slug.clone());
                self.active_slug = slug;
                self.normalize();
                return;
            }
            index = index.saturating_add(1);
        }
    }

    fn normalize(&mut self) {
        self.open_order
            .retain(|slug| self.documents.contains_key(slug));
        if self.open_order.is_empty() {
            self.documents
                .entry("welcome".to_string())
                .or_insert_with(|| sample_note("welcome"));
            self.open_order.push("welcome".to_string());
        }
        if !self.documents.contains_key(&self.active_slug) {
            self.active_slug = self.open_order[0].clone();
        }
        for slug in self.documents.keys() {
            if !self.open_order.iter().any(|s| s == slug) {
                self.open_order.push(slug.clone());
            }
        }
    }
}

fn normalized_slug(slug: &str) -> String {
    let slug = slug.trim();
    if slug.is_empty() {
        "welcome".to_string()
    } else {
        slug.to_string()
    }
}

fn tab_dom_id(slug: &str) -> String {
    let mut id = String::from("notepad-tab-");
    for ch in slug.chars() {
        if ch.is_ascii_alphanumeric() {
            id.push(ch.to_ascii_lowercase());
        } else {
            id.push('-');
        }
    }
    id
}

#[component]
/// Notepad app window contents.
///
/// The component restores and persists a lightweight tabbed note workspace via
/// typed host contracts.
pub fn NotepadApp(
    /// App launch parameters (for example, the initial note slug).
    launch_params: Value,
    /// Manager-restored app state payload for this window instance.
    restored_state: Option<Value>,
    /// Optional app-host bridge for manager-owned commands.
    services: Option<AppServices>,
) -> impl IntoView {
    let requested_slug = launch_params
        .get("slug")
        .and_then(Value::as_str)
        .unwrap_or("welcome")
        .to_string();

    let workspace = create_rw_signal(NotepadWorkspaceState::new(&requested_slug));
    let hydrated = create_rw_signal(false);
    let last_saved = create_rw_signal::<Option<String>>(None);
    let transient_notice = create_rw_signal::<Option<String>>(None);
    let services_for_persist = services.clone();

    if let Some(restored_state) = restored_state.as_ref() {
        if let Ok(mut restored) =
            serde_json::from_value::<NotepadWorkspaceState>(restored_state.clone())
        {
            restored.ensure_document(&requested_slug);
            let serialized = serde_json::to_string(&restored).ok();
            workspace.set(restored);
            last_saved.set(serialized);
            hydrated.set(true);
        }
    }

    hydrated.set(true);

    create_effect(move |_| {
        if !hydrated.get() {
            return;
        }

        let snapshot = workspace.get();
        let serialized = match serde_json::to_string(&snapshot) {
            Ok(raw) => raw,
            Err(err) => {
                logging::warn!("notepad serialize failed: {err}");
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
    });

    let current_text = Signal::derive(move || workspace.get().active_text());
    let line_count = Signal::derive(move || current_text.get().lines().count());
    let char_count = Signal::derive(move || current_text.get().chars().count());
    let on_tab_keydown = move |ev: ev::KeyboardEvent| {
        let key = ev.key();
        match key.as_str() {
            "ArrowLeft" => {
                ev.prevent_default();
                workspace.update(|w| w.move_active_by(-1));
            }
            "ArrowRight" => {
                ev.prevent_default();
                workspace.update(|w| w.move_active_by(1));
            }
            "Home" => {
                ev.prevent_default();
                workspace.update(|w| w.select_index(0));
            }
            "End" => {
                ev.prevent_default();
                workspace.update(|w| {
                    if !w.open_order.is_empty() {
                        w.select_index(w.open_order.len().saturating_sub(1));
                    }
                });
            }
            _ => {}
        }
    };

    view! {
        <AppShell layout_class="app-notepad-shell">
            <MenuBar>
                <Button>"File"</Button>
                <Button>"Edit"</Button>
                <Button>"Search"</Button>
                <Button>"Help"</Button>
            </MenuBar>

            <ToolBar>
                <Button
                    variant=ButtonVariant::Quiet
                    on_click=Callback::new(move |_| {
                        workspace.update(|w| w.wrap_lines = !w.wrap_lines);
                    })
                >
                    {move || if workspace.get().wrap_lines { "Wrap: On" } else { "Wrap: Off" }}
                </Button>
                <Button
                    on_click=Callback::new(move |_| {
                        workspace.update(|w| w.add_scratch());
                        transient_notice.set(Some("Created scratch document".to_string()));
                    })
                >
                    "New Scratch"
                </Button>
                <Button
                    on_click=Callback::new(move |_| {
                        transient_notice.set(Some("Auto-save is enabled (IndexedDB)".to_string()));
                    })
                >
                    "Save"
                </Button>
                <Button
                    on_click=Callback::new(move |_| workspace.update(|w| w.move_active_by(-1)))
                >
                    "Prev"
                </Button>
                <Button
                    on_click=Callback::new(move |_| workspace.update(|w| w.move_active_by(1)))
                >
                    "Next"
                </Button>
            </ToolBar>

            <div class="notepad-ruler" aria-hidden="true">
                "1    10   20   30   40   50   60   70"
            </div>

            <div class="notepad-document">
                <div class="notepad-document-header">
                    <div class="doc-title">{move || format!("{}.txt", workspace.get().active_slug)}</div>
                    <div class="doc-meta">
                        {move || {
                            let w = workspace.get();
                            format!(
                                "{} open doc(s) | {}",
                                w.open_order.len(),
                                if hydrated.get() { "hydrated" } else { "hydrating" }
                            )
                        }}
                    </div>
                </div>

                <div
                    class="notepad-tabstrip"
                    role="tablist"
                    aria-label="Open documents"
                    aria-orientation="horizontal"
                >
                    <For
                        each=move || workspace.get().open_order.clone()
                        key=|slug| slug.clone()
                        let:slug
                    >
                        {move || {
                            let class_slug = slug.clone();
                            let aria_slug = slug.clone();
                            let click_slug = slug.clone();
                            let label_slug = slug.clone();
                            let tab_id_slug = slug.clone();
                            let tabindex_slug = slug.clone();
                            view! {
                                <button
                                    type="button"
                                    class="ui-button notepad-tab"
                                    data-ui-primitive="true"
                                    data-ui-kind="button"
                                    data-ui-selected=move || if workspace.get().active_slug == class_slug { "true" } else { "false" }
                                    id=move || tab_dom_id(&tab_id_slug)
                                    role="tab"
                                    aria-selected=move || workspace.get().active_slug == aria_slug
                                    aria-controls="notepad-tabpanel"
                                    tabindex=move || {
                                        if workspace.get().active_slug == tabindex_slug {
                                            0
                                        } else {
                                            -1
                                        }
                                    }
                                    on:click=move |_| workspace.update(|w| w.ensure_document(&click_slug))
                                    on:keydown=on_tab_keydown
                                >
                                    {label_slug}
                                </button>
                            }
                        }}
                    </For>
                </div>

                <div
                    id="notepad-tabpanel"
                    role="tabpanel"
                    aria-labelledby=move || tab_dom_id(&workspace.get().active_slug)
                >
                    <textarea
                        class=move || if workspace.get().wrap_lines {
                            "ui-textarea notepad-page wrap"
                        } else {
                            "ui-textarea notepad-page nowrap"
                        }
                        data-ui-primitive="true"
                        data-ui-kind="text-area"
                        prop:value=move || current_text.get()
                        on:input=move |ev| {
                            let text = event_target_value(&ev);
                            workspace.update(|w| w.set_active_text(text));
                            transient_notice.set(None);
                        }
                        spellcheck="false"
                        autocomplete="off"
                        aria-label="Notepad document editor"
                    />
                </div>
            </div>

            <StatusBar>
                <span class="ui-statusbar-item">{move || format!("Lines: {}", line_count.get())}</span>
                <span class="ui-statusbar-item">{move || format!("Chars: {}", char_count.get())}</span>
                <span class="ui-statusbar-item">{move || {
                    transient_notice
                        .get()
                        .unwrap_or_else(|| if workspace.get().wrap_lines { "Word Wrap".to_string() } else { "No Wrap".to_string() })
                }}</span>
            </StatusBar>
        </AppShell>
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
             This Notepad workspace now persists documents, wrap settings, and open tabs\n\
             into a versioned IndexedDB app-state namespace (`app.notepad`).\n\n\
             You can edit this text and it will hydrate on the next boot.\n"
        ),
    }
}
