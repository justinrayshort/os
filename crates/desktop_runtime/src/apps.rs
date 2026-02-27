//! Desktop app registry metadata and app-content rendering helpers.

use std::{cell::Cell, rc::Rc};

use crate::model::{AppId, OpenWindowRequest, WindowRecord};
use desktop_app_calculator::CalculatorApp;
use desktop_app_explorer::ExplorerApp;
use desktop_app_notepad::NotepadApp;
use desktop_app_terminal::TerminalApp;
use leptos::*;
use platform_storage::{self};
use serde::{Deserialize, Serialize};

const PAINT_PLACEHOLDER_STATE_SCHEMA_VERSION: u32 = 1;

fn migrate_paint_placeholder_state(
    schema_version: u32,
    envelope: &platform_storage::AppStateEnvelope,
) -> Result<Option<PaintPlaceholderState>, String> {
    match schema_version {
        0 => platform_storage::migrate_envelope_payload(envelope).map(Some),
        _ => Ok(None),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Metadata describing how an app appears in the launcher/desktop and how it is instantiated.
pub struct AppDescriptor {
    /// Stable runtime application identifier.
    pub app_id: AppId,
    /// Label shown in the start/launcher menu.
    pub launcher_label: &'static str,
    /// Label shown under the desktop icon.
    pub desktop_icon_label: &'static str,
    /// Whether the app is listed in launcher menus.
    pub show_in_launcher: bool,
    /// Whether the app is rendered as a desktop icon.
    pub show_on_desktop: bool,
    /// Whether only one instance should be open at a time.
    pub single_instance: bool,
}

const APP_REGISTRY: [AppDescriptor; 6] = [
    AppDescriptor {
        app_id: AppId::Calculator,
        launcher_label: "Calculator",
        desktop_icon_label: "Calculator",
        show_in_launcher: true,
        show_on_desktop: true,
        single_instance: true,
    },
    AppDescriptor {
        app_id: AppId::Explorer,
        launcher_label: "Explorer",
        desktop_icon_label: "Explorer",
        show_in_launcher: true,
        show_on_desktop: true,
        single_instance: false,
    },
    AppDescriptor {
        app_id: AppId::Notepad,
        launcher_label: "Notepad",
        desktop_icon_label: "Notes",
        show_in_launcher: true,
        show_on_desktop: false,
        single_instance: false,
    },
    AppDescriptor {
        app_id: AppId::Paint,
        launcher_label: "Paint",
        desktop_icon_label: "Paint",
        show_in_launcher: true,
        show_on_desktop: false,
        single_instance: false,
    },
    AppDescriptor {
        app_id: AppId::Terminal,
        launcher_label: "Terminal",
        desktop_icon_label: "Terminal",
        show_in_launcher: true,
        show_on_desktop: true,
        single_instance: true,
    },
    AppDescriptor {
        app_id: AppId::Dialup,
        launcher_label: "Dial-up",
        desktop_icon_label: "Connect",
        show_in_launcher: true,
        show_on_desktop: false,
        single_instance: false,
    },
];

/// Returns the static app registry used by the desktop shell.
pub fn app_registry() -> &'static [AppDescriptor] {
    &APP_REGISTRY
}

/// Returns app descriptors that should appear in launcher menus.
pub fn launcher_apps() -> Vec<AppDescriptor> {
    app_registry()
        .iter()
        .copied()
        .filter(|entry| entry.show_in_launcher)
        .collect()
}

/// Returns app descriptors that should appear as desktop icons.
pub fn desktop_icon_apps() -> Vec<AppDescriptor> {
    app_registry()
        .iter()
        .copied()
        .filter(|entry| entry.show_on_desktop)
        .collect()
}

/// Returns the descriptor for `app_id`.
///
/// # Panics
///
/// Panics if the app id is not present in the registry.
pub fn app_descriptor(app_id: AppId) -> &'static AppDescriptor {
    app_registry()
        .iter()
        .find(|entry| entry.app_id == app_id)
        .expect("app descriptor exists")
}

/// Builds the default [`OpenWindowRequest`] for a given app.
///
/// Some apps override the default geometry to better fit their UI.
pub fn default_open_request(app_id: AppId) -> OpenWindowRequest {
    let mut req = OpenWindowRequest::new(app_id);
    if matches!(app_id, AppId::Calculator) {
        req.rect = Some(crate::model::WindowRect {
            x: 72,
            y: 64,
            w: 560,
            h: 420,
        });
    }
    req
}

/// Renders the Leptos view for a runtime window record.
pub fn render_window_contents(window: &WindowRecord) -> View {
    match window.app_id {
        AppId::Calculator => {
            view! { <CalculatorApp launch_params=window.launch_params.clone() /> }.into_view()
        }
        AppId::Explorer => {
            view! { <ExplorerApp launch_params=window.launch_params.clone() /> }.into_view()
        }
        AppId::Notepad => {
            view! { <NotepadApp launch_params=window.launch_params.clone() /> }.into_view()
        }
        AppId::Paint => render_paint_placeholder(),
        AppId::Terminal => {
            view! { <TerminalApp launch_params=window.launch_params.clone() /> }.into_view()
        }
        AppId::Dialup => render_dialup_placeholder(),
    }
}

fn render_paint_placeholder() -> View {
    view! { <PaintPlaceholderApp /> }.into_view()
}

fn render_dialup_placeholder() -> View {
    view! {
        <div class="app app-dialup">
            <p>"Dial-up placeholder"</p>
            <p>"Negotiating connection..."</p>
            <progress max="100" value="45"></progress>
        </div>
    }
    .into_view()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PaintPlaceholderState {
    tool: String,
    brush_size: u8,
    color_hex: String,
    canvas_preset: String,
    status: String,
}

impl Default for PaintPlaceholderState {
    fn default() -> Self {
        Self {
            tool: "brush".to_string(),
            brush_size: 6,
            color_hex: "#0b5fff".to_string(),
            canvas_preset: "800x600".to_string(),
            status: "Canvas placeholder ready".to_string(),
        }
    }
}

fn restore_paint_placeholder_state(mut restored: PaintPlaceholderState) -> PaintPlaceholderState {
    if restored.tool.trim().is_empty() {
        restored.tool = "brush".to_string();
    }
    if restored.color_hex.trim().is_empty() {
        restored.color_hex = "#0b5fff".to_string();
    }
    if restored.canvas_preset.trim().is_empty() {
        restored.canvas_preset = "800x600".to_string();
    }
    restored.brush_size = restored.brush_size.clamp(1, 64);
    if restored.status.trim().is_empty() {
        restored.status = "Canvas placeholder ready".to_string();
    }
    restored
}

#[component]
fn PaintPlaceholderApp() -> impl IntoView {
    let state = create_rw_signal(PaintPlaceholderState::default());
    let hydrated = create_rw_signal(false);
    let last_saved = create_rw_signal::<Option<String>>(None);
    let hydrate_alive = Rc::new(Cell::new(true));
    on_cleanup({
        let hydrate_alive = hydrate_alive.clone();
        move || hydrate_alive.set(false)
    });

    create_effect(move |_| {
        let state = state;
        let hydrated = hydrated;
        let last_saved = last_saved;
        let hydrate_alive = hydrate_alive.clone();
        spawn_local(async move {
            match platform_storage::load_app_state_with_migration::<PaintPlaceholderState, _>(
                platform_storage::PAINT_STATE_NAMESPACE,
                PAINT_PLACEHOLDER_STATE_SCHEMA_VERSION,
                migrate_paint_placeholder_state,
            )
            .await
            {
                Ok(Some(restored)) => {
                    let restored = restore_paint_placeholder_state(restored);
                    if !hydrate_alive.get() {
                        return;
                    }
                    let serialized = serde_json::to_string(&restored).ok();
                    state.set(restored);
                    last_saved.set(serialized);
                }
                Ok(None) => {}
                Err(err) => logging::warn!("paint placeholder hydrate failed: {err}"),
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

        let snapshot = state.get();
        let serialized = match serde_json::to_string(&snapshot) {
            Ok(raw) => raw,
            Err(err) => {
                logging::warn!("paint placeholder serialize failed: {err}");
                return;
            }
        };

        if last_saved.get().as_deref() == Some(serialized.as_str()) {
            return;
        }
        last_saved.set(Some(serialized));

        spawn_local(async move {
            if let Err(err) = platform_storage::save_app_state(
                platform_storage::PAINT_STATE_NAMESPACE,
                PAINT_PLACEHOLDER_STATE_SCHEMA_VERSION,
                &snapshot,
            )
            .await
            {
                logging::warn!("paint placeholder persist failed: {err}");
            }
        });
    });

    view! {
        <div class="app app-paint">
            <p><strong>"Paint (Placeholder)"</strong></p>
            <p>"Future canvas state will persist under the same namespaced IndexedDB schema path."</p>

            <div class="app-toolbar" role="group" aria-label="Paint placeholder controls">
                <label>
                    "Tool "
                    <select
                        prop:value=move || state.get().tool
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            state.update(|s| s.tool = value);
                        }
                    >
                        <option value="brush">"Brush"</option>
                        <option value="line">"Line"</option>
                        <option value="eraser">"Eraser"</option>
                        <option value="fill">"Fill"</option>
                    </select>
                </label>

                <label>
                    "Brush "
                    <input
                        type="range"
                        min="1"
                        max="64"
                        prop:value=move || state.get().brush_size.to_string()
                        on:input=move |ev| {
                            let value = event_target_value(&ev)
                                .parse::<u8>()
                                .unwrap_or(6)
                                .clamp(1, 64);
                            state.update(|s| s.brush_size = value);
                        }
                    />
                </label>

                <label>
                    "Color "
                    <input
                        type="color"
                        prop:value=move || state.get().color_hex
                        on:input=move |ev| {
                            let value = event_target_value(&ev);
                            state.update(|s| s.color_hex = value);
                        }
                    />
                </label>

                <label>
                    "Canvas "
                    <select
                        prop:value=move || state.get().canvas_preset
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            state.update(|s| s.canvas_preset = value);
                        }
                    >
                        <option value="800x600">"800x600"</option>
                        <option value="1024x768">"1024x768"</option>
                        <option value="1280x720">"1280x720"</option>
                    </select>
                </label>

                <button type="button" on:click=move |_| {
                    state.update(|s| s.status = "Placeholder save slot synced to IndexedDB".to_string());
                }>
                    "Save Slot"
                </button>
                <button type="button" on:click=move |_| {
                    state.update(|s| s.status = "Placeholder canvas cleared (state preserved)".to_string());
                }>
                    "Clear"
                </button>
            </div>

            <div class="app-statusbar">
                <span>{move || if hydrated.get() { "Hydrated" } else { "Hydrating" }}</span>
                <span>{move || {
                    let snapshot = state.get();
                    format!(
                        "{} | {}px | {} | {}",
                        snapshot.tool, snapshot.brush_size, snapshot.color_hex, snapshot.canvas_preset
                    )
                }}</span>
            </div>

            <p>{move || state.get().status}</p>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_namespace_migration_supports_schema_zero() {
        let envelope = platform_storage::build_app_state_envelope(
            platform_storage::PAINT_STATE_NAMESPACE,
            0,
            &PaintPlaceholderState::default(),
        )
        .expect("build envelope");

        let migrated = migrate_paint_placeholder_state(0, &envelope)
            .expect("schema-zero migration should succeed");
        assert!(migrated.is_some(), "expected migrated paint state");
    }
}
