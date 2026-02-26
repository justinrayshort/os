//! Leptos provider, context, and desktop shell UI composition.

use std::time::Duration;

use leptos::*;

use crate::{
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
    /// Reducer dispatch callback.
    pub dispatch: Callback<DesktopAction>,
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
    let display_properties_open = create_rw_signal(false);
    let wallpaper_selection = create_rw_signal(
        wallpaper_preset_by_id(&state.get_untracked().theme.wallpaper_id)
            .id
            .to_string(),
    );
    let display_properties_original_wallpaper = create_rw_signal(None::<String>);

    create_effect(move |_| {
        let active_wallpaper = wallpaper_preset_by_id(&state.get().theme.wallpaper_id);
        if !display_properties_open.get() {
            wallpaper_selection.set(active_wallpaper.id.to_string());
        }
    });

    let escape_listener = window_event_listener(ev::keydown, move |ev| {
        if ev.default_prevented() || ev.key() != "Escape" {
            return;
        }

        if desktop_context_menu.get_untracked().is_some() {
            ev.prevent_default();
            ev.stop_propagation();
            desktop_context_menu.set(None);
            return;
        }

        if display_properties_open.get_untracked() {
            ev.prevent_default();
            ev.stop_propagation();

            if let Some(original) = display_properties_original_wallpaper.get_untracked() {
                let current =
                    wallpaper_preset_by_id(&runtime.state.get_untracked().theme.wallpaper_id);
                if current.id != original {
                    runtime.dispatch_action(DesktopAction::SetWallpaper {
                        wallpaper_id: original.clone(),
                    });
                }
                wallpaper_selection.set(original);
            }

            display_properties_original_wallpaper.set(None);
            display_properties_open.set(false);
        }
    });
    on_cleanup(move || escape_listener.remove());

    let on_pointer_move = move |ev: web_sys::MouseEvent| {
        let pointer = pointer_from_mouse_event(&ev);
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

    let close_display_properties_ok = Callback::new(move |_| {
        let selected = wallpaper_preset_by_id(&wallpaper_selection.get_untracked());
        runtime.dispatch_action(DesktopAction::SetWallpaper {
            wallpaper_id: selected.id.to_string(),
        });
        display_properties_original_wallpaper.set(None);
        display_properties_open.set(false);
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

    view! {
        <div
            class="desktop-shell"
            data-theme=move || state.get().theme.name
            data-high-contrast=move || state.get().theme.high_contrast.to_string()
            data-reduced-motion=move || state.get().theme.reduced_motion.to_string()
            on:click=move |_| {
                if desktop_context_menu.get_untracked().is_some() {
                    desktop_context_menu.set(None);
                }
            }
            on:mousemove=on_pointer_move
            on:mouseup=on_pointer_end
            on:mouseleave=on_pointer_end
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
                                runtime.dispatch_action(DesktopAction::OpenWindow(
                                    apps::default_open_request(app.app_id),
                                ));
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

                <Show when=move || desktop_context_menu.get().is_some() fallback=|| ()>
                    {move || {
                        let Some(menu) = desktop_context_menu.get() else {
                            return ().into_view();
                        };
                        let active = wallpaper_preset_by_id(&state.get().theme.wallpaper_id);
                        let menu_style = format!("left:{}px;top:{}px;", menu.x, menu.y);

                        view! {
                            <div
                                class="taskbar-menu desktop-context-menu"
                                role="menu"
                                aria-label="Desktop context menu"
                                style=menu_style
                                on:click=move |ev| ev.stop_propagation()
                            >
                                <button
                                    role="menuitem"
                                    class="taskbar-menu-item"
                                    on:click:undelegated=move |ev| {
                                        stop_mouse_event(&ev);
                                        desktop_context_menu.set(None);
                                    }
                                >
                                    "Refresh"
                                </button>
                                <button
                                    role="menuitem"
                                    class="taskbar-menu-item"
                                    on:click:undelegated=move |ev| {
                                        stop_mouse_event(&ev);
                                        open_display_properties.call(());
                                    }
                                >
                                    "Properties..."
                                </button>

                                <div class="desktop-menu-separator" role="separator" aria-hidden="true"></div>
                                <div class="desktop-context-group-label">
                                    "Quick Backgrounds"
                                </div>

                                <For
                                    each=move || desktop_wallpaper_presets().to_vec()
                                    key=|preset| preset.id
                                    let:preset
                                >
                                    <button
                                        role="menuitemradio"
                                        aria-checked=move || active.id == preset.id
                                        class=move || {
                                            if active.id == preset.id {
                                                "taskbar-menu-item desktop-context-wallpaper-item active"
                                            } else {
                                                "taskbar-menu-item desktop-context-wallpaper-item"
                                            }
                                        }
                                        on:click:undelegated=move |ev| {
                                            stop_mouse_event(&ev);
                                            desktop_context_menu.set(None);
                                            wallpaper_selection.set(preset.id.to_string());
                                            runtime.dispatch_action(DesktopAction::SetWallpaper {
                                                wallpaper_id: preset.id.to_string(),
                                            });
                                        }
                                    >
                                        <span class="desktop-context-wallpaper-check" aria-hidden="true">
                                            {if active.id == preset.id {
                                                view! { <FluentIcon icon=IconName::Checkmark size=IconSize::Xs /> }.into_view()
                                            } else {
                                                ().into_view()
                                            }}
                                        </span>
                                        <span class="desktop-context-wallpaper-text">
                                            <span class="desktop-context-wallpaper-label">{preset.label}</span>
                                            <span class="desktop-context-wallpaper-meta">
                                                {wallpaper_preset_kind_label(preset.kind)}
                                            </span>
                                        </span>
                                    </button>
                                </For>
                            </div>
                        }
                            .into_view()
                    }}
                </Show>

                <Show when=move || display_properties_open.get() fallback=|| ()>
                    <div
                        class="display-properties-overlay"
                        on:mousedown=move |ev| ev.stop_propagation()
                        on:click=move |ev| ev.stop_propagation()
                    >
                        <section
                            class="display-properties-dialog"
                            role="dialog"
                            aria-modal="true"
                            aria-labelledby="display-properties-title"
                            on:mousedown=move |ev| ev.stop_propagation()
                            on:click=move |ev| ev.stop_propagation()
                        >
                            <header class="display-properties-titlebar">
                                <div id="display-properties-title" class="display-properties-title">
                                    "Display Properties"
                                </div>
                                <button
                                    class="display-properties-close"
                                    aria-label="Close display properties"
                                    on:click:undelegated=move |_| close_display_properties_cancel.call(())
                                >
                                    <FluentIcon icon=IconName::Dismiss size=IconSize::Sm />
                                </button>
                            </header>

                            <div class="display-properties-body">
                                <div class="display-properties-tabs" role="tablist" aria-label="Display settings">
                                    <button class="display-properties-tab active" role="tab" aria-selected="true">
                                        "Background"
                                    </button>
                                    <button
                                        class="display-properties-tab"
                                        role="tab"
                                        aria-selected="false"
                                        disabled=true
                                    >
                                        "Appearance"
                                    </button>
                                    <button
                                        class="display-properties-tab"
                                        role="tab"
                                        aria-selected="false"
                                        disabled=true
                                    >
                                        "Effects"
                                    </button>
                                </div>

                                <div class="display-properties-content">
                                    <div class="display-preview-column">
                                        <div class="display-preview-frame" aria-hidden="true">
                                            <div class="display-preview-monitor">
                                                <div
                                                    class="display-preview-screen"
                                                    data-wallpaper=move || {
                                                        wallpaper_preset_by_id(&wallpaper_selection.get()).id
                                                    }
                                                >
                                                    <div class="display-preview-desktop-icon">
                                                        "My Computer"
                                                    </div>
                                                </div>
                                                <div class="display-preview-taskbar">
                                                    <span class="display-preview-start">"Start"</span>
                                                    <span class="display-preview-clock">"9:41 AM"</span>
                                                </div>
                                            </div>
                                        </div>

                                        <div class="display-preview-caption">
                                            {move || {
                                                let preset = wallpaper_preset_by_id(&wallpaper_selection.get());
                                                format!("{} ({})", preset.label, wallpaper_preset_kind_label(preset.kind))
                                            }}
                                        </div>
                                        <div class="display-preview-note">
                                            {move || wallpaper_preset_by_id(&wallpaper_selection.get()).note}
                                        </div>
                                    </div>

                                    <div class="display-options-column">
                                        <label class="display-list-label" for="wallpaper-listbox">
                                            "Wallpaper"
                                        </label>
                                        <div
                                            id="wallpaper-listbox"
                                            class="wallpaper-picker-list"
                                            role="listbox"
                                            aria-label="Wallpaper choices"
                                        >
                                            <For
                                                each=move || desktop_wallpaper_presets().to_vec()
                                                key=|preset| preset.id
                                                let:preset
                                            >
                                                <button
                                                    class=move || {
                                                        if wallpaper_preset_by_id(&wallpaper_selection.get()).id == preset.id {
                                                            "wallpaper-picker-item selected"
                                                        } else {
                                                            "wallpaper-picker-item"
                                                        }
                                                    }
                                                    role="option"
                                                    aria-selected=move || {
                                                        wallpaper_preset_by_id(&wallpaper_selection.get()).id == preset.id
                                                    }
                                                    on:click:undelegated=move |_| {
                                                        wallpaper_selection.set(preset.id.to_string());
                                                    }
                                                    on:dblclick:undelegated=move |_| {
                                                        wallpaper_selection.set(preset.id.to_string());
                                                        preview_selected_wallpaper.call(());
                                                    }
                                                >
                                                    <span
                                                        class="wallpaper-preview-thumb"
                                                        data-wallpaper=preset.id
                                                        aria-hidden="true"
                                                    />
                                                    <span class="wallpaper-picker-item-copy">
                                                        <span class="wallpaper-picker-item-label">
                                                            {preset.label}
                                                        </span>
                                                        <span class="wallpaper-picker-item-meta">
                                                            {wallpaper_preset_kind_label(preset.kind)}
                                                        </span>
                                                    </span>
                                                </button>
                                            </For>
                                        </div>

                                        <div class="display-properties-actions-row">
                                            <button
                                                class="display-action-button"
                                                on:click:undelegated=move |_| preview_selected_wallpaper.call(())
                                            >
                                                "Preview"
                                            </button>
                                            <button
                                                class="display-action-button"
                                                on:click:undelegated=move |_| apply_selected_wallpaper.call(())
                                            >
                                                "Apply"
                                            </button>
                                        </div>

                                        <div class="display-properties-current">
                                            {move || {
                                                let current = wallpaper_preset_by_id(&state.get().theme.wallpaper_id);
                                                format!("Current desktop: {}", current.label)
                                            }}
                                        </div>
                                    </div>
                                </div>
                            </div>

                            <footer class="display-properties-footer">
                                <button
                                    class="display-footer-button"
                                    on:click:undelegated=move |_| close_display_properties_ok.call(())
                                >
                                    "OK"
                                </button>
                                <button
                                    class="display-footer-button"
                                    on:click:undelegated=move |_| close_display_properties_cancel.call(())
                                >
                                    "Cancel"
                                </button>
                            </footer>
                        </section>
                    </div>
                </Show>
            </div>

            <Taskbar />
        </div>
    }
}

#[component]
fn DesktopWindow(window_id: WindowId) -> impl IntoView {
    let runtime = use_desktop_runtime();

    let window = Signal::derive(move || {
        runtime
            .state
            .get()
            .windows
            .into_iter()
            .find(|w| w.id == window_id)
    });

    let focus = move |_| {
        let should_focus = window
            .get()
            .map(|w| !w.is_focused || w.minimized)
            .unwrap_or(false);
        if should_focus {
            runtime.dispatch_action(DesktopAction::FocusWindow { window_id });
        }
    };
    let minimize = move |_| runtime.dispatch_action(DesktopAction::MinimizeWindow { window_id });
    let close = move |_| runtime.dispatch_action(DesktopAction::CloseWindow { window_id });
    let toggle_maximize = move |_| {
        if let Some(win) = window.get() {
            if win.maximized {
                runtime.dispatch_action(DesktopAction::RestoreWindow { window_id });
            } else if win.flags.maximizable {
                runtime.dispatch_action(DesktopAction::MaximizeWindow {
                    window_id,
                    viewport: runtime.host.desktop_viewport_rect(TASKBAR_HEIGHT_PX),
                });
            }
        }
    };
    let begin_move = move |ev: web_sys::MouseEvent| {
        if ev.button() != 0 {
            return;
        }
        ev.prevent_default();
        ev.stop_propagation();
        runtime.dispatch_action(DesktopAction::BeginMove {
            window_id,
            pointer: pointer_from_mouse_event(&ev),
        });
    };
    let titlebar_double_click = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        if let Some(win) = window.get() {
            if win.flags.maximizable {
                if win.maximized {
                    runtime.dispatch_action(DesktopAction::RestoreWindow { window_id });
                } else {
                    runtime.dispatch_action(DesktopAction::MaximizeWindow {
                        window_id,
                        viewport: runtime.host.desktop_viewport_rect(TASKBAR_HEIGHT_PX),
                    });
                }
            }
        }
    };

    view! {
        <Show when=move || window.get().is_some() fallback=|| ()>
            {move || {
                let win = window.get().expect("window exists while shown");
                let style = format!(
                    "left:{}px;top:{}px;width:{}px;height:{}px;z-index:{};",
                    win.rect.x, win.rect.y, win.rect.w, win.rect.h, win.z_index
                );
                let focused_class = if win.is_focused { " focused" } else { "" };
                let minimized_class = if win.minimized { " minimized" } else { "" };
                let maximized_class = if win.maximized { " maximized" } else { "" };

                view! {
                    <section
                        class=format!(
                            "desktop-window{}{}{}",
                            focused_class,
                            minimized_class,
                            maximized_class
                        )
                        style=style
                        on:mousedown=focus
                        role="dialog"
                        aria-label=win.title.clone()
                    >
                        <header
                            class="titlebar"
                            on:mousedown=begin_move
                            on:dblclick=titlebar_double_click
                        >
                            <div class="titlebar-title">
                                <span class="titlebar-app-icon" aria-hidden="true">
                                    <FluentIcon icon=app_icon_name(win.app_id) size=IconSize::Sm />
                                </span>
                                <span>{win.title.clone()}</span>
                            </div>
                            <div class="titlebar-controls">
                                <button
                                    disabled=!win.flags.minimizable
                                    aria-label="Minimize window"
                                    on:mousedown=move |ev| stop_mouse_event(&ev)
                                    on:click=move |ev| {
                                        stop_mouse_event(&ev);
                                        minimize(ev);
                                    }
                                >
                                    <FluentIcon icon=IconName::WindowMinimize size=IconSize::Xs />
                                </button>
                                <button
                                    disabled=!win.flags.maximizable
                                    aria-label=if win.maximized {
                                        "Restore window"
                                    } else {
                                        "Maximize window"
                                    }
                                    on:mousedown=move |ev| stop_mouse_event(&ev)
                                    on:click=move |ev| {
                                        stop_mouse_event(&ev);
                                        toggle_maximize(ev);
                                    }
                                >
                                    <FluentIcon
                                        icon=if win.maximized {
                                            IconName::WindowRestore
                                        } else {
                                            IconName::WindowMaximize
                                        }
                                        size=IconSize::Xs
                                    />
                                </button>
                                <button
                                    aria-label="Close window"
                                    on:mousedown=move |ev| stop_mouse_event(&ev)
                                    on:click=move |ev| {
                                        stop_mouse_event(&ev);
                                        close(ev);
                                    }
                                >
                                    <FluentIcon icon=IconName::Dismiss size=IconSize::Xs />
                                </button>
                            </div>
                        </header>
                        <div class="window-body">
                            <WindowBody window_id=window_id />
                        </div>
                        <Show
                            when=move || {
                                window
                                    .get()
                                    .map(|w| w.flags.resizable && !w.maximized)
                                    .unwrap_or(false)
                            }
                            fallback=|| ()
                        >
                            <WindowResizeHandle window_id=window_id edge=ResizeEdge::North />
                            <WindowResizeHandle window_id=window_id edge=ResizeEdge::South />
                            <WindowResizeHandle window_id=window_id edge=ResizeEdge::East />
                            <WindowResizeHandle window_id=window_id edge=ResizeEdge::West />
                            <WindowResizeHandle window_id=window_id edge=ResizeEdge::NorthEast />
                            <WindowResizeHandle window_id=window_id edge=ResizeEdge::NorthWest />
                            <WindowResizeHandle window_id=window_id edge=ResizeEdge::SouthEast />
                            <WindowResizeHandle window_id=window_id edge=ResizeEdge::SouthWest />
                        </Show>
                    </section>
                }
                    .into_view()
            }}
        </Show>
    }
}

#[component]
fn WindowResizeHandle(window_id: WindowId, edge: ResizeEdge) -> impl IntoView {
    let runtime = use_desktop_runtime();
    let class_name = format!("window-resize-handle {}", resize_edge_class(edge));

    let on_mousedown = move |ev: web_sys::MouseEvent| {
        if ev.button() != 0 {
            return;
        }
        ev.prevent_default();
        ev.stop_propagation();
        runtime.dispatch_action(DesktopAction::BeginResize {
            window_id,
            edge,
            pointer: pointer_from_mouse_event(&ev),
        });
    };

    view! {
        <div
            class=class_name
            aria-hidden="true"
            on:mousedown=on_mousedown
        />
    }
}

#[component]
fn WindowBody(window_id: WindowId) -> impl IntoView {
    let runtime = use_desktop_runtime();
    let state = runtime.state;
    let contents = state
        .get_untracked()
        .windows
        .into_iter()
        .find(|w| w.id == window_id)
        .map(|w| apps::render_window_contents(&w))
        .unwrap_or_else(|| view! { <p>"Closed"</p> }.into_view());

    view! {
        <div class="window-body-content">
            {contents}
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

    runtime.dispatch_action(DesktopAction::OpenWindow(apps::default_open_request(
        app_id,
    )));
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

fn shortcut_digit_index(ev: &web_sys::KeyboardEvent) -> Option<usize> {
    match ev.key().as_str() {
        "1" => Some(0),
        "2" => Some(1),
        "3" => Some(2),
        "4" => Some(3),
        "5" => Some(4),
        "6" => Some(5),
        "7" => Some(6),
        "8" => Some(7),
        "9" => Some(8),
        _ => None,
    }
}

fn is_context_menu_shortcut(ev: &web_sys::KeyboardEvent) -> bool {
    ev.key() == "ContextMenu" || (ev.shift_key() && ev.key() == "F10")
}

fn is_activation_key(ev: &web_sys::KeyboardEvent) -> bool {
    matches!(ev.key().as_str(), "Enter" | " " | "Spacebar")
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

#[component]
fn Taskbar() -> impl IntoView {
    let runtime = use_desktop_runtime();
    let state = runtime.state;

    let viewport_width = create_rw_signal(runtime.host.desktop_viewport_rect(TASKBAR_HEIGHT_PX).w);
    let clock_config = create_rw_signal(TaskbarClockConfig::default());
    let clock_now = create_rw_signal(TaskbarClockSnapshot::now());
    let selected_running_window = create_rw_signal(None::<WindowId>);
    let window_context_menu = create_rw_signal(None::<TaskbarWindowContextMenuState>);
    let overflow_menu_open = create_rw_signal(false);
    let clock_menu_open = create_rw_signal(false);

    let resize_listener = window_event_listener(ev::resize, move |_| {
        viewport_width.set(runtime.host.desktop_viewport_rect(TASKBAR_HEIGHT_PX).w);
    });
    on_cleanup(move || resize_listener.remove());

    if let Ok(interval) = set_interval_with_handle(
        move || clock_now.set(TaskbarClockSnapshot::now()),
        Duration::from_secs(1),
    ) {
        on_cleanup(move || interval.clear());
    }

    let outside_click_listener = window_event_listener(ev::mousedown, move |_| {
        let had_window_menu = window_context_menu.get_untracked().is_some();
        let had_overflow_menu = overflow_menu_open.get_untracked();
        let had_clock_menu = clock_menu_open.get_untracked();
        let had_start_menu = runtime.state.get_untracked().start_menu_open;

        window_context_menu.set(None);
        overflow_menu_open.set(false);
        clock_menu_open.set(false);

        // Avoid dispatching on every click. A redundant desktop-state update remounts app views,
        // which resets app-local UI state (e.g., calculator input/history signal state).
        if had_start_menu || had_window_menu || had_overflow_menu || had_clock_menu {
            runtime.dispatch_action(DesktopAction::CloseStartMenu);
        }
    });
    on_cleanup(move || outside_click_listener.remove());

    let global_shortcut_listener = window_event_listener(ev::keydown, move |ev| {
        if ev.default_prevented() {
            return;
        }

        if ev.ctrl_key() && !ev.alt_key() && !ev.meta_key() && ev.key() == "Escape" {
            ev.prevent_default();
            ev.stop_propagation();
            window_context_menu.set(None);
            overflow_menu_open.set(false);
            clock_menu_open.set(false);
            runtime.dispatch_action(DesktopAction::ToggleStartMenu);
            return;
        }

        if ev.alt_key() && !ev.ctrl_key() && !ev.meta_key() {
            if let Some(index) = shortcut_digit_index(&ev) {
                let desktop = runtime.state.get_untracked();
                if let Some(target) = build_taskbar_shortcut_targets(&desktop)
                    .into_iter()
                    .nth(index)
                {
                    ev.prevent_default();
                    ev.stop_propagation();
                    window_context_menu.set(None);
                    overflow_menu_open.set(false);
                    clock_menu_open.set(false);
                    runtime.dispatch_action(DesktopAction::CloseStartMenu);
                    activate_taskbar_shortcut_target(runtime, target);
                }
                return;
            }
        }

        if ev.key() == "Escape"
            && (runtime.state.get_untracked().start_menu_open
                || window_context_menu.get_untracked().is_some()
                || overflow_menu_open.get_untracked()
                || clock_menu_open.get_untracked())
        {
            ev.prevent_default();
            ev.stop_propagation();
            window_context_menu.set(None);
            overflow_menu_open.set(false);
            clock_menu_open.set(false);
            runtime.dispatch_action(DesktopAction::CloseStartMenu);
        }
    });
    on_cleanup(move || global_shortcut_listener.remove());

    create_effect(move |_| {
        let desktop = state.get();
        let running = ordered_taskbar_windows(&desktop);
        let focused = running
            .iter()
            .find(|win| win.is_focused && !win.minimized)
            .map(|win| win.id);

        selected_running_window.update(|selected| {
            let selected_exists = selected
                .and_then(|id| running.iter().find(|win| win.id == id))
                .is_some();
            if selected_exists {
                return;
            }
            *selected = focused.or_else(|| running.first().map(|win| win.id));
        });

        window_context_menu.update(|menu| {
            if let Some(current) = *menu {
                if running.iter().all(|win| win.id != current.window_id) {
                    *menu = None;
                }
            }
        });
    });

    let on_taskbar_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.ctrl_key() && !ev.alt_key() && !ev.meta_key() && ev.key() == "Escape" {
            ev.prevent_default();
            ev.stop_propagation();
            window_context_menu.set(None);
            overflow_menu_open.set(false);
            clock_menu_open.set(false);
            runtime.dispatch_action(DesktopAction::ToggleStartMenu);
            return;
        }

        if ev.alt_key() && !ev.ctrl_key() && !ev.meta_key() {
            if let Some(index) = shortcut_digit_index(&ev) {
                let desktop = runtime.state.get_untracked();
                if let Some(target) = build_taskbar_shortcut_targets(&desktop)
                    .into_iter()
                    .nth(index)
                {
                    ev.prevent_default();
                    ev.stop_propagation();
                    window_context_menu.set(None);
                    overflow_menu_open.set(false);
                    clock_menu_open.set(false);
                    runtime.dispatch_action(DesktopAction::CloseStartMenu);
                    activate_taskbar_shortcut_target(runtime, target);
                }
                return;
            }
        }

        match ev.key().as_str() {
            "Escape" => {
                if runtime.state.get_untracked().start_menu_open
                    || window_context_menu.get_untracked().is_some()
                    || overflow_menu_open.get_untracked()
                    || clock_menu_open.get_untracked()
                {
                    ev.prevent_default();
                    ev.stop_propagation();
                    window_context_menu.set(None);
                    overflow_menu_open.set(false);
                    clock_menu_open.set(false);
                    runtime.dispatch_action(DesktopAction::CloseStartMenu);
                }
            }
            "ArrowRight" => {
                let desktop = runtime.state.get_untracked();
                let running = ordered_taskbar_windows(&desktop);
                if let Some(next) = cycle_selected_running_window(
                    &running,
                    selected_running_window.get_untracked(),
                    1,
                ) {
                    ev.prevent_default();
                    selected_running_window.set(Some(next));
                }
            }
            "ArrowLeft" => {
                let desktop = runtime.state.get_untracked();
                let running = ordered_taskbar_windows(&desktop);
                if let Some(next) = cycle_selected_running_window(
                    &running,
                    selected_running_window.get_untracked(),
                    -1,
                ) {
                    ev.prevent_default();
                    selected_running_window.set(Some(next));
                }
            }
            "Home" => {
                let desktop = runtime.state.get_untracked();
                let running = ordered_taskbar_windows(&desktop);
                if let Some(first) = running.first() {
                    ev.prevent_default();
                    selected_running_window.set(Some(first.id));
                }
            }
            "End" => {
                let desktop = runtime.state.get_untracked();
                let running = ordered_taskbar_windows(&desktop);
                if let Some(last) = running.last() {
                    ev.prevent_default();
                    selected_running_window.set(Some(last.id));
                }
            }
            _ => {
                if is_activation_key(&ev) {
                    if let Some(window_id) = selected_running_window.get_untracked() {
                        ev.prevent_default();
                        ev.stop_propagation();
                        window_context_menu.set(None);
                        overflow_menu_open.set(false);
                        clock_menu_open.set(false);
                        runtime.dispatch_action(DesktopAction::ToggleTaskbarWindow { window_id });
                    }
                } else if is_context_menu_shortcut(&ev) {
                    if let Some(window_id) = selected_running_window.get_untracked() {
                        ev.prevent_default();
                        ev.stop_propagation();
                        window_context_menu.set(None);
                        overflow_menu_open.set(false);
                        clock_menu_open.set(false);
                        runtime.dispatch_action(DesktopAction::CloseStartMenu);
                        let viewport = runtime.host.desktop_viewport_rect(TASKBAR_HEIGHT_PX);
                        let x = (viewport.w / 2).max(24);
                        let y = (viewport.h + TASKBAR_HEIGHT_PX - 180).max(24);
                        open_taskbar_window_context_menu(
                            runtime.host,
                            window_context_menu,
                            window_id,
                            x,
                            y,
                        );
                    }
                }
            }
        }
    };

    view! {
        <footer
            class="taskbar"
            role="toolbar"
            aria-label="Desktop taskbar"
            aria-keyshortcuts="Ctrl+Escape Alt+1 Alt+2 Alt+3 Alt+4 Alt+5 Alt+6 Alt+7 Alt+8 Alt+9"
            on:mousedown=move |ev| ev.stop_propagation()
            on:keydown=on_taskbar_keydown
        >
            <div class="taskbar-left">
                <button
                    class="start-button"
                    aria-label="Open application launcher"
                    aria-haspopup="menu"
                    aria-controls="desktop-launcher-menu"
                    aria-expanded=move || state.get().start_menu_open
                    aria-keyshortcuts="Ctrl+Escape"
                    on:click=move |_| {
                        window_context_menu.set(None);
                        overflow_menu_open.set(false);
                        clock_menu_open.set(false);
                        runtime.dispatch_action(DesktopAction::ToggleStartMenu);
                    }
                >
                    <span class="taskbar-glyph" aria-hidden="true">
                        <FluentIcon icon=IconName::Launcher size=IconSize::Sm />
                    </span>
                    <span>"Start"</span>
                </button>

                <div class="taskbar-pins" role="group" aria-label="Pinned apps">
                    <For
                        each=move || pinned_taskbar_apps().to_vec()
                        key=|app_id| *app_id as u8
                        let:app_id
                    >
                        <button
                            class=move || {
                                let desktop = state.get();
                                taskbar_pinned_button_class(pinned_taskbar_app_state(&desktop, app_id))
                            }
                            data-app=app_id.icon_id()
                            title=move || {
                                let desktop = state.get();
                                let status = pinned_taskbar_app_state(&desktop, app_id);
                                taskbar_pinned_aria_label(app_id, status)
                            }
                            aria-label=move || {
                                let desktop = state.get();
                                let status = pinned_taskbar_app_state(&desktop, app_id);
                                taskbar_pinned_aria_label(app_id, status)
                            }
                            on:click=move |_| {
                                window_context_menu.set(None);
                                overflow_menu_open.set(false);
                                clock_menu_open.set(false);
                                runtime.dispatch_action(DesktopAction::CloseStartMenu);
                                activate_pinned_taskbar_app(runtime, app_id);
                            }
                        >
                            <span class="taskbar-app-icon" aria-hidden="true">
                                <FluentIcon icon=app_icon_name(app_id) size=IconSize::Sm />
                            </span>
                            <span class="visually-hidden">{app_id.title()}</span>
                        </button>
                    </For>
                </div>
            </div>

            <div class="taskbar-running-region" role="group" aria-label="Running windows">
                <div class="taskbar-running-strip">
                    <For
                        each=move || {
                            let desktop = state.get();
                            let tray_count = build_taskbar_tray_widgets(&desktop).len();
                            let layout = compute_taskbar_layout(
                                viewport_width.get(),
                                pinned_taskbar_apps().len(),
                                desktop.windows.len(),
                                tray_count,
                                clock_config.get().show_date,
                            );
                            ordered_taskbar_windows(&desktop)
                                .into_iter()
                                .take(layout.visible_running_count)
                                .collect::<Vec<_>>()
                        }
                        key=|win| win.id.0
                        let:win
                    >
                        <button
                            class=move || {
                                let desktop = state.get();
                                let tray_count = build_taskbar_tray_widgets(&desktop).len();
                                let layout = compute_taskbar_layout(
                                    viewport_width.get(),
                                    pinned_taskbar_apps().len(),
                                    desktop.windows.len(),
                                    tray_count,
                                    clock_config.get().show_date,
                                );
                                taskbar_window_button_class(
                                    win.is_focused && !win.minimized,
                                    win.minimized,
                                    layout.compact_running_items,
                                    selected_running_window.get() == Some(win.id),
                                )
                            }
                            data-app=win.app_id.icon_id()
                            aria-pressed=move || win.is_focused && !win.minimized
                            aria-label=taskbar_window_aria_label(&win)
                            title=taskbar_window_aria_label(&win)
                            on:click=move |_| {
                                selected_running_window.set(Some(win.id));
                                window_context_menu.set(None);
                                overflow_menu_open.set(false);
                                clock_menu_open.set(false);
                                runtime.dispatch_action(DesktopAction::CloseStartMenu);
                                runtime.dispatch_action(DesktopAction::ToggleTaskbarWindow {
                                    window_id: win.id,
                                });
                            }
                            on:contextmenu=move |ev| {
                                ev.prevent_default();
                                ev.stop_propagation();
                                selected_running_window.set(Some(win.id));
                                overflow_menu_open.set(false);
                                clock_menu_open.set(false);
                                runtime.dispatch_action(DesktopAction::CloseStartMenu);
                                open_taskbar_window_context_menu(
                                    runtime.host,
                                    window_context_menu,
                                    win.id,
                                    ev.client_x(),
                                    ev.client_y(),
                                );
                            }
                        >
                            <span class="taskbar-app-icon" aria-hidden="true">
                                <FluentIcon icon=app_icon_name(win.app_id) size=IconSize::Sm />
                            </span>
                            <span class="taskbar-app-label">{win.title.clone()}</span>
                        </button>
                    </For>

                    <Show
                        when=move || {
                            let desktop = state.get();
                            let tray_count = build_taskbar_tray_widgets(&desktop).len();
                            let layout = compute_taskbar_layout(
                                viewport_width.get(),
                                pinned_taskbar_apps().len(),
                                desktop.windows.len(),
                                tray_count,
                                clock_config.get().show_date,
                            );
                            desktop.windows.len() > layout.visible_running_count
                        }
                        fallback=|| ()
                    >
                        <div class="taskbar-overflow-wrap">
                            <button
                                class="taskbar-overflow-button"
                                aria-haspopup="menu"
                                aria-controls="taskbar-overflow-menu"
                                aria-expanded=move || overflow_menu_open.get()
                                on:click=move |_| {
                                    window_context_menu.set(None);
                                    clock_menu_open.set(false);
                                    runtime.dispatch_action(DesktopAction::CloseStartMenu);
                                    overflow_menu_open.update(|open| *open = !*open);
                                }
                            >
                                <span class="taskbar-overflow-icon" aria-hidden="true">
                                    <FluentIcon icon=IconName::ChevronDown size=IconSize::Xs />
                                </span>
                                {move || {
                                    let desktop = state.get();
                                    let tray_count = build_taskbar_tray_widgets(&desktop).len();
                                    let layout = compute_taskbar_layout(
                                        viewport_width.get(),
                                        pinned_taskbar_apps().len(),
                                        desktop.windows.len(),
                                        tray_count,
                                        clock_config.get().show_date,
                                    );
                                    let hidden = desktop.windows.len().saturating_sub(layout.visible_running_count);
                                    format!("+{hidden}")
                                }}
                            </button>

                            <Show when=move || overflow_menu_open.get() fallback=|| ()>
                                <div
                                    id="taskbar-overflow-menu"
                                    class="taskbar-menu taskbar-overflow-menu"
                                    role="menu"
                                    aria-label="Hidden taskbar windows"
                                    on:mousedown=move |ev| ev.stop_propagation()
                                >
                                    <For
                                        each=move || {
                                            let desktop = state.get();
                                            let tray_count = build_taskbar_tray_widgets(&desktop).len();
                                            let layout = compute_taskbar_layout(
                                                viewport_width.get(),
                                                pinned_taskbar_apps().len(),
                                                desktop.windows.len(),
                                                tray_count,
                                                clock_config.get().show_date,
                                            );
                                            ordered_taskbar_windows(&desktop)
                                                .into_iter()
                                                .skip(layout.visible_running_count)
                                                .collect::<Vec<_>>()
                                        }
                                        key=|win| win.id.0
                                        let:win
                                    >
                                        <button
                                            role="menuitem"
                                            class=move || {
                                                if win.minimized {
                                                    "taskbar-menu-item minimized"
                                                } else {
                                                    "taskbar-menu-item"
                                                }
                                            }
                                            on:click=move |_| {
                                                selected_running_window.set(Some(win.id));
                                                overflow_menu_open.set(false);
                                                window_context_menu.set(None);
                                                clock_menu_open.set(false);
                                                runtime.dispatch_action(DesktopAction::CloseStartMenu);
                                                let desktop = runtime.state.get_untracked();
                                                focus_or_unminimize_window(runtime, &desktop, win.id);
                                            }
                                            on:contextmenu=move |ev| {
                                                ev.prevent_default();
                                                ev.stop_propagation();
                                                selected_running_window.set(Some(win.id));
                                                overflow_menu_open.set(false);
                                                clock_menu_open.set(false);
                                                runtime.dispatch_action(DesktopAction::CloseStartMenu);
                                                open_taskbar_window_context_menu(
                                                    runtime.host,
                                                    window_context_menu,
                                                    win.id,
                                                    ev.client_x(),
                                                    ev.client_y(),
                                                );
                                            }
                                        >
                                            <span class="taskbar-app-icon" aria-hidden="true">
                                                <FluentIcon icon=app_icon_name(win.app_id) size=IconSize::Sm />
                                            </span>
                                            <span class="taskbar-menu-item-label">{win.title.clone()}</span>
                                        </button>
                                    </For>
                                </div>
                            </Show>
                        </div>
                    </Show>
                </div>
            </div>

            <div class="taskbar-right">
                <div class="taskbar-tray" role="group" aria-label="System tray">
                    <For
                        each=move || build_taskbar_tray_widgets(&state.get())
                        key=|widget| widget.id
                        let:widget
                    >
                        <button
                            class=move || {
                                match widget.pressed {
                                    Some(true) => "tray-widget pressed",
                                    Some(false) => "tray-widget",
                                    None => "tray-widget passive",
                                }
                            }
                            aria-label=format!("{}: {}", widget.label, widget.value)
                            aria-pressed=widget.pressed.unwrap_or(false)
                            title=format!("{}: {}", widget.label, widget.value)
                            on:click=move |_| {
                                if !matches!(widget.action, TaskbarTrayWidgetAction::None) {
                                    window_context_menu.set(None);
                                    overflow_menu_open.set(false);
                                    runtime.dispatch_action(DesktopAction::CloseStartMenu);
                                }
                                activate_taskbar_tray_widget(runtime, widget.action);
                            }
                        >
                            <span class="tray-widget-glyph" aria-hidden="true">
                                <FluentIcon icon=widget.icon size=IconSize::Xs />
                            </span>
                            <span class="tray-widget-value">{widget.value.clone()}</span>
                        </button>
                    </For>
                </div>

                <div class="taskbar-clock-wrap">
                    <button
                        class="taskbar-clock"
                        aria-label=move || format_taskbar_clock_aria(clock_now.get(), clock_config.get())
                        aria-haspopup="menu"
                        aria-controls="taskbar-clock-menu"
                        aria-expanded=move || clock_menu_open.get()
                        on:click=move |_| {
                            window_context_menu.set(None);
                            overflow_menu_open.set(false);
                            runtime.dispatch_action(DesktopAction::CloseStartMenu);
                            clock_menu_open.update(|open| *open = !*open);
                        }
                    >
                        <span class="taskbar-clock-time">
                            {move || format_taskbar_clock_time(clock_now.get(), clock_config.get())}
                        </span>
                        <Show when=move || clock_config.get().show_date fallback=|| ()>
                            <span class="taskbar-clock-date">
                                {move || format_taskbar_clock_date(clock_now.get())}
                            </span>
                        </Show>
                    </button>

                    <Show when=move || clock_menu_open.get() fallback=|| ()>
                        <div
                            id="taskbar-clock-menu"
                            class="taskbar-menu taskbar-clock-menu"
                            role="menu"
                            aria-label="Clock settings"
                            on:mousedown=move |ev| ev.stop_propagation()
                        >
                            <button
                                role="menuitemcheckbox"
                                aria-checked=move || clock_config.get().use_24_hour
                                class="taskbar-menu-item"
                                on:click=move |_| {
                                    clock_config.update(|cfg| cfg.use_24_hour = !cfg.use_24_hour);
                                }
                            >
                                "24-hour time"
                            </button>
                            <button
                                role="menuitemcheckbox"
                                aria-checked=move || clock_config.get().show_date
                                class="taskbar-menu-item"
                                on:click=move |_| {
                                    clock_config.update(|cfg| cfg.show_date = !cfg.show_date);
                                }
                            >
                                "Show date"
                            </button>
                            <button
                                role="menuitem"
                                class="taskbar-menu-item"
                                on:click=move |_| clock_menu_open.set(false)
                            >
                                "Close"
                            </button>
                        </div>
                    </Show>
                </div>
            </div>

            <Show when=move || state.get().start_menu_open fallback=|| ()>
                <div
                    id="desktop-launcher-menu"
                    class="start-menu"
                    role="menu"
                    aria-label="Application launcher"
                    on:mousedown=move |ev| ev.stop_propagation()
                >
                    <For each=move || apps::launcher_apps() key=|app| app.app_id as u8 let:app>
                        <button
                            role="menuitem"
                            on:click=move |_| {
                                window_context_menu.set(None);
                                overflow_menu_open.set(false);
                                clock_menu_open.set(false);
                                runtime.dispatch_action(DesktopAction::OpenWindow(
                                    apps::default_open_request(app.app_id),
                                ));
                            }
                        >
                            <span class="taskbar-app-icon" aria-hidden="true">
                                <FluentIcon icon=app_icon_name(app.app_id) size=IconSize::Sm />
                            </span>
                            <span>{format!("Open {}", app.launcher_label)}</span>
                        </button>
                    </For>
                    <button
                        role="menuitem"
                        on:click=move |_| runtime.dispatch_action(DesktopAction::CloseStartMenu)
                    >
                        "Close"
                    </button>
                </div>
            </Show>

            <Show
                when=move || {
                    window_context_menu
                        .get()
                        .and_then(|menu| {
                            state
                                .get()
                                .windows
                                .into_iter()
                                .find(|win| win.id == menu.window_id)
                                .map(|win| (menu, win))
                        })
                        .is_some()
                }
                fallback=|| ()
            >
                {move || {
                    let Some((menu, win)) = window_context_menu.get().and_then(|menu| {
                        state
                            .get()
                            .windows
                            .into_iter()
                            .find(|win| win.id == menu.window_id)
                            .map(|win| (menu, win))
                    }) else {
                        return ().into_view();
                    };

                    let menu_style = format!("left:{}px;top:{}px;", menu.x, menu.y);
                    let can_focus = !win.is_focused && !win.minimized;
                    let can_restore = win.minimized || win.maximized;
                    let can_minimize = win.flags.minimizable && !win.minimized;
                    let can_maximize = win.flags.maximizable && !win.maximized;
                    let restore_label = if win.minimized {
                        "Restore"
                    } else {
                        "Restore Size"
                    };
                    let window_id = win.id;

                    view! {
                        <div
                            class="taskbar-menu taskbar-window-menu"
                            role="menu"
                            aria-label=format!("Window menu for {}", win.title)
                            style=menu_style
                            on:mousedown=move |ev| ev.stop_propagation()
                        >
                            <button
                                role="menuitem"
                                class="taskbar-menu-item"
                                disabled=!can_focus
                                on:click=move |_| {
                                    window_context_menu.set(None);
                                    let desktop = runtime.state.get_untracked();
                                    focus_or_unminimize_window(runtime, &desktop, window_id);
                                }
                            >
                                "Focus"
                            </button>
                            <button
                                role="menuitem"
                                class="taskbar-menu-item"
                                disabled=!can_restore
                                on:click=move |_| {
                                    window_context_menu.set(None);
                                    runtime.dispatch_action(DesktopAction::RestoreWindow { window_id });
                                }
                            >
                                {restore_label}
                            </button>
                            <button
                                role="menuitem"
                                class="taskbar-menu-item"
                                disabled=!can_minimize
                                on:click=move |_| {
                                    window_context_menu.set(None);
                                    runtime.dispatch_action(DesktopAction::MinimizeWindow { window_id });
                                }
                            >
                                "Minimize"
                            </button>
                            <button
                                role="menuitem"
                                class="taskbar-menu-item"
                                disabled=!can_maximize
                                on:click=move |_| {
                                    window_context_menu.set(None);
                                    runtime.dispatch_action(DesktopAction::MaximizeWindow {
                                        window_id,
                                        viewport: runtime.host.desktop_viewport_rect(
                                            TASKBAR_HEIGHT_PX,
                                        ),
                                    });
                                }
                            >
                                "Maximize"
                            </button>
                            <button
                                role="menuitem"
                                class="taskbar-menu-item danger"
                                on:click=move |_| {
                                    window_context_menu.set(None);
                                    runtime.dispatch_action(DesktopAction::CloseWindow { window_id });
                                }
                            >
                                "Close"
                            </button>
                        </div>
                    }
                        .into_view()
                }}
            </Show>
        </footer>
    }
}

fn stop_mouse_event(ev: &web_sys::MouseEvent) {
    ev.prevent_default();
    ev.stop_propagation();
}

fn pointer_from_mouse_event(ev: &web_sys::MouseEvent) -> PointerPosition {
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
