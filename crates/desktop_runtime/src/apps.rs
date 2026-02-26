use std::{cell::Cell, rc::Rc};

use crate::model::{AppId, OpenWindowRequest, WindowRecord};
use desktop_app_calculator::CalculatorApp;
use desktop_app_explorer::ExplorerApp;
use desktop_app_notepad::NotepadApp;
use desktop_app_terminal::TerminalApp;
use leptos::*;
use platform_storage::{self, AppStateEnvelope};
use serde::{Deserialize, Serialize};

const PAINT_PLACEHOLDER_STATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppDescriptor {
    pub app_id: AppId,
    pub launcher_label: &'static str,
    pub desktop_icon_label: &'static str,
    pub show_in_launcher: bool,
    pub show_on_desktop: bool,
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

pub fn app_registry() -> &'static [AppDescriptor] {
    &APP_REGISTRY
}

pub fn launcher_apps() -> Vec<AppDescriptor> {
    app_registry()
        .iter()
        .copied()
        .filter(|entry| entry.show_in_launcher)
        .collect()
}

pub fn desktop_icon_apps() -> Vec<AppDescriptor> {
    app_registry()
        .iter()
        .copied()
        .filter(|entry| entry.show_on_desktop)
        .collect()
}

pub fn app_descriptor(app_id: AppId) -> &'static AppDescriptor {
    app_registry()
        .iter()
        .find(|entry| entry.app_id == app_id)
        .expect("app descriptor exists")
}

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

fn restore_paint_placeholder_state(envelope: AppStateEnvelope) -> Option<PaintPlaceholderState> {
    if envelope.envelope_version != platform_storage::APP_STATE_ENVELOPE_VERSION {
        return None;
    }
    if envelope.schema_version > PAINT_PLACEHOLDER_STATE_SCHEMA_VERSION {
        return None;
    }

    let mut restored = serde_json::from_value::<PaintPlaceholderState>(envelope.payload).ok()?;
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
    Some(restored)
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
            match platform_storage::load_app_state_envelope(platform_storage::PAINT_STATE_NAMESPACE)
                .await
            {
                Ok(Some(envelope)) => {
                    if let Some(restored) = restore_paint_placeholder_state(envelope) {
                        if !hydrate_alive.get() {
                            return;
                        }
                        let serialized = serde_json::to_string(&restored).ok();
                        state.set(restored);
                        last_saved.set(serialized);
                    }
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

        let envelope = match platform_storage::build_app_state_envelope(
            platform_storage::PAINT_STATE_NAMESPACE,
            PAINT_PLACEHOLDER_STATE_SCHEMA_VERSION,
            &snapshot,
        ) {
            Ok(envelope) => envelope,
            Err(err) => {
                logging::warn!("paint placeholder envelope build failed: {err}");
                return;
            }
        };

        spawn_local(async move {
            if let Err(err) = platform_storage::save_app_state_envelope(&envelope).await {
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
