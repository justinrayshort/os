//! Desktop app registry metadata and app-content mounting helpers.

use std::sync::OnceLock;

use crate::icons::IconName;
use crate::model::{OpenWindowRequest, DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH};
use desktop_app_calculator::CalculatorApp;
use desktop_app_contract::{
    AppCapability, AppModule, AppMountContext, ApplicationId, SuspendPolicy,
};
use desktop_app_explorer::ExplorerApp;
use desktop_app_notepad::NotepadApp;
use desktop_app_settings::SettingsApp;
use desktop_app_terminal::TerminalApp;
use leptos::*;
use serde::{Deserialize, Serialize};
const APP_ID_CALCULATOR: &str = "system.calculator";
const APP_ID_EXPLORER: &str = "system.explorer";
const APP_ID_NOTEPAD: &str = "system.notepad";
const APP_ID_PAINT: &str = "system.paint";
const APP_ID_TERMINAL: &str = "system.terminal";
const APP_ID_SETTINGS: &str = "system.settings";
const APP_ID_DIALUP: &str = "system.dialup";

#[derive(Debug, Clone, Copy)]
struct GeneratedAppManifestMetadata {
    display_name: &'static str,
    requested_capabilities: &'static [AppCapability],
    single_instance: bool,
    suspend_policy: SuspendPolicy,
    show_in_launcher: bool,
    show_on_desktop: bool,
    window_defaults: (i32, i32),
}

include!(concat!(env!("OUT_DIR"), "/app_catalog_generated.rs"));

fn builtin_app_id(raw: &'static str) -> ApplicationId {
    ApplicationId::trusted(raw)
}

/// Returns the generated manifest catalog payload used for build-time discovery validation.
pub fn app_manifest_catalog_json() -> &'static str {
    APP_MANIFEST_CATALOG_JSON
}

#[derive(Debug, Clone)]
/// Metadata describing how an app appears in the launcher/desktop and how it is instantiated.
pub struct AppDescriptor {
    /// Stable runtime application identifier.
    pub app_id: ApplicationId,
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
    /// Declared capability scopes requested by the app.
    pub requested_capabilities: &'static [AppCapability],
}

fn build_app_registry() -> Vec<AppDescriptor> {
    vec![
        AppDescriptor {
            app_id: builtin_app_id(APP_ID_CALCULATOR),
            launcher_label: SYSTEM_CALCULATOR_MANIFEST.display_name,
            desktop_icon_label: SYSTEM_CALCULATOR_MANIFEST.display_name,
            show_in_launcher: SYSTEM_CALCULATOR_MANIFEST.show_in_launcher,
            show_on_desktop: SYSTEM_CALCULATOR_MANIFEST.show_on_desktop,
            single_instance: SYSTEM_CALCULATOR_MANIFEST.single_instance,
            module: AppModule::new(mount_calculator_app),
            suspend_policy: SYSTEM_CALCULATOR_MANIFEST.suspend_policy,
            requested_capabilities: SYSTEM_CALCULATOR_MANIFEST.requested_capabilities,
        },
        AppDescriptor {
            app_id: builtin_app_id(APP_ID_EXPLORER),
            launcher_label: SYSTEM_EXPLORER_MANIFEST.display_name,
            desktop_icon_label: SYSTEM_EXPLORER_MANIFEST.display_name,
            show_in_launcher: SYSTEM_EXPLORER_MANIFEST.show_in_launcher,
            show_on_desktop: SYSTEM_EXPLORER_MANIFEST.show_on_desktop,
            single_instance: SYSTEM_EXPLORER_MANIFEST.single_instance,
            module: AppModule::new(mount_explorer_app),
            suspend_policy: SYSTEM_EXPLORER_MANIFEST.suspend_policy,
            requested_capabilities: SYSTEM_EXPLORER_MANIFEST.requested_capabilities,
        },
        AppDescriptor {
            app_id: builtin_app_id(APP_ID_NOTEPAD),
            launcher_label: SYSTEM_NOTEPAD_MANIFEST.display_name,
            desktop_icon_label: "Notes",
            show_in_launcher: SYSTEM_NOTEPAD_MANIFEST.show_in_launcher,
            show_on_desktop: SYSTEM_NOTEPAD_MANIFEST.show_on_desktop,
            single_instance: SYSTEM_NOTEPAD_MANIFEST.single_instance,
            module: AppModule::new(mount_notepad_app),
            suspend_policy: SYSTEM_NOTEPAD_MANIFEST.suspend_policy,
            requested_capabilities: SYSTEM_NOTEPAD_MANIFEST.requested_capabilities,
        },
        AppDescriptor {
            app_id: builtin_app_id(APP_ID_PAINT),
            launcher_label: "Paint",
            desktop_icon_label: "Paint",
            show_in_launcher: true,
            show_on_desktop: false,
            single_instance: false,
            module: AppModule::new(mount_paint_placeholder_app),
            suspend_policy: SuspendPolicy::OnMinimize,
            requested_capabilities: &[AppCapability::Window, AppCapability::State],
        },
        AppDescriptor {
            app_id: builtin_app_id(APP_ID_TERMINAL),
            launcher_label: SYSTEM_TERMINAL_MANIFEST.display_name,
            desktop_icon_label: SYSTEM_TERMINAL_MANIFEST.display_name,
            show_in_launcher: SYSTEM_TERMINAL_MANIFEST.show_in_launcher,
            show_on_desktop: SYSTEM_TERMINAL_MANIFEST.show_on_desktop,
            single_instance: SYSTEM_TERMINAL_MANIFEST.single_instance,
            module: AppModule::new(mount_terminal_app),
            suspend_policy: SYSTEM_TERMINAL_MANIFEST.suspend_policy,
            requested_capabilities: SYSTEM_TERMINAL_MANIFEST.requested_capabilities,
        },
        AppDescriptor {
            app_id: builtin_app_id(APP_ID_SETTINGS),
            launcher_label: SYSTEM_SETTINGS_MANIFEST.display_name,
            desktop_icon_label: "Settings",
            show_in_launcher: SYSTEM_SETTINGS_MANIFEST.show_in_launcher,
            show_on_desktop: SYSTEM_SETTINGS_MANIFEST.show_on_desktop,
            single_instance: SYSTEM_SETTINGS_MANIFEST.single_instance,
            module: AppModule::new(mount_settings_app),
            suspend_policy: SYSTEM_SETTINGS_MANIFEST.suspend_policy,
            requested_capabilities: SYSTEM_SETTINGS_MANIFEST.requested_capabilities,
        },
        AppDescriptor {
            app_id: builtin_app_id(APP_ID_DIALUP),
            launcher_label: "Dial-up",
            desktop_icon_label: "Connect",
            show_in_launcher: true,
            show_on_desktop: false,
            single_instance: false,
            module: AppModule::new(mount_dialup_placeholder_app),
            suspend_policy: SuspendPolicy::OnMinimize,
            requested_capabilities: &[AppCapability::Window],
        },
    ]
}

fn app_registry_storage() -> &'static OnceLock<Vec<AppDescriptor>> {
    static APP_REGISTRY: OnceLock<Vec<AppDescriptor>> = OnceLock::new();
    &APP_REGISTRY
}

const BUILTIN_PRIVILEGED_APP_IDS: &[&str] = &["system.settings"];
const LEGACY_BUILTIN_APP_ID_MAPPINGS: &[(&str, &str)] = &[
    ("Calculator", APP_ID_CALCULATOR),
    ("Explorer", APP_ID_EXPLORER),
    ("Notepad", APP_ID_NOTEPAD),
    ("Paint", APP_ID_PAINT),
    ("Terminal", APP_ID_TERMINAL),
    ("Settings", APP_ID_SETTINGS),
    ("Dialup", APP_ID_DIALUP),
];

/// Returns the static app registry used by the desktop shell.
pub fn app_registry() -> &'static [AppDescriptor] {
    app_registry_storage()
        .get_or_init(build_app_registry)
        .as_slice()
}

/// Returns app descriptors that should appear in launcher menus.
pub fn launcher_apps() -> Vec<AppDescriptor> {
    app_registry()
        .iter()
        .cloned()
        .filter(|entry| entry.show_in_launcher)
        .collect()
}

/// Returns app descriptors that should appear as desktop icons.
pub fn desktop_icon_apps() -> Vec<AppDescriptor> {
    app_registry()
        .iter()
        .cloned()
        .filter(|entry| entry.show_on_desktop)
        .collect()
}

/// Returns the descriptor for a canonical application id.
///
/// # Panics
///
/// Panics if the app id is not present in the registry.
pub fn app_descriptor_by_id(app_id: &ApplicationId) -> &'static AppDescriptor {
    app_registry()
        .iter()
        .find(|entry| &entry.app_id == app_id)
        .expect("app descriptor exists")
}

/// Returns the managed app module descriptor for one canonical app id.
pub fn app_module_by_id(app_id: &ApplicationId) -> AppModule {
    app_descriptor_by_id(app_id).module
}

/// Returns the window-manager suspend policy for one canonical app id.
pub fn app_suspend_policy_by_id(app_id: &ApplicationId) -> SuspendPolicy {
    app_descriptor_by_id(app_id).suspend_policy
}

/// Returns declared capability scopes for one canonical app id.
pub fn app_requested_capabilities_by_id(app_id: &ApplicationId) -> &'static [AppCapability] {
    app_descriptor_by_id(app_id).requested_capabilities
}

/// Returns whether `app_id` is privileged in shell policy.
pub fn app_is_privileged_by_id(app_id: &ApplicationId) -> bool {
    BUILTIN_PRIVILEGED_APP_IDS
        .iter()
        .any(|id| *id == app_id.as_str())
}

/// Parses a canonical or legacy serialized app id into an [`ApplicationId`].
pub fn parse_application_id_compat(raw: &str) -> Option<ApplicationId> {
    ApplicationId::new(raw.trim()).ok().or_else(|| {
        LEGACY_BUILTIN_APP_ID_MAPPINGS
            .iter()
            .find_map(|(legacy, canonical)| (*legacy == raw.trim()).then_some(*canonical))
            .map(ApplicationId::trusted)
    })
}

/// Returns the shell title for one canonical app id.
pub fn app_title_by_id(app_id: &ApplicationId) -> &'static str {
    app_descriptor_by_id(app_id).launcher_label
}

/// Returns the default icon id string for one canonical app id.
pub fn app_icon_id_by_id(app_id: &ApplicationId) -> &'static str {
    match app_id.as_str() {
        APP_ID_CALCULATOR => "calculator",
        APP_ID_EXPLORER => "folder",
        APP_ID_NOTEPAD => "notepad",
        APP_ID_PAINT => "paint",
        APP_ID_TERMINAL => "terminal",
        APP_ID_SETTINGS => "settings",
        APP_ID_DIALUP => "modem",
        _ => "window",
    }
}

/// Returns the semantic shell icon for one canonical app id.
pub fn app_icon_name_by_id(app_id: &ApplicationId) -> IconName {
    match app_id.as_str() {
        APP_ID_CALCULATOR => IconName::Calculator,
        APP_ID_EXPLORER => IconName::ExplorerFolder,
        APP_ID_NOTEPAD => IconName::DocumentText,
        APP_ID_PAINT => IconName::PaintBrush,
        APP_ID_TERMINAL => IconName::Terminal,
        APP_ID_SETTINGS => IconName::Settings,
        APP_ID_DIALUP => IconName::Connect,
        _ => IconName::WindowMultiple,
    }
}

/// Returns the canonical system settings application id.
pub fn settings_application_id() -> ApplicationId {
    builtin_app_id(APP_ID_SETTINGS)
}

/// Returns whether `app_id` refers to the built-in dial-up app.
pub fn is_dialup_application_id(app_id: &ApplicationId) -> bool {
    app_id.as_str() == APP_ID_DIALUP
}

/// Returns the canonical pinned taskbar application ids in display order.
pub fn pinned_taskbar_app_ids() -> Vec<ApplicationId> {
    [
        APP_ID_EXPLORER,
        APP_ID_TERMINAL,
        APP_ID_NOTEPAD,
        APP_ID_CALCULATOR,
    ]
    .into_iter()
    .map(builtin_app_id)
    .collect()
}

/// Builds the default [`OpenWindowRequest`] for a canonical application id.
pub fn default_open_request_by_id(
    app_id: &ApplicationId,
    viewport: Option<crate::model::WindowRect>,
) -> Option<OpenWindowRequest> {
    app_registry()
        .iter()
        .any(|entry| entry.app_id == *app_id)
        .then(|| {
            let mut req = OpenWindowRequest::new(app_id.clone());
            req.rect = Some(default_window_rect_for_app(app_id, viewport));
            req.viewport = viewport;
            req
        })
}

fn default_window_rect_for_app(
    app_id: &ApplicationId,
    viewport: Option<crate::model::WindowRect>,
) -> crate::model::WindowRect {
    let vp = viewport.unwrap_or(crate::model::WindowRect {
        x: 0,
        y: 0,
        w: 1280,
        h: 760,
    });

    let (min_w, min_h, max_w_ratio, max_h_ratio, default_w_ratio, default_h_ratio) =
        match app_id.as_str() {
            APP_ID_EXPLORER => (
                SYSTEM_EXPLORER_MANIFEST.window_defaults.0,
                SYSTEM_EXPLORER_MANIFEST.window_defaults.1,
                0.92,
                0.92,
                0.80,
                0.78,
            ),
            APP_ID_NOTEPAD => (
                SYSTEM_NOTEPAD_MANIFEST.window_defaults.0,
                SYSTEM_NOTEPAD_MANIFEST.window_defaults.1,
                0.88,
                0.88,
                0.74,
                0.74,
            ),
            APP_ID_TERMINAL => (
                SYSTEM_TERMINAL_MANIFEST.window_defaults.0,
                SYSTEM_TERMINAL_MANIFEST.window_defaults.1,
                0.88,
                0.86,
                0.74,
                0.70,
            ),
            APP_ID_SETTINGS => (
                SYSTEM_SETTINGS_MANIFEST.window_defaults.0,
                SYSTEM_SETTINGS_MANIFEST.window_defaults.1,
                0.92,
                0.92,
                0.82,
                0.82,
            ),
            APP_ID_CALCULATOR => (
                SYSTEM_CALCULATOR_MANIFEST.window_defaults.0,
                SYSTEM_CALCULATOR_MANIFEST.window_defaults.1,
                0.78,
                0.86,
                0.56,
                0.74,
            ),
            APP_ID_PAINT => (620, 420, 0.92, 0.92, 0.78, 0.78),
            APP_ID_DIALUP => (420, 300, 0.66, 0.68, 0.48, 0.50),
            _ => (
                DEFAULT_WINDOW_WIDTH,
                DEFAULT_WINDOW_HEIGHT,
                0.80,
                0.80,
                0.70,
                0.70,
            ),
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
    fn default_open_request_by_id_scales_to_viewport() {
        let viewport = crate::model::WindowRect {
            x: 0,
            y: 0,
            w: 900,
            h: 620,
        };
        let req = default_open_request_by_id(&builtin_app_id(APP_ID_EXPLORER), Some(viewport))
            .expect("default request");
        let rect = req.rect.expect("default rect");

        assert!(rect.w <= ((viewport.w as f32) * 0.92) as i32);
        assert!(rect.h <= ((viewport.h as f32) * 0.92) as i32);
        assert!(rect.w >= 620);
        assert!(rect.h >= 420);
    }

    #[test]
    fn calculator_defaults_are_more_compact_than_explorer_by_id() {
        let viewport = crate::model::WindowRect {
            x: 0,
            y: 0,
            w: 1280,
            h: 760,
        };
        let calc = default_open_request_by_id(&builtin_app_id(APP_ID_CALCULATOR), Some(viewport))
            .expect("calculator request")
            .rect
            .expect("calculator rect");
        let explorer = default_open_request_by_id(&builtin_app_id(APP_ID_EXPLORER), Some(viewport))
            .expect("explorer request")
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
            services=Some(context.services)
        />
    }
    .into_view()
}

fn mount_explorer_app(context: AppMountContext) -> View {
    view! {
        <ExplorerApp
            launch_params=context.launch_params.clone()
            restored_state=Some(context.restored_state.clone())
            services=Some(context.services)
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
            services=Some(context.services)
        />
    }
    .into_view()
}

fn mount_terminal_app(context: AppMountContext) -> View {
    view! {
        <TerminalApp
            window_id=context.window_id
            launch_params=context.launch_params.clone()
            restored_state=Some(context.restored_state.clone())
            services=Some(context.services)
        />
    }
    .into_view()
}

fn mount_settings_app(context: AppMountContext) -> View {
    view! {
        <SettingsApp
            _launch_params=context.launch_params.clone()
            restored_state=Some(context.restored_state.clone())
            services=Some(context.services)
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
