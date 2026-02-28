//! Built-in placeholder app implementations used while fuller apps are still in development.

use desktop_app_contract::AppMountContext;
use leptos::*;
use serde::{Deserialize, Serialize};

/// Mounts the Paint placeholder app.
pub(super) fn mount_paint_placeholder_app(context: AppMountContext) -> View {
    view! { <PaintPlaceholderApp context=context /> }.into_view()
}

/// Mounts the Dial-up placeholder app.
pub(super) fn mount_dialup_placeholder_app(_: AppMountContext) -> View {
    view! {
        <div class="app-shell app-dialup-shell">
            <div class="app-toolbar" role="group" aria-label="Dial-up placeholder controls">
                <button type="button" class="app-action">"Connect"</button>
                <button type="button" class="app-action">"Disconnect"</button>
                <button type="button" class="app-action">"Retry"</button>
            </div>
            <div class="app-dialup-card">
                <p><strong>"Dial-up (Placeholder)"</strong></p>
                <p>"Negotiating connection..."</p>
                <progress class="app-progress" max="100" value="45"></progress>
            </div>
            <div class="app-statusbar">
                <span>"Status: connecting"</span>
                <span>"Carrier: simulated 56k"</span>
                <span>"Progress: 45%"</span>
            </div>
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
fn PaintPlaceholderApp(context: AppMountContext) -> impl IntoView {
    let state = create_rw_signal(PaintPlaceholderState::default());
    let hydrated = create_rw_signal(false);
    let last_saved = create_rw_signal::<Option<String>>(None);

    create_effect(move |_| {
        if context.restored_state.is_object() {
            if let Ok(restored) =
                serde_json::from_value::<PaintPlaceholderState>(context.restored_state.clone())
            {
                let restored = restore_paint_placeholder_state(restored);
                let serialized = serde_json::to_string(&restored).ok();
                state.set(restored);
                last_saved.set(serialized);
            }
        }
    });

    hydrated.set(true);

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

        // Manager-owned app state path.
        if let Ok(value) = serde_json::to_value(&snapshot) {
            context.services.state.persist_window_state(value);
        }
    });

    view! {
        <div class="app-shell app-paint-shell">
            <div class="app-toolbar app-paint-intro" role="note">
                <strong>"Paint (Placeholder)"</strong>
                <span>"Future canvas state persists through the desktop runtime window snapshot."</span>
            </div>

            <div class="app-toolbar" role="group" aria-label="Paint placeholder controls">
                <label>
                    "Tool "
                    <select
                        class="app-field"
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
                        class="app-field"
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
                        class="app-field"
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
                        class="app-field"
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

                <button type="button" class="app-action" on:click=move |_| {
                    state.update(|s| s.status = "Placeholder save slot synced to IndexedDB".to_string());
                }>
                    "Save Slot"
                </button>
                <button type="button" class="app-action" on:click=move |_| {
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
                <span>{move || state.get().status.clone()}</span>
            </div>
        </div>
    }
}
