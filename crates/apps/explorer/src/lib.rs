//! Explorer desktop app UI component backed by typed host contracts.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

use std::{cell::Cell, rc::Rc};

use desktop_app_contract::{AppEvent, AppServices, CacheHostService, ExplorerHostService};
use leptos::*;
use platform_host::{
    explorer_preview_cache_key, session_store, CapabilityStatus, ExplorerBackend,
    ExplorerBackendStatus, ExplorerEntry, ExplorerEntryKind, ExplorerMetadata,
    ExplorerPermissionMode, ExplorerPrefs, EXPLORER_CACHE_NAME, EXPLORER_PREFS_KEY,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExplorerPersistedState {
    cwd: String,
    selected_path: Option<String>,
    selected_metadata: Option<ExplorerMetadata>,
    editor_path: Option<String>,
    editor_text: String,
    editor_dirty: bool,
    last_backend: ExplorerBackend,
}

impl Default for ExplorerPersistedState {
    fn default() -> Self {
        Self {
            cwd: "/".to_string(),
            selected_path: None,
            selected_metadata: None,
            editor_path: None,
            editor_text: String::new(),
            editor_dirty: false,
            last_backend: ExplorerBackend::IndexedDbVirtual,
        }
    }
}

#[derive(Clone, Copy)]
struct ExplorerSignals {
    status: RwSignal<Option<ExplorerBackendStatus>>,
    cwd: RwSignal<String>,
    entries: RwSignal<Vec<ExplorerEntry>>,
    selected_path: RwSignal<Option<String>>,
    selected_metadata: RwSignal<Option<ExplorerMetadata>>,
    editor_path: RwSignal<Option<String>>,
    editor_text: RwSignal<String>,
    editor_dirty: RwSignal<bool>,
    error: RwSignal<Option<String>>,
    notice: RwSignal<Option<String>>,
    busy: RwSignal<bool>,
}

fn normalize_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return "/".to_string();
    }
    let mut out = String::new();
    for segment in trimmed.replace('\\', "/").split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            if let Some(idx) = out.rfind('/') {
                out.truncate(idx);
            }
            continue;
        }
        out.push('/');
        out.push_str(segment);
    }
    if out.is_empty() {
        "/".to_string()
    } else {
        out
    }
}

fn join_path(base: &str, name: &str) -> String {
    let base = normalize_path(base);
    let name = name.trim().trim_matches('/');
    if name.is_empty() {
        return base;
    }
    if base == "/" {
        format!("/{name}")
    } else {
        format!("{base}/{name}")
    }
}

fn parent_path(path: &str) -> String {
    let path = normalize_path(path);
    if path == "/" {
        return path;
    }
    match path.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(idx) => path[..idx].to_string(),
    }
}

fn entry_name(path: &str) -> String {
    let path = normalize_path(path);
    if path == "/" {
        "/".to_string()
    } else {
        path.rsplit('/').next().unwrap_or_default().to_string()
    }
}

fn explorer_row_dom_id(path: &str) -> String {
    let mut id = String::from("explorer-row-");
    for ch in path.chars() {
        if ch.is_ascii_alphanumeric() {
            id.push(ch.to_ascii_lowercase());
        } else {
            id.push('-');
        }
    }
    id
}

fn persisted_snapshot(signals: ExplorerSignals) -> ExplorerPersistedState {
    ExplorerPersistedState {
        cwd: signals.cwd.get(),
        selected_path: signals.selected_path.get(),
        selected_metadata: signals.selected_metadata.get(),
        editor_path: signals.editor_path.get(),
        editor_text: signals.editor_text.get(),
        editor_dirty: signals.editor_dirty.get(),
        last_backend: signals
            .status
            .get()
            .map(|s| s.backend)
            .unwrap_or(ExplorerBackend::IndexedDbVirtual),
    }
}

fn set_error(signals: ExplorerSignals, message: impl Into<String>) {
    signals.error.set(Some(message.into()));
    signals.notice.set(None);
}

fn set_notice(signals: ExplorerSignals, message: impl Into<String>) {
    signals.notice.set(Some(message.into()));
    signals.error.set(None);
}

fn native_explorer_status(services: Option<&AppServices>) -> CapabilityStatus {
    services
        .map(|services| services.capabilities().host().native_explorer)
        .unwrap_or(CapabilityStatus::Unavailable)
}

fn can_connect_native_folder(status: CapabilityStatus) -> bool {
    !matches!(status, CapabilityStatus::Unavailable)
}

fn native_explorer_status_label(status: CapabilityStatus) -> &'static str {
    match status {
        CapabilityStatus::Available => "Native folder access is available.",
        CapabilityStatus::RequiresUserActivation => {
            "Native folder access requires explicit user activation."
        }
        CapabilityStatus::Unavailable => "Native folder access is unavailable on this host.",
    }
}

fn refresh_directory(
    signals: ExplorerSignals,
    explorer: Option<ExplorerHostService>,
    path: Option<String>,
) {
    let target = path.unwrap_or_else(|| signals.cwd.get_untracked());
    let target = normalize_path(&target);
    signals.busy.set(true);
    spawn_local(async move {
        let Some(explorer) = explorer else {
            set_error(signals, "Explorer host service unavailable");
            signals.busy.set(false);
            return;
        };
        let list_result = explorer.list_dir(&target).await;
        match list_result {
            Ok(result) => {
                let cwd = result.cwd.clone();
                signals.cwd.set(cwd.clone());
                signals.entries.set(result.entries);
                let status = signals.status.get_untracked();
                let merged_status = ExplorerBackendStatus {
                    backend: result.backend,
                    native_supported: status.as_ref().map(|s| s.native_supported).unwrap_or(false),
                    has_native_root: status.as_ref().map(|s| s.has_native_root).unwrap_or(false),
                    permission: result.permission,
                    root_path_hint: status.and_then(|s| s.root_path_hint),
                };
                signals.status.set(Some(merged_status));

                let current_selection = signals.selected_path.get_untracked();
                let still_exists = current_selection.as_ref().map(|sel| {
                    sel == &cwd
                        || signals
                            .entries
                            .get_untracked()
                            .iter()
                            .any(|entry| &entry.path == sel)
                });
                if !matches!(still_exists, Some(true)) {
                    signals.selected_path.set(None);
                    signals.selected_metadata.set(None);
                }
                set_notice(signals, format!("Loaded {}", cwd));
            }
            Err(err) => set_error(signals, format!("list failed: {err}")),
        }
        signals.busy.set(false);
    });
}

fn inspect_path(signals: ExplorerSignals, explorer: Option<ExplorerHostService>, path: String) {
    let path = normalize_path(&path);
    spawn_local(async move {
        let Some(explorer) = explorer else {
            set_error(signals, "Explorer host service unavailable");
            return;
        };
        match explorer.stat(&path).await {
            Ok(meta) => signals.selected_metadata.set(Some(meta)),
            Err(err) => set_error(signals, format!("metadata failed: {err}")),
        }
    });
}

fn open_file(
    signals: ExplorerSignals,
    explorer: Option<ExplorerHostService>,
    cache: Option<CacheHostService>,
    path: String,
) {
    let path = normalize_path(&path);
    signals.busy.set(true);
    spawn_local(async move {
        let Some(explorer) = explorer else {
            set_error(signals, "Explorer host service unavailable");
            signals.busy.set(false);
            return;
        };
        match explorer.read_text_file(&path).await {
            Ok(file) => {
                signals.editor_path.set(Some(file.path.clone()));
                signals.editor_text.set(file.text.clone());
                signals.editor_dirty.set(false);
                signals.selected_path.set(Some(file.path.clone()));
                signals.selected_metadata.set(Some(file.metadata.clone()));
                set_notice(
                    signals,
                    format!("Opened {} ({:?})", file.path, file.metadata.backend),
                );
            }
            Err(err) => {
                let cache_key = explorer_preview_cache_key(&path);
                let Some(cache) = cache else {
                    set_error(signals, format!("read failed: {err}"));
                    signals.busy.set(false);
                    return;
                };
                match cache.get_text(EXPLORER_CACHE_NAME, &cache_key).await {
                    Ok(Some(cached)) => {
                        signals.editor_path.set(Some(path.clone()));
                        signals.editor_text.set(cached);
                        signals.editor_dirty.set(true);
                        set_error(
                            signals,
                            format!("read failed: {err}. Loaded cached preview; save to restore"),
                        );
                    }
                    Ok(None) => set_error(signals, format!("read failed: {err}")),
                    Err(cache_err) => set_error(
                        signals,
                        format!("read failed: {err}; cache fallback failed: {cache_err}"),
                    ),
                }
            }
        }
        signals.busy.set(false);
    });
}

fn save_editor(
    signals: ExplorerSignals,
    explorer: Option<ExplorerHostService>,
    cache: Option<CacheHostService>,
) {
    let Some(path) = signals.editor_path.get_untracked() else {
        set_error(signals, "No file is open in the editor");
        return;
    };
    let text = signals.editor_text.get_untracked();
    signals.busy.set(true);
    spawn_local(async move {
        let Some(explorer) = explorer else {
            set_error(signals, "Explorer host service unavailable");
            signals.busy.set(false);
            return;
        };
        match explorer.write_text_file(&path, &text).await {
            Ok(meta) => {
                signals.editor_dirty.set(false);
                signals.selected_metadata.set(Some(meta.clone()));
                if let Some(cache) = cache {
                    let cache_key = explorer_preview_cache_key(&path);
                    if let Err(err) = cache.delete(EXPLORER_CACHE_NAME, &cache_key).await {
                        logging::warn!("explorer cache delete failed: {err}");
                    }
                }
                set_notice(signals, format!("Saved {}", meta.path));
                refresh_directory(signals, Some(explorer.clone()), Some(parent_path(&path)));
            }
            Err(err) => set_error(signals, format!("save failed: {err}")),
        }
        signals.busy.set(false);
    });
}

fn create_folder(
    signals: ExplorerSignals,
    explorer: Option<ExplorerHostService>,
    cwd: String,
    name: String,
) {
    let path = join_path(&cwd, &name);
    signals.busy.set(true);
    spawn_local(async move {
        let Some(explorer) = explorer else {
            set_error(signals, "Explorer host service unavailable");
            signals.busy.set(false);
            return;
        };
        match explorer.create_dir(&path).await {
            Ok(meta) => {
                set_notice(signals, format!("Created folder {}", meta.path));
                refresh_directory(
                    signals,
                    Some(explorer.clone()),
                    Some(parent_path(&meta.path)),
                );
            }
            Err(err) => set_error(signals, format!("create folder failed: {err}")),
        }
        signals.busy.set(false);
    });
}

fn create_file(
    signals: ExplorerSignals,
    explorer: Option<ExplorerHostService>,
    cache: Option<CacheHostService>,
    cwd: String,
    name: String,
) {
    let path = join_path(&cwd, &name);
    signals.busy.set(true);
    spawn_local(async move {
        let Some(explorer) = explorer else {
            set_error(signals, "Explorer host service unavailable");
            signals.busy.set(false);
            return;
        };
        match explorer.create_file(&path, "").await {
            Ok(meta) => {
                signals.selected_path.set(Some(meta.path.clone()));
                signals.selected_metadata.set(Some(meta.clone()));
                refresh_directory(
                    signals,
                    Some(explorer.clone()),
                    Some(parent_path(&meta.path)),
                );
                open_file(signals, Some(explorer), cache, meta.path.clone());
                set_notice(signals, format!("Created file {}", meta.path));
            }
            Err(err) => set_error(signals, format!("create file failed: {err}")),
        }
        signals.busy.set(false);
    });
}

fn delete_selected(
    signals: ExplorerSignals,
    explorer: Option<ExplorerHostService>,
    cache: Option<CacheHostService>,
) {
    let Some(path) = signals.selected_path.get_untracked() else {
        set_error(signals, "Select a file or folder to delete");
        return;
    };
    if path == "/" {
        set_error(signals, "Cannot delete the root directory");
        return;
    }
    signals.busy.set(true);
    spawn_local(async move {
        let Some(explorer) = explorer else {
            set_error(signals, "Explorer host service unavailable");
            signals.busy.set(false);
            return;
        };
        match explorer.delete(&path, true).await {
            Ok(()) => {
                if signals.editor_path.get_untracked() == Some(path.clone()) {
                    signals.editor_path.set(None);
                    signals.editor_text.set(String::new());
                    signals.editor_dirty.set(false);
                }
                if let Some(cache) = cache {
                    let cache_key = explorer_preview_cache_key(&path);
                    if let Err(err) = cache.delete(EXPLORER_CACHE_NAME, &cache_key).await {
                        logging::warn!("explorer cache delete failed: {err}");
                    }
                }
                signals.selected_path.set(None);
                signals.selected_metadata.set(None);
                set_notice(signals, format!("Deleted {}", path));
                refresh_directory(signals, Some(explorer), Some(parent_path(&path)));
            }
            Err(err) => set_error(signals, format!("delete failed: {err}")),
        }
        signals.busy.set(false);
    });
}

fn request_rw_permission(signals: ExplorerSignals, explorer: Option<ExplorerHostService>) {
    spawn_local(async move {
        let Some(explorer) = explorer else {
            set_error(signals, "Explorer host service unavailable");
            return;
        };
        match explorer
            .request_permission(ExplorerPermissionMode::Readwrite)
            .await
        {
            Ok(permission) => {
                if let Some(mut status) = signals.status.get_untracked() {
                    status.permission = permission;
                    signals.status.set(Some(status));
                }
                set_notice(signals, format!("Permission: {:?}", permission));
            }
            Err(err) => set_error(signals, format!("permission request failed: {err}")),
        }
    });
}

fn connect_native_folder(signals: ExplorerSignals, explorer: Option<ExplorerHostService>) {
    signals.busy.set(true);
    spawn_local(async move {
        let Some(explorer) = explorer else {
            set_error(signals, "Explorer host service unavailable");
            signals.busy.set(false);
            return;
        };
        match explorer.pick_native_directory().await {
            Ok(status) => {
                signals.status.set(Some(status));
                signals.cwd.set("/".to_string());
                refresh_directory(signals, Some(explorer), Some("/".to_string()));
                set_notice(signals, "Native folder connected");
            }
            Err(err) => set_error(signals, format!("connect folder failed: {err}")),
        }
        signals.busy.set(false);
    });
}

#[component]
/// Explorer app window contents.
///
/// The component hydrates persisted UI state and proxies filesystem/cache operations through
/// typed host contracts.
pub fn ExplorerApp(
    /// App launch parameters (for example, initial project slug hints).
    launch_params: Value,
    /// Manager-restored app state payload for this window instance.
    restored_state: Option<Value>,
    /// Optional app-host bridge for manager-owned commands.
    services: Option<AppServices>,
    /// Optional runtime inbox for app-bus events.
    inbox: Option<RwSignal<Vec<AppEvent>>>,
) -> impl IntoView {
    let native_explorer = native_explorer_status(services.as_ref());
    let initial_target = launch_params
        .get("project_slug")
        .and_then(Value::as_str)
        .map(|slug| format!("/Projects/{slug}"))
        .unwrap_or_else(|| "/".to_string());

    let prefs = create_rw_signal(ExplorerPrefs::default());
    let prefs_hydrated = create_rw_signal(false);
    let status = create_rw_signal::<Option<ExplorerBackendStatus>>(None);
    let cwd = create_rw_signal(normalize_path(&initial_target));
    let entries = create_rw_signal(Vec::<ExplorerEntry>::new());
    let selected_path = create_rw_signal::<Option<String>>(None);
    let selected_metadata = create_rw_signal::<Option<ExplorerMetadata>>(None);
    let editor_path = create_rw_signal::<Option<String>>(None);
    let editor_text = create_rw_signal(String::new());
    let editor_dirty = create_rw_signal(false);
    let error = create_rw_signal::<Option<String>>(None);
    let notice = create_rw_signal::<Option<String>>(None);
    let busy = create_rw_signal(false);
    let hydrated = create_rw_signal(false);
    let last_saved = create_rw_signal::<Option<String>>(None);
    let services_for_bus = services.clone();
    let services_for_persist = services.clone();
    let services_for_publish = services.clone();
    let explorer_service = store_value(services.as_ref().map(|services| services.explorer.clone()));
    let cache_service = store_value(services.as_ref().map(|services| services.cache.clone()));
    let prefs_service = store_value(services.as_ref().map(|services| services.prefs.clone()));

    let session_store = session_store();
    let initial_draft_name = session_store
        .get::<String>("explorer.ui.new_entry_name")
        .unwrap_or_default();
    let new_entry_name = create_rw_signal(initial_draft_name);

    let signals = ExplorerSignals {
        status,
        cwd,
        entries,
        selected_path,
        selected_metadata,
        editor_path,
        editor_text,
        editor_dirty,
        error,
        notice,
        busy,
    };

    if let Some(restored_state) = restored_state.as_ref() {
        if let Ok(restored) =
            serde_json::from_value::<ExplorerPersistedState>(restored_state.clone())
        {
            let serialized = serde_json::to_string(&restored).ok();
            signals.cwd.set(normalize_path(&restored.cwd));
            signals.selected_path.set(restored.selected_path);
            signals.selected_metadata.set(restored.selected_metadata);
            signals.editor_path.set(restored.editor_path.clone());
            signals.editor_text.set(restored.editor_text);
            signals.editor_dirty.set(restored.editor_dirty);
            last_saved.set(serialized);
        }
    }

    if let Some(services) = services_for_bus {
        create_effect(move |_| {
            services.ipc.subscribe("explorer.refresh");
        });
        on_cleanup(move || {
            services.ipc.unsubscribe("explorer.refresh");
        });
    }

    if let Some(services) = services_for_publish {
        create_effect(move |_| {
            services
                .ipc
                .publish("explorer.cwd.changed", json!({ "cwd": cwd.get() }));
        });
    }

    create_effect(move |_| {
        if signals.notice.get_untracked().is_none() && signals.error.get_untracked().is_none() {
            set_notice(signals, native_explorer_status_label(native_explorer));
        }
    });

    if let Some(inbox) = inbox {
        let cursor = Rc::new(Cell::new(0usize));
        create_effect(move |_| {
            let events = inbox.get();
            let start = cursor.get().min(events.len());
            for event in events[start..].iter() {
                if event.topic == "explorer.refresh" {
                    let target = event
                        .payload
                        .get("path")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    refresh_directory(signals, explorer_service.get_value(), target);
                }
            }
            cursor.set(events.len());
        });
    }

    create_effect(move |_| {
        if !prefs_hydrated.get() {
            return;
        }
        let prefs_value = prefs.get();
        let prefs_service = prefs_service.get_value();
        spawn_local(async move {
            if let Some(prefs_service) = prefs_service {
                if let Err(err) = prefs_service.save(EXPLORER_PREFS_KEY, &prefs_value).await {
                    logging::warn!("explorer prefs persist failed: {err}");
                }
            }
        });
    });

    let session_store_for_name = session_store.clone();
    create_effect(move |_| {
        let value = new_entry_name.get();
        let _ = session_store_for_name.set("explorer.ui.new_entry_name", &value);
    });

    let session_store_for_selection = session_store.clone();
    create_effect(move |_| {
        let value = selected_path.get();
        let _ = session_store_for_selection.set("explorer.ui.selected_path", &value);
    });

    create_effect(move |_| {
        if hydrated.get_untracked() {
            return;
        }
        let _ = prefs;
        let _ = last_saved;
        prefs_hydrated.set(true);
        hydrated.set(true);
    });

    create_effect(move |_| {
        if !hydrated.get() {
            return;
        }
        let snapshot = persisted_snapshot(signals);
        let serialized = match serde_json::to_string(&snapshot) {
            Ok(raw) => raw,
            Err(err) => {
                logging::warn!("explorer serialize failed: {err}");
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

    let visible_entries = Signal::derive(move || {
        let show_hidden = prefs.get().show_hidden;
        entries
            .get()
            .into_iter()
            .filter(|entry| show_hidden || !entry.name.starts_with('.'))
            .collect::<Vec<_>>()
    });
    let on_list_grid_keydown = move |ev: ev::KeyboardEvent| {
        let rows = visible_entries.get_untracked();
        if rows.is_empty() {
            return;
        }

        let selected = selected_path.get_untracked();
        let current_index = selected
            .as_deref()
            .and_then(|path| rows.iter().position(|entry| entry.path == path));
        let last_index = rows.len().saturating_sub(1);
        let key = ev.key();

        match key.as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                let next = current_index
                    .map(|idx| (idx + 1).min(last_index))
                    .unwrap_or(0);
                let entry = rows[next].clone();
                signals.selected_path.set(Some(entry.path.clone()));
                inspect_path(signals, explorer_service.get_value(), entry.path);
            }
            "ArrowUp" => {
                ev.prevent_default();
                let next = current_index
                    .map(|idx| idx.saturating_sub(1))
                    .unwrap_or(last_index);
                let entry = rows[next].clone();
                signals.selected_path.set(Some(entry.path.clone()));
                inspect_path(signals, explorer_service.get_value(), entry.path);
            }
            "Home" => {
                ev.prevent_default();
                let entry = rows[0].clone();
                signals.selected_path.set(Some(entry.path.clone()));
                inspect_path(signals, explorer_service.get_value(), entry.path);
            }
            "End" => {
                ev.prevent_default();
                let entry = rows[last_index].clone();
                signals.selected_path.set(Some(entry.path.clone()));
                inspect_path(signals, explorer_service.get_value(), entry.path);
            }
            " " | "Spacebar" => {
                ev.prevent_default();
                let index = current_index.unwrap_or(0);
                let entry = rows[index].clone();
                signals.selected_path.set(Some(entry.path.clone()));
                inspect_path(signals, explorer_service.get_value(), entry.path);
            }
            "Enter" => {
                ev.prevent_default();
                let index = current_index.unwrap_or(0);
                let entry = rows[index].clone();
                signals.selected_path.set(Some(entry.path.clone()));
                match entry.kind {
                    ExplorerEntryKind::Directory => {
                        refresh_directory(signals, explorer_service.get_value(), Some(entry.path))
                    }
                    ExplorerEntryKind::File => open_file(
                        signals,
                        explorer_service.get_value(),
                        cache_service.get_value(),
                        entry.path,
                    ),
                }
            }
            _ => {}
        }
    };

    view! {
        <div class="ui-app-shell app-explorer-shell" data-ui-primitive="true" data-ui-kind="app-shell">
            <div class="ui-menubar" data-ui-primitive="true" data-ui-kind="menubar">
                <button type="button" class="ui-button">"File"</button>
                <button type="button" class="ui-button">"Edit"</button>
                <button type="button" class="ui-button">"View"</button>
                <button type="button" class="ui-button">"Tools"</button>
                <button type="button" class="ui-button">"Help"</button>
            </div>

            <div class="ui-toolbar" data-ui-primitive="true" data-ui-kind="toolbar">
                <button
                    type="button"
                    class="ui-button"
                    title=native_explorer_status_label(native_explorer)
                    disabled=!can_connect_native_folder(native_explorer)
                    on:click=move |_| connect_native_folder(signals, explorer_service.get_value())
                >
                    "Connect Folder"
                </button>
                <button type="button" class="ui-button" on:click=move |_| refresh_directory(signals, explorer_service.get_value(), None)>
                    "Refresh"
                </button>
                <button type="button" class="ui-button" on:click=move |_| refresh_directory(signals, explorer_service.get_value(), Some(parent_path(&cwd.get_untracked())))>
                    "Up"
                </button>
                <button
                    type="button"
                    class="ui-button"
                    title=native_explorer_status_label(native_explorer)
                    disabled=!can_connect_native_folder(native_explorer)
                    on:click=move |_| request_rw_permission(signals, explorer_service.get_value())
                >
                    "Request RW"
                </button>
                <button type="button" class="ui-button" on:click=move |_| save_editor(signals, explorer_service.get_value(), cache_service.get_value()) disabled=move || !editor_dirty.get()>
                    "Save"
                </button>
                <button type="button" class="ui-button" on:click=move |_| delete_selected(signals, explorer_service.get_value(), cache_service.get_value())>
                    "Delete"
                </button>
                <button type="button" class="ui-button" on:click=move |_| prefs.update(|p| p.details_visible = !p.details_visible)>
                    {move || if prefs.get().details_visible { "Details On" } else { "Details Off" }}
                </button>
                <button type="button" class="ui-button" on:click=move |_| prefs.update(|p| p.show_hidden = !p.show_hidden)>
                    {move || if prefs.get().show_hidden { "Hidden On" } else { "Hidden Off" }}
                </button>
            </div>

            <div class="ui-toolbar" data-ui-primitive="true" data-ui-kind="toolbar">
                <input
                    type="text"
                    class="explorer-create-name ui-field"
                    placeholder="new-file.txt or folder"
                    value=move || new_entry_name.get()
                    on:input=move |ev| new_entry_name.set(event_target_value(&ev))
                    aria-label="New item name"
                />
                <button type="button" class="ui-button" on:click=move |_| {
                    let name = new_entry_name.get_untracked();
                    if name.trim().is_empty() {
                        set_error(signals, "Enter a name first");
                        return;
                    }
                    create_file(
                        signals,
                        explorer_service.get_value(),
                        cache_service.get_value(),
                        cwd.get_untracked(),
                        name,
                    );
                }>
                    "New File"
                </button>
                <button type="button" class="ui-button" on:click=move |_| {
                    let name = new_entry_name.get_untracked();
                    if name.trim().is_empty() {
                        set_error(signals, "Enter a name first");
                        return;
                    }
                    create_folder(signals, explorer_service.get_value(), cwd.get_untracked(), name);
                }>
                    "New Folder"
                </button>
                <button type="button" class="ui-button" on:click=move |_| {
                    signals.editor_path.set(None);
                    signals.editor_text.set(String::new());
                    signals.editor_dirty.set(false);
                }>
                    "Close Editor"
                </button>
            </div>

            <div class="explorer-workspace">
                <aside class="explorer-tree" aria-label="Explorer status and path">
                    <div class="tree-header">"Storage"</div>
                    <div class="explorer-status-card">
                        <div><strong>"Backend"</strong></div>
                        <div>{move || {
                            status
                                .get()
                                .map(|s| format!("{:?}", s.backend))
                                .unwrap_or_else(|| "Unknown".to_string())
                        }}</div>
                        <div><strong>"Permission"</strong></div>
                        <div>{move || {
                            status
                                .get()
                                .map(|s| format!("{:?}", s.permission))
                                .unwrap_or_else(|| "Unknown".to_string())
                        }}</div>
                        <div><strong>"Root"</strong></div>
                        <div>{move || {
                            status
                                .get()
                                .and_then(|s| s.root_path_hint)
                                .unwrap_or_else(|| "(virtual root)".to_string())
                        }}</div>
                    </div>

                    <div class="tree-header">"Path Segments"</div>
                    <ul class="tree-list">
                        <li>
                            <button type="button" class="tree-node ui-button" on:click=move |_| refresh_directory(signals, explorer_service.get_value(), Some("/".to_string()))>
                                <span class="tree-glyph">"[]"</span>
                                <span>"/"</span>
                            </button>
                        </li>
                        <For
                            each=move || {
                                let current = cwd.get();
                                let mut segments = Vec::new();
                                let mut running = String::new();
                                for seg in current.trim_start_matches('/').split('/') {
                                    if seg.is_empty() {
                                        continue;
                                    }
                                    running = join_path(&running, seg);
                                    segments.push((seg.to_string(), running.clone()));
                                }
                                segments
                            }
                            key=|(_, path)| path.clone()
                            let:item
                        >
                            <li>
                                <button type="button" class="tree-node ui-button" on:click=move |_| refresh_directory(signals, explorer_service.get_value(), Some(item.1.clone()))>
                                    <span class="tree-glyph">">"</span>
                                    <span>{item.0.clone()}</span>
                                </button>
                            </li>
                        </For>
                    </ul>
                </aside>

                <section class="explorer-pane">
                    <div class="pane-header">
                        <div class="pane-title">"Contents"</div>
                        <div class="pane-path">{move || format!("Path: {}", cwd.get())}</div>
                    </div>

                    <div class="explorer-listwrap">
                        <table
                            class="explorer-list"
                            role="grid"
                            aria-label="Explorer list view"
                            tabindex="0"
                            aria-activedescendant=move || {
                                selected_path.get().map(|path| explorer_row_dom_id(&path)).unwrap_or_default()
                            }
                            on:keydown=on_list_grid_keydown
                        >
                            <thead>
                                <tr>
                                    <th>"Name"</th>
                                    <th>"Type"</th>
                                    <th>"Modified"</th>
                                    <th>"Size"</th>
                                </tr>
                            </thead>
                            <tbody>
                                <For each=move || visible_entries.get() key=|entry| entry.path.clone() let:entry>
                                    {move || {
                                        let entry_for_select = entry.clone();
                                        let entry_for_open = entry.clone();
                                        let explorer_for_select = explorer_service.get_value();
                                        let explorer_for_open = explorer_service.get_value();
                                        let cache_for_open = cache_service.get_value();
                                        let row_selected = selected_path.get() == Some(entry.path.clone());
                                        view! {
                                            <tr
                                                id=explorer_row_dom_id(&entry.path)
                                                class=if row_selected { "selected" } else { "" }
                                                aria-selected=row_selected
                                                on:mousedown=move |_| {
                                                    signals.selected_path.set(Some(entry_for_select.path.clone()));
                                                    inspect_path(
                                                        signals,
                                                        explorer_for_select.clone(),
                                                        entry_for_select.path.clone(),
                                                    );
                                                }
                                                on:dblclick=move |_| {
                                                    signals.selected_path.set(Some(entry_for_open.path.clone()));
                                                    match entry_for_open.kind {
                                                        ExplorerEntryKind::Directory => {
                                                            refresh_directory(
                                                                signals,
                                                                explorer_for_open.clone(),
                                                                Some(entry_for_open.path.clone()),
                                                            );
                                                        }
                                                        ExplorerEntryKind::File => {
                                                            open_file(
                                                                signals,
                                                                explorer_for_open.clone(),
                                                                cache_for_open.clone(),
                                                                entry_for_open.path.clone(),
                                                            );
                                                        }
                                                    }
                                                }
                                            >
                                                <td>{entry.name.clone()}</td>
                                                <td>{match entry.kind { ExplorerEntryKind::Directory => "Folder", ExplorerEntryKind::File => "File" }}</td>
                                                <td>{entry.modified_at_unix_ms.map(format_timestamp).unwrap_or_else(|| "-".to_string())}</td>
                                                <td>{entry.size.map(format_bytes).unwrap_or_else(|| "-".to_string())}</td>
                                            </tr>
                                        }
                                    }}
                                </For>
                            </tbody>
                        </table>
                    </div>

                    <Show when=move || editor_path.get().is_some() fallback=|| ()>
                        <div class="explorer-editor">
                            <div class="pane-header">
                                <div class="pane-title">{move || {
                                    editor_path
                                        .get()
                                        .map(|path| format!("Editor: {}", entry_name(&path)))
                                        .unwrap_or_else(|| "Editor".to_string())
                                }}</div>
                                <div class="pane-path">{move || {
                                    if editor_dirty.get() { "Unsaved changes".to_string() } else { "Saved".to_string() }
                                }}</div>
                            </div>
                            <textarea
                                class="explorer-file-editor ui-textarea"
                                prop:value=move || editor_text.get()
                                on:input=move |ev| {
                                    editor_text.set(event_target_value(&ev));
                                    editor_dirty.set(true);
                                }
                                spellcheck="false"
                                autocomplete="off"
                                aria-label="Explorer text file editor"
                            />
                        </div>
                    </Show>

                    <Show when=move || prefs.get().details_visible fallback=|| ()>
                        <div class="explorer-details">
                            {move || {
                                if let Some(meta) = selected_metadata.get() {
                                    view! {
                                        <div class="details-grid">
                                            <div>"Name"</div><div>{meta.name.clone()}</div>
                                            <div>"Path"</div><div>{meta.path.clone()}</div>
                                            <div>"Kind"</div><div>{format!("{:?}", meta.kind)}</div>
                                            <div>"Backend"</div><div>{format!("{:?}", meta.backend)}</div>
                                            <div>"Permission"</div><div>{format!("{:?}", meta.permission)}</div>
                                            <div>"Modified"</div><div>{meta.modified_at_unix_ms.map(format_timestamp).unwrap_or_else(|| "-".to_string())}</div>
                                            <div>"Size"</div><div>{meta.size.map(format_bytes).unwrap_or_else(|| "-".to_string())}</div>
                                        </div>
                                    }
                                    .into_view()
                                } else {
                                    view! { <div class="details-empty">"Select an item to view metadata."</div> }
                                        .into_view()
                                }
                            }}
                        </div>
                    </Show>
                </section>
            </div>

            <div class="ui-statusbar" data-ui-primitive="true" data-ui-kind="statusbar">
                <span>{move || format!("{} item(s)", visible_entries.get().len())}</span>
                <span>{move || {
                    status
                        .get()
                        .map(|s| format!("Backend: {:?} | Permission: {:?}", s.backend, s.permission))
                        .unwrap_or_else(|| "Backend: loading".to_string())
                }}</span>
                <span>{move || {
                    if let Some(err) = error.get() {
                        format!("Error: {err}")
                    } else if let Some(note) = notice.get() {
                        note
                    } else if busy.get() {
                        "Working...".to_string()
                    } else if hydrated.get() {
                        "Ready".to_string()
                    } else {
                        "Hydrating...".to_string()
                    }
                }}</span>
            </div>
        </div>
    }
}

fn format_timestamp(unix_ms: u64) -> String {
    // Avoid pulling in chrono for a small client-side status formatter.
    let seconds = unix_ms / 1000;
    format!("{}s", seconds)
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let bytes_f = bytes as f64;
    if bytes_f >= MB {
        format!("{:.1} MB", bytes_f / MB)
    } else if bytes_f >= KB {
        format!("{:.1} KB", bytes_f / KB)
    } else {
        format!("{} B", bytes)
    }
}
