//! Desktop app registry metadata and app-content mounting helpers.

use std::{cell::Cell, rc::Rc};

use crate::model::{AppId, OpenWindowRequest};
use desktop_app_calculator::CalculatorApp;
use desktop_app_contract::{AppModule, AppMountContext, SuspendPolicy};
use desktop_app_explorer::ExplorerApp;
use desktop_app_notepad::NotepadApp;
use desktop_app_settings::SettingsApp;
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

#[derive(Debug, Clone, Copy)]
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
    /// Managed app module mount descriptor.
    pub module: AppModule,
    /// Suspend policy applied by the desktop window manager.
    pub suspend_policy: SuspendPolicy,
}

const APP_REGISTRY: [AppDescriptor; 7] = [
    AppDescriptor {
        app_id: AppId::Calculator,
        launcher_label: "Calculator",
        desktop_icon_label: "Calculator",
        show_in_launcher: true,
        show_on_desktop: true,
        single_instance: true,
        module: AppModule::new(mount_calculator_app),
        suspend_policy: SuspendPolicy::OnMinimize,
    },
    AppDescriptor {
        app_id: AppId::Explorer,
        launcher_label: "Explorer",
        desktop_icon_label: "Explorer",
        show_in_launcher: true,
        show_on_desktop: true,
        single_instance: false,
        module: AppModule::new(mount_explorer_app),
        suspend_policy: SuspendPolicy::OnMinimize,
    },
    AppDescriptor {
        app_id: AppId::Notepad,
        launcher_label: "Notepad",
        desktop_icon_label: "Notes",
        show_in_launcher: true,
        show_on_desktop: false,
        single_instance: false,
        module: AppModule::new(mount_notepad_app),
        suspend_policy: SuspendPolicy::OnMinimize,
    },
    AppDescriptor {
        app_id: AppId::Paint,
        launcher_label: "Paint",
        desktop_icon_label: "Paint",
        show_in_launcher: true,
        show_on_desktop: false,
        single_instance: false,
        module: AppModule::new(mount_paint_placeholder_app),
        suspend_policy: SuspendPolicy::OnMinimize,
    },
    AppDescriptor {
        app_id: AppId::Terminal,
        launcher_label: "Terminal",
        desktop_icon_label: "Terminal",
        show_in_launcher: true,
        show_on_desktop: true,
        single_instance: true,
        module: AppModule::new(mount_terminal_app),
        suspend_policy: SuspendPolicy::Never,
    },
    AppDescriptor {
        app_id: AppId::Settings,
        launcher_label: "System Settings",
        desktop_icon_label: "Settings",
        show_in_launcher: true,
        show_on_desktop: false,
        single_instance: true,
        module: AppModule::new(mount_settings_app),
        suspend_policy: SuspendPolicy::OnMinimize,
    },
    AppDescriptor {
        app_id: AppId::Dialup,
        launcher_label: "Dial-up",
        desktop_icon_label: "Connect",
        show_in_launcher: true,
        show_on_desktop: false,
        single_instance: false,
        module: AppModule::new(mount_dialup_placeholder_app),
        suspend_policy: SuspendPolicy::OnMinimize,
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

/// Returns the managed app module descriptor for `app_id`.
pub fn app_module(app_id: AppId) -> AppModule {
    app_descriptor(app_id).module
}

/// Returns the window-manager suspend policy for `app_id`.
pub fn app_suspend_policy(app_id: AppId) -> SuspendPolicy {
    app_descriptor(app_id).suspend_policy
}

/// Builds the default [`OpenWindowRequest`] for a given app.
///
/// Some apps override the default geometry to better fit their UI.
///
/// When `viewport` is provided, the returned request uses adaptive default sizing and placement
/// heuristics so app windows open with readable dimensions while respecting available space.
pub fn default_open_request(
    app_id: AppId,
    viewport: Option<crate::model::WindowRect>,
) -> OpenWindowRequest {
    let mut req = OpenWindowRequest::new(app_id);
    req.rect = Some(default_window_rect_for_app(app_id, viewport));
    req.viewport = viewport;
    req
}

fn default_window_rect_for_app(
    app_id: AppId,
    viewport: Option<crate::model::WindowRect>,
) -> crate::model::WindowRect {
    let vp = viewport.unwrap_or(crate::model::WindowRect {
        x: 0,
        y: 0,
        w: 1280,
        h: 760,
    });

    let (min_w, min_h, max_w_ratio, max_h_ratio, default_w_ratio, default_h_ratio) = match app_id {
        AppId::Explorer => (620, 420, 0.92, 0.92, 0.80, 0.78),
        AppId::Notepad => (560, 380, 0.88, 0.88, 0.74, 0.74),
        AppId::Terminal => (560, 360, 0.88, 0.86, 0.74, 0.70),
        AppId::Settings => (680, 480, 0.92, 0.92, 0.82, 0.82),
        AppId::Calculator => (460, 360, 0.78, 0.86, 0.56, 0.74),
        AppId::Paint => (620, 420, 0.92, 0.92, 0.78, 0.78),
        AppId::Dialup => (420, 300, 0.66, 0.68, 0.48, 0.50),
    };

    let max_w = ((vp.w as f32) * max_w_ratio) as i32;
    let max_h = ((vp.h as f32) * max_h_ratio) as i32;
    let w = (((vp.w as f32) * default_w_ratio) as i32).clamp(min_w, max_w.max(min_w));
    let h = (((vp.h as f32) * default_h_ratio) as i32).clamp(min_h, max_h.max(min_h));
    let x = vp.x + ((vp.w - w) / 2).max(10);
    let y = vp.y + ((vp.h - h) / 2).max(10);

    crate::model::WindowRect { x, y, w, h }
}

#[cfg(test)]
mod default_open_request_tests {
    use super::*;

    #[test]
    fn default_open_request_scales_to_viewport() {
        let viewport = crate::model::WindowRect {
            x: 0,
            y: 0,
            w: 900,
            h: 620,
        };
        let req = default_open_request(AppId::Explorer, Some(viewport));
        let rect = req.rect.expect("default rect");

        assert!(rect.w <= ((viewport.w as f32) * 0.92) as i32);
        assert!(rect.h <= ((viewport.h as f32) * 0.92) as i32);
        assert!(rect.w >= 620);
        assert!(rect.h >= 420);
    }

    #[test]
    fn calculator_defaults_are_more_compact_than_explorer() {
        let viewport = crate::model::WindowRect {
            x: 0,
            y: 0,
            w: 1280,
            h: 760,
        };
        let calc = default_open_request(AppId::Calculator, Some(viewport))
            .rect
            .expect("calculator rect");
        let explorer = default_open_request(AppId::Explorer, Some(viewport))
            .rect
            .expect("explorer rect");

        assert!(calc.w < explorer.w);
        assert!(calc.h <= explorer.h);
    }
}

fn mount_calculator_app(context: AppMountContext) -> View {
    view! {
        <CalculatorApp
            launch_params=context.launch_params.clone()
            restored_state=Some(context.restored_state.clone())
            host=Some(context.host)
        />
    }
    .into_view()
}

fn mount_explorer_app(context: AppMountContext) -> View {
    view! {
        <ExplorerApp
            launch_params=context.launch_params.clone()
            restored_state=Some(context.restored_state.clone())
            host=Some(context.host)
            inbox=Some(context.inbox)
        />
    }
    .into_view()
}

fn mount_notepad_app(context: AppMountContext) -> View {
    view! {
        <NotepadApp
            launch_params=context.launch_params.clone()
            restored_state=Some(context.restored_state.clone())
            host=Some(context.host)
        />
    }
    .into_view()
}

fn mount_terminal_app(context: AppMountContext) -> View {
    view! {
        <TerminalApp
            launch_params=context.launch_params.clone()
            restored_state=Some(context.restored_state.clone())
            host=Some(context.host)
        />
    }
    .into_view()
}

fn mount_settings_app(context: AppMountContext) -> View {
    view! {
        <SettingsApp
            launch_params=context.launch_params.clone()
            restored_state=Some(context.restored_state.clone())
            host=Some(context.host)
        />
    }
    .into_view()
}

fn mount_paint_placeholder_app(context: AppMountContext) -> View {
    view! { <PaintPlaceholderApp context=context /> }.into_view()
}

fn mount_dialup_placeholder_app(_: AppMountContext) -> View {
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
    let hydrate_alive = Rc::new(Cell::new(true));
    on_cleanup({
        let hydrate_alive = hydrate_alive.clone();
        move || hydrate_alive.set(false)
    });

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
                    if last_saved.get_untracked().is_none() {
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

        // Manager-owned app state path.
        if let Ok(value) = serde_json::to_value(&snapshot) {
            context.host.persist_state(value);
        }

        // Legacy namespace persistence retained for migration compatibility.
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
        <div class="app-shell app-paint-shell">
            <div class="app-toolbar app-paint-intro" role="note">
                <strong>"Paint (Placeholder)"</strong>
                <span>"Future canvas state persists under the same namespaced IndexedDB path."</span>
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
