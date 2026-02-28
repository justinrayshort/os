//! Built-in lightweight utility app implementations used until fuller apps exist.

use desktop_app_contract::AppMountContext;
use leptos::*;
use serde::{Deserialize, Serialize};
use system_ui::prelude::*;

/// Mounts the Paint utility app.
pub(super) fn mount_paint_placeholder_app(context: AppMountContext) -> View {
    view! { <PaintUtilityApp context=context /> }.into_view()
}

/// Mounts the Dial-up networking setup utility app.
pub(super) fn mount_dialup_placeholder_app(context: AppMountContext) -> View {
    view! { <DialupUtilityApp context=context /> }.into_view()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct DialupUtilityState {
    provider: String,
    profile: String,
    status: String,
    connected: bool,
}

impl Default for DialupUtilityState {
    fn default() -> Self {
        Self {
            provider: "Workstation ISP".to_string(),
            profile: "Default".to_string(),
            status: "Ready to connect".to_string(),
            connected: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PaintUtilityState {
    tool: String,
    brush_size: u8,
    color_hex: String,
    canvas_preset: String,
    marks: Vec<String>,
    status: String,
}

impl Default for PaintUtilityState {
    fn default() -> Self {
        Self {
            tool: "brush".to_string(),
            brush_size: 6,
            color_hex: "#0b5fff".to_string(),
            canvas_preset: "800x600".to_string(),
            marks: Vec::new(),
            status: "Sketch pad ready".to_string(),
        }
    }
}

#[component]
fn DialupUtilityApp(context: AppMountContext) -> impl IntoView {
    let state = create_rw_signal(DialupUtilityState::default());
    hydrate_persisted_state(&context, state);

    view! {
        <AppShell layout_class="app-dialup-shell">
            <ToolBar aria_label="Connection actions">
                <Button
                    variant=ButtonVariant::Primary
                    on_click=Callback::new(move |_| {
                        state.update(|state| {
                            state.connected = true;
                            state.status = format!(
                                "Connected via {} ({})",
                                state.provider, state.profile
                            );
                        });
                    })
                >
                    "Connect"
                </Button>
                <Button
                    variant=ButtonVariant::Quiet
                    on_click=Callback::new(move |_| {
                        state.update(|state| {
                            state.connected = false;
                            state.status = "Disconnected".to_string();
                        });
                    })
                >
                    "Disconnect"
                </Button>
            </ToolBar>

            <Panel layout_class="app-dialup-card" variant=SurfaceVariant::Standard>
                <Stack gap=LayoutGap::Md>
                    <Heading>"Connection Setup"</Heading>
                    <Text tone=TextTone::Secondary>
                        "This minimal utility stores a provider profile and a connection state. It no longer simulates fake negotiation progress."
                    </Text>
                    <Grid layout_class="settings-form-grid">
                        <label class="settings-field">
                            <Text role=TextRole::Label>"Provider"</Text>
                            <TextField
                                value=Signal::derive(move || state.get().provider)
                                on_input=Callback::new(move |ev| {
                                    let value = event_target_value(&ev);
                                    state.update(|state| state.provider = value);
                                })
                            />
                        </label>
                        <label class="settings-field">
                            <Text role=TextRole::Label>"Profile"</Text>
                            <TextField
                                value=Signal::derive(move || state.get().profile)
                                on_input=Callback::new(move |ev| {
                                    let value = event_target_value(&ev);
                                    state.update(|state| state.profile = value);
                                })
                            />
                        </label>
                    </Grid>
                </Stack>
            </Panel>

            <StatusBar>
                <span>{move || if state.get().connected { "Status: connected" } else { "Status: offline" }.to_string()}</span>
                <span>{move || format!("Provider: {}", state.get().provider)}</span>
                <span>{move || state.get().status.clone()}</span>
            </StatusBar>
        </AppShell>
    }
}

#[component]
fn PaintUtilityApp(context: AppMountContext) -> impl IntoView {
    let state = create_rw_signal(PaintUtilityState::default());
    hydrate_persisted_state(&context, state);

    view! {
        <AppShell layout_class="app-paint-shell">
            <ToolBar aria_label="Sketch controls">
                <label>
                    "Tool "
                    <SelectField
                        value=Signal::derive(move || state.get().tool)
                        on_change=Callback::new(move |ev| {
                            let value = event_target_value(&ev);
                            state.update(|state| state.tool = value);
                        })
                    >
                        <option value="brush">"Brush"</option>
                        <option value="line">"Line"</option>
                        <option value="eraser">"Eraser"</option>
                        <option value="marker">"Marker"</option>
                    </SelectField>
                </label>

                <label>
                    "Brush "
                    <RangeField
                        min="1"
                        max="64"
                        value=Signal::derive(move || state.get().brush_size.to_string())
                        on_input=Callback::new(move |ev| {
                            let value = event_target_value(&ev)
                                .parse::<u8>()
                                .unwrap_or(6)
                                .clamp(1, 64);
                            state.update(|state| state.brush_size = value);
                        })
                    />
                </label>

                <label>
                    "Color "
                    <ColorField
                        value=Signal::derive(move || state.get().color_hex)
                        on_input=Callback::new(move |ev| {
                            let value = event_target_value(&ev);
                            state.update(|state| state.color_hex = value);
                        })
                    />
                </label>

                <Button
                    variant=ButtonVariant::Primary
                    on_click=Callback::new(move |_| {
                        state.update(|state| {
                            state.marks.push(format!(
                                "{} • {}px • {}",
                                state.tool, state.brush_size, state.color_hex
                            ));
                            state.status = format!("Added mark {}", state.marks.len());
                        });
                    })
                >
                    "Add Mark"
                </Button>
                <Button
                    variant=ButtonVariant::Quiet
                    on_click=Callback::new(move |_| {
                        state.update(|state| {
                            state.marks.clear();
                            state.status = "Canvas cleared".to_string();
                        });
                    })
                >
                    "Clear"
                </Button>
            </ToolBar>

            <Panel layout_class="app-paint-canvas" variant=SurfaceVariant::Inset elevation=Elevation::Inset>
                <Stack gap=LayoutGap::Md>
                    <Heading>"Sketch Pad"</Heading>
                    <Text tone=TextTone::Secondary>
                        "This lightweight utility keeps a small persistent mark list instead of exposing fake unfinished canvas controls."
                    </Text>
                    <div class="paint-mark-list" role="list">
                        <Show when=move || !state.get().marks.is_empty() fallback=|| {
                            view! { <Text tone=TextTone::Secondary>"No marks yet. Add one to capture the current tool, brush, and color."</Text> }
                        }>
                            <For each=move || state.get().marks key=|mark| mark.clone() let:mark>
                                <Surface layout_class="paint-mark" variant=SurfaceVariant::Muted elevation=Elevation::Raised>
                                    <Text role=TextRole::Code>{mark}</Text>
                                </Surface>
                            </For>
                        </Show>
                    </div>
                </Stack>
            </Panel>

            <StatusBar>
                <span>{move || format!("Tool: {}", state.get().tool)}</span>
                <span>{move || format!("Brush: {} px | {}", state.get().brush_size, state.get().color_hex)}</span>
                <span>{move || state.get().status.clone()}</span>
            </StatusBar>
        </AppShell>
    }
}

fn hydrate_persisted_state<T>(context: &AppMountContext, state: RwSignal<T>)
where
    T: Clone + for<'de> Deserialize<'de> + Serialize + 'static,
{
    let restored_state = context.restored_state.clone();
    let services = context.services.clone();
    let last_saved = create_rw_signal::<Option<String>>(None);
    let hydrated = create_rw_signal(false);

    create_effect(move |_| {
        if restored_state.is_object() {
            if let Ok(restored) = serde_json::from_value::<T>(restored_state.clone()) {
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
                logging::warn!("utility app serialize failed: {err}");
                return;
            }
        };

        if last_saved.get().as_deref() == Some(serialized.as_str()) {
            return;
        }
        last_saved.set(Some(serialized));

        if let Ok(value) = serde_json::to_value(snapshot) {
            services.state.persist_window_state(value);
        }
    });
}
