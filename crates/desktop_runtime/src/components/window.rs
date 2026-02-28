use super::*;
use crate::app_runtime::ensure_window_session;
use crate::shell;
use desktop_app_calculator::CalculatorApp;
use desktop_app_contract::{AppMountContext, AppServices};
use desktop_app_explorer::ExplorerApp;
use desktop_app_notepad::NotepadApp;
use desktop_app_settings::SettingsApp;
use desktop_app_terminal::TerminalApp;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
fn try_set_pointer_capture(ev: &web_sys::PointerEvent) {
    if let Some(target) = ev.current_target() {
        if let Ok(element) = target.dyn_into::<web_sys::Element>() {
            let _ = element.set_pointer_capture(ev.pointer_id());
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn try_set_pointer_capture(_: &web_sys::PointerEvent) {}

#[component]
pub(super) fn DesktopWindow(window_id: WindowId) -> impl IntoView {
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
    let begin_move = move |ev: web_sys::PointerEvent| {
        if ev.pointer_type() == "mouse" && ev.button() != 0 {
            return;
        }
        if ev.pointer_type() != "mouse" && !ev.is_primary() {
            return;
        }
        try_set_pointer_capture(&ev);
        if ev.button() != 0 {
            return;
        }
        ev.prevent_default();
        ev.stop_propagation();
        runtime.dispatch_action(DesktopAction::BeginMove {
            window_id,
            pointer: pointer_from_pointer_event(&ev),
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
            <section
                class=move || {
                    let win = window.get().expect("window exists while shown");
                    let focused_class = if win.is_focused { " focused" } else { "" };
                    let minimized_class = if win.minimized { " minimized" } else { "" };
                    let maximized_class = if win.maximized { " maximized" } else { "" };
                    format!(
                        "desktop-window{}{}{}",
                        focused_class,
                        minimized_class,
                        maximized_class
                    )
                }
                style=move || {
                    let win = window.get().expect("window exists while shown");
                    format!(
                        "left:{}px;top:{}px;width:{}px;height:{}px;z-index:{};",
                        win.rect.x, win.rect.y, win.rect.w, win.rect.h, win.z_index
                    )
                }
                on:pointerdown=focus
                role="dialog"
                aria-label=move || {
                    window
                        .get()
                        .map(|win| win.title)
                        .unwrap_or_default()
                }
            >
                <header
                    class="titlebar"
                    on:pointerdown=begin_move
                    on:dblclick=titlebar_double_click
                >
                    <div class="titlebar-title">
                        <span class="titlebar-app-icon" aria-hidden="true">
                            <FluentIcon
                                icon=app_icon_name(&apps::builtin_application_id(
                                    window
                                        .get_untracked()
                                        .expect("window exists while shown")
                                        .app_id,
                                ))
                                size=IconSize::Sm
                            />
                        </span>
                        <span>
                            {move || {
                                window
                                    .get()
                                    .map(|win| win.title)
                                    .unwrap_or_default()
                            }}
                        </span>
                    </div>
                    <div class="titlebar-controls">
                        <button
                            disabled=move || {
                                !window
                                    .get()
                                    .expect("window exists while shown")
                                    .flags
                                    .minimizable
                            }
                            aria-label="Minimize window"
                            on:pointerdown=move |ev: web_sys::PointerEvent| {
                                ev.prevent_default();
                                ev.stop_propagation();
                            }
                            on:mousedown=move |ev| stop_mouse_event(&ev)
                            on:click=move |ev| {
                                stop_mouse_event(&ev);
                                minimize(ev);
                            }
                        >
                            <FluentIcon icon=IconName::WindowMinimize size=IconSize::Xs />
                        </button>
                        <button
                            disabled=move || {
                                !window
                                    .get()
                                    .expect("window exists while shown")
                                    .flags
                                    .maximizable
                            }
                            aria-label=move || {
                                if window
                                    .get()
                                    .expect("window exists while shown")
                                    .maximized
                                {
                                    "Restore window"
                                } else {
                                    "Maximize window"
                                }
                            }
                            on:pointerdown=move |ev: web_sys::PointerEvent| {
                                ev.prevent_default();
                                ev.stop_propagation();
                            }
                            on:mousedown=move |ev| stop_mouse_event(&ev)
                            on:click=move |ev| {
                                stop_mouse_event(&ev);
                                toggle_maximize(ev);
                            }
                        >
                            {move || {
                                if window
                                    .get()
                                    .expect("window exists while shown")
                                    .maximized
                                {
                                    view! { <FluentIcon icon=IconName::WindowRestore size=IconSize::Xs /> }
                                } else {
                                    view! { <FluentIcon icon=IconName::WindowMaximize size=IconSize::Xs /> }
                                }
                            }}
                        </button>
                        <button
                            aria-label="Close window"
                            on:pointerdown=move |ev: web_sys::PointerEvent| {
                                ev.prevent_default();
                                ev.stop_propagation();
                            }
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
        </Show>
    }
}

#[component]
fn WindowResizeHandle(window_id: WindowId, edge: ResizeEdge) -> impl IntoView {
    let runtime = use_desktop_runtime();
    let class_name = format!("window-resize-handle {}", resize_edge_class(edge));

    let on_pointerdown = move |ev: web_sys::PointerEvent| {
        if ev.pointer_type() == "mouse" && ev.button() != 0 {
            return;
        }
        if ev.pointer_type() != "mouse" && !ev.is_primary() {
            return;
        }
        try_set_pointer_capture(&ev);
        ev.prevent_default();
        ev.stop_propagation();
        runtime.dispatch_action(DesktopAction::BeginResize {
            window_id,
            edge,
            pointer: pointer_from_pointer_event(&ev),
            viewport: runtime.host.desktop_viewport_rect(TASKBAR_HEIGHT_PX),
        });
    };

    view! {
        <div
            class=class_name
            aria-hidden="true"
            on:pointerdown=on_pointerdown
        />
    }
}

#[component]
fn WindowBody(window_id: WindowId) -> impl IntoView {
    let runtime = use_desktop_runtime();
    let state = runtime.state;
    let session = ensure_window_session(runtime.app_runtime, window_id);
    let lifecycle = session.lifecycle.read_only();
    let inbox = session.inbox;
    let theme_skin_id = create_rw_signal(
        runtime
            .state
            .get_untracked()
            .theme
            .skin
            .css_id()
            .to_string(),
    );
    let theme_high_contrast = create_rw_signal(runtime.state.get_untracked().theme.high_contrast);
    let theme_reduced_motion = create_rw_signal(runtime.state.get_untracked().theme.reduced_motion);
    let wallpaper_current = create_rw_signal(runtime.state.get_untracked().wallpaper);
    let wallpaper_preview = create_rw_signal(runtime.state.get_untracked().wallpaper_preview);
    let wallpaper_library = create_rw_signal(runtime.state.get_untracked().wallpaper_library);
    let terminal_history = create_rw_signal(runtime.state.get_untracked().terminal_history);
    create_effect(move |_| {
        let desktop = runtime.state.get();
        theme_skin_id.set(desktop.theme.skin.css_id().to_string());
        theme_high_contrast.set(desktop.theme.high_contrast);
        theme_reduced_motion.set(desktop.theme.reduced_motion);
        wallpaper_current.set(desktop.wallpaper);
        wallpaper_preview.set(desktop.wallpaper_preview);
        wallpaper_library.set(desktop.wallpaper_library);
        terminal_history.set(desktop.terminal_history);
    });
    let command_sender = Callback::new(move |command| {
        spawn_local(async move {
            runtime.dispatch_action(DesktopAction::HandleAppCommand { window_id, command });
        });
    });
    let services = store_value(AppServices::new(
        command_sender,
        theme_skin_id.read_only(),
        theme_high_contrast.read_only(),
        theme_reduced_motion.read_only(),
        wallpaper_current.read_only(),
        wallpaper_preview.read_only(),
        wallpaper_library.read_only(),
        shell::build_command_service(
            runtime.clone(),
            state
                .get_untracked()
                .windows
                .into_iter()
                .find(|w| w.id == window_id)
                .map(|w| w.app_id)
                .expect("window app id"),
            window_id,
            terminal_history.read_only(),
        ),
    ));
    let mounted_window = state
        .get_untracked()
        .windows
        .into_iter()
        .find(|window| window.id == window_id)
        .expect("window exists while body is mounted");
    let contents = view! {
        <MountedManagedApp
            app_id=mounted_window.app_id
            context=AppMountContext {
                app_id: apps::builtin_application_id(mounted_window.app_id),
                window_id: mounted_window.id.0,
                launch_params: mounted_window.launch_params.clone(),
                restored_state: mounted_window.app_state.clone(),
                lifecycle,
                inbox,
                services: services.get_value(),
            }
        />
    };

    view! {
        <div class="window-body-content">
            {contents}
        </div>
    }
}

#[component]
fn MountedManagedApp(app_id: AppId, context: AppMountContext) -> impl IntoView {
    match app_id {
        AppId::Calculator => view! {
            <CalculatorApp
                launch_params=context.launch_params.clone()
                restored_state=Some(context.restored_state.clone())
                services=Some(context.services.clone())
            />
        }
        .into_view(),
        AppId::Explorer => view! {
            <ExplorerApp
                launch_params=context.launch_params.clone()
                restored_state=Some(context.restored_state.clone())
                services=Some(context.services.clone())
                inbox=Some(context.inbox)
            />
        }
        .into_view(),
        AppId::Notepad => view! {
            <NotepadApp
                launch_params=context.launch_params.clone()
                restored_state=Some(context.restored_state.clone())
                services=Some(context.services.clone())
            />
        }
        .into_view(),
        AppId::Terminal => view! {
            <TerminalApp
                window_id=context.window_id
                launch_params=context.launch_params.clone()
                restored_state=Some(context.restored_state.clone())
                services=Some(context.services.clone())
            />
        }
        .into_view(),
        AppId::Settings => view! {
            <SettingsApp
                _launch_params=context.launch_params.clone()
                restored_state=Some(context.restored_state.clone())
                services=Some(context.services.clone())
            />
        }
        .into_view(),
        _ => apps::app_module(app_id).mount(context),
    }
}
