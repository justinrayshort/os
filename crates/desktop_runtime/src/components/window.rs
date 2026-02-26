use super::*;
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
                        on:pointerdown=focus
                        role="dialog"
                        aria-label=win.title.clone()
                    >
                        <header
                            class="titlebar"
                            on:pointerdown=begin_move
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
                                    disabled=!win.flags.maximizable
                                    aria-label=if win.maximized {
                                        "Restore window"
                                    } else {
                                        "Maximize window"
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
