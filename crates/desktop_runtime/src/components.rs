//! Leptos provider, context, and desktop shell UI composition.

mod a11y;
mod display_properties;
mod menus;
mod taskbar;
mod taskbar_input;
mod window;

use std::time::Duration;

use leptos::*;

use self::{
    a11y::{
        active_html_element, focus_element_by_id, focus_first_menu_item, focus_html_element,
        handle_menu_roving_keydown, trap_tab_focus,
    },
    display_properties::DisplayPropertiesDialog,
    menus::DesktopContextMenu,
    taskbar::Taskbar,
    taskbar_input::{is_activation_key, is_context_menu_shortcut, try_handle_taskbar_shortcuts},
    window::DesktopWindow,
};

use crate::{
    app_runtime::{sync_runtime_sessions, AppRuntimeState},
    apps,
    host::DesktopHostContext,
    icons::{app_icon_name, FluentIcon, IconName, IconSize},
    model::{
        AppId, DesktopState, InteractionState, PointerPosition, ResizeEdge, WindowId, WindowRecord,
    },
    reducer::{reduce_desktop, DesktopAction, RuntimeEffect},
};

const TASKBAR_HEIGHT_PX: i32 = 38;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DesktopContextMenuState {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WallpaperPresetKind {
    Pattern,
    Picture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WallpaperPreset {
    id: &'static str,
    label: &'static str,
    kind: WallpaperPresetKind,
    note: &'static str,
}

fn desktop_wallpaper_presets() -> &'static [WallpaperPreset] {
    const PRESETS: &[WallpaperPreset] = &[
        WallpaperPreset {
            id: "teal-solid",
            label: "Solid Teal",
            kind: WallpaperPresetKind::Pattern,
            note: "Classic single-color desktop fill",
        },
        WallpaperPreset {
            id: "teal-grid",
            label: "Teal Grid",
            kind: WallpaperPresetKind::Pattern,
            note: "Subtle tiled grid on teal",
        },
        WallpaperPreset {
            id: "woven-steel",
            label: "Woven Steel",
            kind: WallpaperPresetKind::Pattern,
            note: "Crosshatch weave pattern",
        },
        WallpaperPreset {
            id: "cloud-bands",
            label: "Cloud Bands",
            kind: WallpaperPresetKind::Picture,
            note: "Soft sky bands and clouds",
        },
        WallpaperPreset {
            id: "green-hills",
            label: "Green Hills",
            kind: WallpaperPresetKind::Picture,
            note: "Rolling hills and blue sky",
        },
        WallpaperPreset {
            id: "sunset-lake",
            label: "Sunset Lake",
            kind: WallpaperPresetKind::Picture,
            note: "Warm dusk landscape scene",
        },
    ];
    PRESETS
}

fn wallpaper_preset_kind_label(kind: WallpaperPresetKind) -> &'static str {
    match kind {
        WallpaperPresetKind::Pattern => "Pattern",
        WallpaperPresetKind::Picture => "Picture",
    }
}

fn wallpaper_preset_by_id(id: &str) -> WallpaperPreset {
    let normalized = match id {
        // Backward-compatible alias for older snapshots.
        "slate-grid" => "teal-grid",
        _ => id,
    };

    desktop_wallpaper_presets()
        .iter()
        .copied()
        .find(|preset| preset.id == normalized)
        .unwrap_or_else(|| desktop_wallpaper_presets()[0])
}

fn wallpaper_option_dom_id(id: &str) -> String {
    format!("wallpaper-option-{id}")
}

fn taskbar_window_button_dom_id(window_id: WindowId) -> String {
    format!("taskbar-window-button-{}", window_id.0)
}

#[derive(Clone, Copy)]
/// Leptos context for reading desktop runtime state and dispatching [`DesktopAction`] values.
pub struct DesktopRuntimeContext {
    /// Host service bundle for executing runtime side effects and environment queries.
    pub host: DesktopHostContext,
    /// Reactive desktop state signal.
    pub state: RwSignal<DesktopState>,
    /// Reactive pointer/drag/resize interaction state signal.
    pub interaction: RwSignal<InteractionState>,
    /// Queue of runtime effects emitted by the reducer and processed by the shell.
    pub effects: RwSignal<Vec<RuntimeEffect>>,
    /// Runtime app-session and pub/sub state.
    pub app_runtime: RwSignal<AppRuntimeState>,
    /// Reducer dispatch callback.
    pub dispatch: Callback<DesktopAction>,
}

#[derive(Clone, Copy)]
struct DesktopShellUiContext {
    display_properties_open: RwSignal<bool>,
}

impl DesktopRuntimeContext {
    /// Dispatches a reducer action through the runtime context callback.
    pub fn dispatch_action(&self, action: DesktopAction) {
        self.dispatch.call(action);
    }
}

#[component]
/// Provides [`DesktopRuntimeContext`] to descendant components and boots persisted state.
pub fn DesktopProvider(children: Children) -> impl IntoView {
    let host = DesktopHostContext::default();
    let state = create_rw_signal(DesktopState::default());
    let interaction = create_rw_signal(InteractionState::default());
    let effects = create_rw_signal(Vec::<RuntimeEffect>::new());
    let app_runtime = create_rw_signal(AppRuntimeState::default());

    let dispatch = Callback::new(move |action: DesktopAction| {
        let mut reducer_outcome = None;

        state.update(|desktop| {
            interaction.update(|ui| {
                reducer_outcome = Some(reduce_desktop(desktop, ui, action));
            });
        });

        match reducer_outcome.expect("desktop reducer executed") {
            Ok(new_effects) => {
                if !new_effects.is_empty() {
                    let mut queue = effects.get_untracked();
                    queue.extend(new_effects);
                    effects.set(queue);
                }
            }
            Err(err) => logging::warn!("desktop reducer error: {err}"),
        }
    });

    provide_context(DesktopRuntimeContext {
        host,
        state,
        interaction,
        effects,
        app_runtime,
        dispatch,
    });

    host.install_boot_hydration(dispatch);

    children().into_view()
}

/// Returns the current [`DesktopRuntimeContext`].
///
/// # Panics
///
/// Panics if called outside [`DesktopProvider`].
pub fn use_desktop_runtime() -> DesktopRuntimeContext {
    use_context::<DesktopRuntimeContext>().expect("DesktopRuntimeContext not provided")
}

#[component]
/// Renders the full desktop shell UI and processes queued [`RuntimeEffect`] values.
pub fn DesktopShell() -> impl IntoView {
    let runtime = use_desktop_runtime();
    let state = runtime.state;
    let desktop_context_menu = create_rw_signal(None::<DesktopContextMenuState>);
    let desktop_context_menu_was_open = create_rw_signal(false);
    let display_properties_open = create_rw_signal(false);
    let wallpaper_selection = create_rw_signal(
        wallpaper_preset_by_id(&state.get_untracked().theme.wallpaper_id)
            .id
            .to_string(),
    );
    let display_properties_original_wallpaper = create_rw_signal(None::<String>);
    let display_properties_last_focused = create_rw_signal(None::<web_sys::HtmlElement>);
    let display_properties_was_open = create_rw_signal(false);
    provide_context(DesktopShellUiContext {
        display_properties_open,
    });

    create_effect(move |_| {
        let active_wallpaper = wallpaper_preset_by_id(&state.get().theme.wallpaper_id);
        if !display_properties_open.get() {
            wallpaper_selection.set(active_wallpaper.id.to_string());
        }
    });

    create_effect(move |_| {
        let windows = state.get().windows;
        sync_runtime_sessions(runtime.app_runtime, &windows);
    });

    create_effect(move |_| {
        let is_open = desktop_context_menu.get().is_some();
        let was_open = desktop_context_menu_was_open.get_untracked();
        if is_open && !was_open {
            desktop_context_menu_was_open.set(true);
            let _ = focus_first_menu_item("desktop-context-menu");
        } else if !is_open && was_open {
            desktop_context_menu_was_open.set(false);
        }
    });

    create_effect(move |_| {
        let is_open = display_properties_open.get();
        let was_open = display_properties_was_open.get_untracked();
        if is_open && !was_open {
            display_properties_last_focused.set(active_html_element());
            display_properties_was_open.set(true);
            if !focus_element_by_id("wallpaper-listbox") {
                let _ = focus_element_by_id("display-properties-close-button");
            }
            return;
        }

        if !is_open && was_open {
            display_properties_was_open.set(false);
            if let Some(element) = display_properties_last_focused.get_untracked() {
                focus_html_element(&element);
            } else {
                let _ = focus_element_by_id("desktop-shell-root");
            }
            display_properties_last_focused.set(None);
        }
    });

    let close_display_properties_cancel = Callback::new(move |_| {
        if let Some(original) = display_properties_original_wallpaper.get_untracked() {
            let current = wallpaper_preset_by_id(&runtime.state.get_untracked().theme.wallpaper_id);
            if current.id != original {
                runtime.dispatch_action(DesktopAction::SetWallpaper {
                    wallpaper_id: original.clone(),
                });
            }
            wallpaper_selection.set(original);
        }

        display_properties_original_wallpaper.set(None);
        display_properties_open.set(false);
    });

    let escape_listener = window_event_listener(ev::keydown, move |ev| {
        if ev.default_prevented() || ev.key() != "Escape" {
            return;
        }

        if desktop_context_menu.get_untracked().is_some() {
            ev.prevent_default();
            ev.stop_propagation();
            desktop_context_menu.set(None);
            let _ = focus_element_by_id("desktop-shell-root");
            return;
        }

        if display_properties_open.get_untracked() {
            ev.prevent_default();
            ev.stop_propagation();
            close_display_properties_cancel.call(());
        }
    });
    on_cleanup(move || escape_listener.remove());

    let on_pointer_move = move |ev: web_sys::PointerEvent| {
        let pointer = pointer_from_pointer_event(&ev);
        let interaction = runtime.interaction.get_untracked();

        if interaction.dragging.is_some() {
            runtime.dispatch_action(DesktopAction::UpdateMove { pointer });
        }
        if interaction.resizing.is_some() {
            runtime.dispatch_action(DesktopAction::UpdateResize { pointer });
        }
    };
    let on_pointer_end = move |_| end_active_pointer_interaction(runtime);
    // Runtime effect runner: clear current queue before processing so nested dispatches enqueue a
    // fresh batch instead of getting wiped.
    create_effect(move |_| {
        let queued = runtime.effects.get();
        if queued.is_empty() {
            return;
        }

        runtime.effects.set(Vec::new());

        for effect in queued {
            runtime.host.run_runtime_effect(runtime, effect);
        }
    });

    let open_display_properties = Callback::new(move |_| {
        let active = wallpaper_preset_by_id(&runtime.state.get_untracked().theme.wallpaper_id);
        desktop_context_menu.set(None);
        runtime.dispatch_action(DesktopAction::CloseStartMenu);
        wallpaper_selection.set(active.id.to_string());
        display_properties_original_wallpaper.set(Some(active.id.to_string()));
        display_properties_open.set(true);
    });

    let preview_selected_wallpaper = Callback::new(move |_| {
        let selected = wallpaper_preset_by_id(&wallpaper_selection.get_untracked());
        runtime.dispatch_action(DesktopAction::SetWallpaper {
            wallpaper_id: selected.id.to_string(),
        });
    });

    let apply_selected_wallpaper = Callback::new(move |_| {
        let selected = wallpaper_preset_by_id(&wallpaper_selection.get_untracked());
        runtime.dispatch_action(DesktopAction::SetWallpaper {
            wallpaper_id: selected.id.to_string(),
        });
        display_properties_original_wallpaper.set(Some(selected.id.to_string()));
    });
    let on_wallpaper_listbox_keydown = Callback::new(move |ev: web_sys::KeyboardEvent| {
        let presets = desktop_wallpaper_presets();
        if presets.is_empty() {
            return;
        }

        let selected_id = wallpaper_selection.get_untracked();
        let current_index = presets
            .iter()
            .position(|preset| preset.id == selected_id)
            .unwrap_or(0);
        let last_index = presets.len().saturating_sub(1);
        let next_index = match ev.key().as_str() {
            "ArrowUp" => Some(current_index.saturating_sub(1)),
            "ArrowDown" => Some((current_index + 1).min(last_index)),
            "Home" => Some(0),
            "End" => Some(last_index),
            "Enter" | " " | "Spacebar" => {
                ev.prevent_default();
                preview_selected_wallpaper.call(());
                None
            }
            _ => None,
        };

        if let Some(index) = next_index {
            ev.prevent_default();
            wallpaper_selection.set(presets[index].id.to_string());
        }
    });

    let close_display_properties_ok = Callback::new(move |_| {
        let selected = wallpaper_preset_by_id(&wallpaper_selection.get_untracked());
        runtime.dispatch_action(DesktopAction::SetWallpaper {
            wallpaper_id: selected.id.to_string(),
        });
        display_properties_original_wallpaper.set(None);
        display_properties_open.set(false);
    });

    view! {
        <div
            id="desktop-shell-root"
            class="desktop-shell"
            tabindex="-1"
            data-theme=move || state.get().theme.name
            data-high-contrast=move || state.get().theme.high_contrast.to_string()
            data-reduced-motion=move || state.get().theme.reduced_motion.to_string()
            on:click=move |_| {
                if desktop_context_menu.get_untracked().is_some() {
                    desktop_context_menu.set(None);
                }
            }
            on:pointermove=on_pointer_move
            on:pointerup=on_pointer_end
            on:pointercancel=on_pointer_end
        >
            <div
                class="desktop-wallpaper"
                data-wallpaper=move || wallpaper_preset_by_id(&state.get().theme.wallpaper_id).id
            >
                <div
                    class="desktop-surface-dismiss"
                    on:mousedown=move |_| {
                        desktop_context_menu.set(None);
                        runtime.dispatch_action(DesktopAction::CloseStartMenu);
                    }
                    on:contextmenu=move |ev| {
                        ev.prevent_default();
                        ev.stop_propagation();
                        runtime.dispatch_action(DesktopAction::CloseStartMenu);
                        display_properties_open.set(false);
                        open_desktop_context_menu(
                            runtime.host,
                            desktop_context_menu,
                            ev.client_x(),
                            ev.client_y(),
                        );
                    }
                />
                <div class="desktop-icons">
                    <For each=move || apps::desktop_icon_apps() key=|app| app.app_id as u8 let:app>
                        <button
                            class="desktop-icon"
                            data-app=app.app_id.icon_id()
                            on:click=move |_| {
                                runtime.dispatch_action(DesktopAction::ActivateApp {
                                    app_id: app.app_id,
                                });
                            }
                        >
                            <span class="icon" data-app=app.app_id.icon_id()>
                                <FluentIcon icon=app_icon_name(app.app_id) size=IconSize::Lg />
                            </span>
                            <span>{app.desktop_icon_label}</span>
                        </button>
                    </For>
                </div>

                <div class="window-layer">
                    <For
                        each=move || state.get().windows
                        key=|win| win.id.0
                        let:win
                    >
                        <DesktopWindow window_id=win.id />
                    </For>
                </div>

                <DesktopContextMenu
                    state
                    runtime
                    desktop_context_menu
                    wallpaper_selection
                    open_display_properties
                />

                <DisplayPropertiesDialog
                    state
                    display_properties_open
                    wallpaper_selection
                    on_wallpaper_listbox_keydown
                    preview_selected_wallpaper
                    apply_selected_wallpaper
                    close_display_properties_ok
                    close_display_properties_cancel
                />
            </div>

            <Taskbar />
        </div>
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TaskbarClockConfig {
    use_24_hour: bool,
    show_date: bool,
}

impl Default for TaskbarClockConfig {
    fn default() -> Self {
        Self {
            use_24_hour: false,
            show_date: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TaskbarClockSnapshot {
    year: u32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
}

impl TaskbarClockSnapshot {
    fn now() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            let date = js_sys::Date::new_0();
            return Self {
                year: date.get_full_year(),
                month: date.get_month() + 1,
                day: date.get_date(),
                hour: date.get_hours(),
                minute: date.get_minutes(),
                second: date.get_seconds(),
            };
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {
                year: 1970,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TaskbarLayoutPlan {
    compact_running_items: bool,
    visible_running_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TaskbarWindowContextMenuState {
    window_id: WindowId,
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct PinnedTaskbarAppState {
    running_count: usize,
    focused: bool,
    all_minimized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskbarShortcutTarget {
    Pinned(AppId),
    Window(WindowId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskbarTrayWidgetAction {
    None,
    ToggleHighContrast,
    ToggleReducedMotion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskbarTrayWidget {
    id: &'static str,
    icon: IconName,
    label: &'static str,
    value: String,
    pressed: Option<bool>,
    action: TaskbarTrayWidgetAction,
}

fn pinned_taskbar_apps() -> &'static [AppId] {
    const PINNED: &[AppId] = &[
        AppId::Explorer,
        AppId::Terminal,
        AppId::Notepad,
        AppId::Calculator,
    ];
    PINNED
}

fn ordered_taskbar_windows(state: &DesktopState) -> Vec<WindowRecord> {
    let mut windows = state.windows.clone();
    windows.sort_by_key(|win| (win.z_index, win.id.0));
    windows
}

fn preferred_window_for_app(state: &DesktopState, app_id: AppId) -> Option<WindowId> {
    state
        .windows
        .iter()
        .rev()
        .find(|win| win.app_id == app_id && !win.minimized && win.is_focused)
        .or_else(|| {
            state
                .windows
                .iter()
                .rev()
                .find(|win| win.app_id == app_id && !win.minimized)
        })
        .or_else(|| state.windows.iter().rev().find(|win| win.app_id == app_id))
        .map(|win| win.id)
}

fn pinned_taskbar_app_state(state: &DesktopState, app_id: AppId) -> PinnedTaskbarAppState {
    let windows: Vec<&WindowRecord> = state
        .windows
        .iter()
        .filter(|win| win.app_id == app_id)
        .collect();
    let running_count = windows.len();
    let focused = windows.iter().any(|win| win.is_focused && !win.minimized);
    let all_minimized = running_count > 0 && windows.iter().all(|win| win.minimized);

    PinnedTaskbarAppState {
        running_count,
        focused,
        all_minimized,
    }
}

fn compute_taskbar_layout(
    viewport_width: i32,
    pinned_count: usize,
    running_count: usize,
    tray_widget_count: usize,
    clock_show_date: bool,
) -> TaskbarLayoutPlan {
    if running_count == 0 {
        return TaskbarLayoutPlan {
            compact_running_items: false,
            visible_running_count: 0,
        };
    }

    let viewport_width = viewport_width.max(320);
    let pinned_width = (pinned_count as i32) * 42;
    let tray_width = 28 + ((tray_widget_count.min(4)) as i32 * 56);
    let clock_width = if clock_show_date { 136 } else { 88 };
    let reserved = 96 + pinned_width + tray_width + clock_width + 48;
    let available = (viewport_width - reserved).max(0);

    let full_item_width = 148;
    let compact_item_width = 44;
    let full_visible = (available / full_item_width).max(0) as usize;
    if full_visible >= running_count {
        return TaskbarLayoutPlan {
            compact_running_items: false,
            visible_running_count: running_count,
        };
    }

    let mut compact_visible = (available / compact_item_width).max(0) as usize;
    if compact_visible == 0 && available >= 32 {
        compact_visible = 1;
    }

    TaskbarLayoutPlan {
        compact_running_items: true,
        visible_running_count: compact_visible.min(running_count),
    }
}

fn taskbar_window_button_class(
    focused: bool,
    minimized: bool,
    compact: bool,
    keyboard_selected: bool,
) -> String {
    let mut class_name = String::from("taskbar-app");
    if focused {
        class_name.push_str(" focused");
    }
    if minimized {
        class_name.push_str(" minimized");
    } else {
        class_name.push_str(" active");
    }
    if compact {
        class_name.push_str(" compact");
    }
    if keyboard_selected {
        class_name.push_str(" keyboard-selected");
    }
    class_name
}

fn taskbar_pinned_button_class(status: PinnedTaskbarAppState) -> String {
    let mut class_name = String::from("taskbar-app taskbar-pinned");
    if status.running_count > 0 {
        class_name.push_str(" active");
    }
    if status.focused {
        class_name.push_str(" focused");
    }
    if status.all_minimized {
        class_name.push_str(" minimized");
    }
    class_name
}

fn taskbar_window_aria_label(win: &WindowRecord) -> String {
    let mut parts = vec![win.title.clone()];
    if win.is_focused && !win.minimized {
        parts.push("focused".to_string());
    }
    if win.minimized {
        parts.push("minimized".to_string());
    }
    if win.maximized {
        parts.push("maximized".to_string());
    }
    parts.join(", ")
}

fn taskbar_pinned_aria_label(app_id: AppId, status: PinnedTaskbarAppState) -> String {
    match status.running_count {
        0 => format!("Pinned {} (not running)", app_id.title()),
        1 => format!("Pinned {} (1 window running)", app_id.title()),
        count => format!("Pinned {} ({} windows running)", app_id.title(), count),
    }
}

fn build_taskbar_shortcut_targets(state: &DesktopState) -> Vec<TaskbarShortcutTarget> {
    let mut targets: Vec<TaskbarShortcutTarget> = pinned_taskbar_apps()
        .iter()
        .copied()
        .map(TaskbarShortcutTarget::Pinned)
        .collect();

    targets.extend(
        ordered_taskbar_windows(state)
            .into_iter()
            .map(|win| TaskbarShortcutTarget::Window(win.id)),
    );

    targets
}

fn activate_pinned_taskbar_app(runtime: DesktopRuntimeContext, app_id: AppId) {
    let state = runtime.state.get_untracked();
    let descriptor = apps::app_descriptor(app_id);

    if descriptor.single_instance {
        if let Some(window_id) = preferred_window_for_app(&state, app_id) {
            focus_or_unminimize_window(runtime, &state, window_id);
            return;
        }
    }

    runtime.dispatch_action(DesktopAction::ActivateApp { app_id });
}

fn activate_taskbar_shortcut_target(runtime: DesktopRuntimeContext, target: TaskbarShortcutTarget) {
    match target {
        TaskbarShortcutTarget::Pinned(app_id) => activate_pinned_taskbar_app(runtime, app_id),
        TaskbarShortcutTarget::Window(window_id) => {
            let state = runtime.state.get_untracked();
            focus_or_unminimize_window(runtime, &state, window_id);
        }
    }
}

fn focus_or_unminimize_window(
    runtime: DesktopRuntimeContext,
    state: &DesktopState,
    window_id: WindowId,
) {
    if let Some(window) = state.windows.iter().find(|win| win.id == window_id) {
        if window.minimized {
            runtime.dispatch_action(DesktopAction::RestoreWindow { window_id });
        } else if !window.is_focused {
            runtime.dispatch_action(DesktopAction::FocusWindow { window_id });
        }
    }
}

fn cycle_selected_running_window(
    running_windows: &[WindowRecord],
    selected: Option<WindowId>,
    delta: i32,
) -> Option<WindowId> {
    if running_windows.is_empty() {
        return None;
    }

    let current_idx = selected
        .and_then(|id| running_windows.iter().position(|win| win.id == id))
        .unwrap_or_else(|| {
            running_windows
                .iter()
                .position(|win| win.is_focused && !win.minimized)
                .unwrap_or(0)
        });
    let len = running_windows.len() as i32;
    let next_idx = (current_idx as i32 + delta).rem_euclid(len) as usize;
    Some(running_windows[next_idx].id)
}

fn open_desktop_context_menu(
    host: DesktopHostContext,
    menu: RwSignal<Option<DesktopContextMenuState>>,
    x: i32,
    y: i32,
) {
    let (x, y) = clamp_desktop_popup_position(host, x, y, 260, 340);
    menu.set(Some(DesktopContextMenuState { x, y }));
}

fn clamp_desktop_popup_position(
    host: DesktopHostContext,
    x: i32,
    y: i32,
    popup_w: i32,
    popup_h: i32,
) -> (i32, i32) {
    let viewport = host.desktop_viewport_rect(TASKBAR_HEIGHT_PX);
    let max_x = (viewport.w - popup_w - 6).max(6);
    let max_y = (viewport.h - popup_h - 6).max(6);
    (x.clamp(6, max_x), y.clamp(6, max_y))
}

fn open_taskbar_window_context_menu(
    host: DesktopHostContext,
    menu: RwSignal<Option<TaskbarWindowContextMenuState>>,
    window_id: WindowId,
    x: i32,
    y: i32,
) {
    let (x, y) = clamp_taskbar_popup_position(host, x, y, 220, 190);
    menu.set(Some(TaskbarWindowContextMenuState { window_id, x, y }));
}

fn clamp_taskbar_popup_position(
    host: DesktopHostContext,
    x: i32,
    y: i32,
    popup_w: i32,
    popup_h: i32,
) -> (i32, i32) {
    let viewport = host.desktop_viewport_rect(TASKBAR_HEIGHT_PX);
    let max_x = (viewport.w - popup_w - 6).max(6);
    let max_y = (viewport.h + TASKBAR_HEIGHT_PX - popup_h - 6).max(6);
    (x.clamp(6, max_x), y.clamp(6, max_y))
}

fn build_taskbar_tray_widgets(state: &DesktopState) -> Vec<TaskbarTrayWidget> {
    let total_windows = state.windows.len();
    let minimized_windows = state.windows.iter().filter(|win| win.minimized).count();
    let dialup_online = state
        .windows
        .iter()
        .any(|win| matches!(win.app_id, AppId::Dialup) && !win.minimized);

    vec![
        TaskbarTrayWidget {
            id: "win-count",
            icon: IconName::WindowMultiple,
            label: "Open windows",
            value: total_windows.to_string(),
            pressed: None,
            action: TaskbarTrayWidgetAction::None,
        },
        TaskbarTrayWidget {
            id: "bg-count",
            icon: IconName::DesktopArrowDown,
            label: "Minimized windows",
            value: minimized_windows.to_string(),
            pressed: None,
            action: TaskbarTrayWidgetAction::None,
        },
        TaskbarTrayWidget {
            id: "network",
            icon: if dialup_online {
                IconName::WifiOn
            } else {
                IconName::WifiOff
            },
            label: "Network status",
            value: if dialup_online { "ON" } else { "IDLE" }.to_string(),
            pressed: Some(dialup_online),
            action: TaskbarTrayWidgetAction::None,
        },
        TaskbarTrayWidget {
            id: "contrast",
            icon: if state.theme.high_contrast {
                IconName::Checkmark
            } else {
                IconName::Dismiss
            },
            label: "High contrast",
            value: if state.theme.high_contrast {
                "ON"
            } else {
                "OFF"
            }
            .to_string(),
            pressed: Some(state.theme.high_contrast),
            action: TaskbarTrayWidgetAction::ToggleHighContrast,
        },
        TaskbarTrayWidget {
            id: "motion",
            icon: if state.theme.reduced_motion {
                IconName::MotionOff
            } else {
                IconName::MotionOn
            },
            label: "Reduced motion",
            value: if state.theme.reduced_motion {
                "ON"
            } else {
                "OFF"
            }
            .to_string(),
            pressed: Some(state.theme.reduced_motion),
            action: TaskbarTrayWidgetAction::ToggleReducedMotion,
        },
    ]
}

fn activate_taskbar_tray_widget(runtime: DesktopRuntimeContext, action: TaskbarTrayWidgetAction) {
    match action {
        TaskbarTrayWidgetAction::None => {}
        TaskbarTrayWidgetAction::ToggleHighContrast => {
            let enabled = runtime.state.get_untracked().theme.high_contrast;
            runtime.dispatch_action(DesktopAction::SetHighContrast { enabled: !enabled });
        }
        TaskbarTrayWidgetAction::ToggleReducedMotion => {
            let enabled = runtime.state.get_untracked().theme.reduced_motion;
            runtime.dispatch_action(DesktopAction::SetReducedMotion { enabled: !enabled });
        }
    }
}

fn format_taskbar_clock_time(snapshot: TaskbarClockSnapshot, config: TaskbarClockConfig) -> String {
    if config.use_24_hour {
        format!(
            "{:02}:{:02}:{:02}",
            snapshot.hour, snapshot.minute, snapshot.second
        )
    } else {
        let mut hour = snapshot.hour % 12;
        if hour == 0 {
            hour = 12;
        }
        let suffix = if snapshot.hour >= 12 { "PM" } else { "AM" };
        format!(
            "{:02}:{:02}:{:02} {}",
            hour, snapshot.minute, snapshot.second, suffix
        )
    }
}

fn format_taskbar_clock_date(snapshot: TaskbarClockSnapshot) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        snapshot.year, snapshot.month, snapshot.day
    )
}

fn format_taskbar_clock_aria(snapshot: TaskbarClockSnapshot, config: TaskbarClockConfig) -> String {
    let time_text = format_taskbar_clock_time(snapshot, config);
    if config.show_date {
        format!("{}, {}", format_taskbar_clock_date(snapshot), time_text)
    } else {
        time_text
    }
}

fn stop_mouse_event(ev: &web_sys::MouseEvent) {
    ev.prevent_default();
    ev.stop_propagation();
}

fn pointer_from_pointer_event(ev: &web_sys::PointerEvent) -> PointerPosition {
    PointerPosition {
        x: ev.client_x(),
        y: ev.client_y(),
    }
}

fn end_active_pointer_interaction(runtime: DesktopRuntimeContext) {
    let interaction = runtime.interaction.get_untracked();
    if interaction.dragging.is_some() {
        runtime.dispatch_action(DesktopAction::EndMoveWithViewport {
            viewport: runtime.host.desktop_viewport_rect(TASKBAR_HEIGHT_PX),
        });
    }
    if interaction.resizing.is_some() {
        runtime.dispatch_action(DesktopAction::EndResize);
    }
}

fn resize_edge_class(edge: ResizeEdge) -> &'static str {
    match edge {
        ResizeEdge::North => "edge-n",
        ResizeEdge::South => "edge-s",
        ResizeEdge::East => "edge-e",
        ResizeEdge::West => "edge-w",
        ResizeEdge::NorthEast => "edge-ne",
        ResizeEdge::NorthWest => "edge-nw",
        ResizeEdge::SouthEast => "edge-se",
        ResizeEdge::SouthWest => "edge-sw",
    }
}
