use super::*;

#[component]
pub(super) fn Taskbar() -> impl IntoView {
    let runtime = use_desktop_runtime();
    let state = runtime.state;
    let shell_ui = use_context::<DesktopShellUiContext>();
    let modal_open = move || {
        shell_ui
            .map(|ctx| ctx.display_properties_open.get_untracked())
            .unwrap_or(false)
    };

    let viewport_width = create_rw_signal(runtime.host.desktop_viewport_rect(TASKBAR_HEIGHT_PX).w);
    let clock_config = create_rw_signal(TaskbarClockConfig::default());
    let clock_now = create_rw_signal(TaskbarClockSnapshot::now());
    let selected_running_window = create_rw_signal(None::<WindowId>);
    let window_context_menu = create_rw_signal(None::<TaskbarWindowContextMenuState>);
    let overflow_menu_open = create_rw_signal(false);
    let clock_menu_open = create_rw_signal(false);
    let start_menu_was_open = create_rw_signal(false);
    let overflow_menu_was_open = create_rw_signal(false);
    let clock_menu_was_open = create_rw_signal(false);
    let window_menu_was_open = create_rw_signal(false);

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
        if modal_open() {
            return;
        }
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
        if modal_open() {
            return;
        }
        if ev.default_prevented() {
            return;
        }
        if try_handle_taskbar_shortcuts(
            runtime,
            window_context_menu,
            overflow_menu_open,
            clock_menu_open,
            &ev,
        ) {
            return;
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

    create_effect(move |_| {
        let is_open = state.get().start_menu_open;
        let was_open = start_menu_was_open.get_untracked();
        if is_open && !was_open {
            start_menu_was_open.set(true);
            let _ = focus_first_menu_item("desktop-launcher-menu");
        } else if !is_open && was_open {
            start_menu_was_open.set(false);
        }
    });

    create_effect(move |_| {
        let is_open = overflow_menu_open.get();
        let was_open = overflow_menu_was_open.get_untracked();
        if is_open && !was_open {
            overflow_menu_was_open.set(true);
            let _ = focus_first_menu_item("taskbar-overflow-menu");
        } else if !is_open && was_open {
            overflow_menu_was_open.set(false);
        }
    });

    create_effect(move |_| {
        let is_open = clock_menu_open.get();
        let was_open = clock_menu_was_open.get_untracked();
        if is_open && !was_open {
            clock_menu_was_open.set(true);
            let _ = focus_first_menu_item("taskbar-clock-menu");
        } else if !is_open && was_open {
            clock_menu_was_open.set(false);
        }
    });

    create_effect(move |_| {
        let is_open = window_context_menu.get().is_some();
        let was_open = window_menu_was_open.get_untracked();
        if is_open && !was_open {
            window_menu_was_open.set(true);
            let _ = focus_first_menu_item("taskbar-window-context-menu");
        } else if !is_open && was_open {
            window_menu_was_open.set(false);
        }
    });

    let on_taskbar_keydown = move |ev: web_sys::KeyboardEvent| {
        if modal_open() {
            ev.prevent_default();
            ev.stop_propagation();
            return;
        }
        if try_handle_taskbar_shortcuts(
            runtime,
            window_context_menu,
            overflow_menu_open,
            clock_menu_open,
            &ev,
        ) {
            return;
        }

        match ev.key().as_str() {
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
            aria-hidden=move || if shell_ui.map(|ctx| ctx.display_properties_open.get()).unwrap_or(false) {
                "true"
            } else {
                "false"
            }
            aria-keyshortcuts="Ctrl+Escape Alt+1 Alt+2 Alt+3 Alt+4 Alt+5 Alt+6 Alt+7 Alt+8 Alt+9"
            on:mousedown:capture=move |ev: web_sys::MouseEvent| {
                if modal_open() {
                    ev.prevent_default();
                    ev.stop_propagation();
                }
            }
            on:click:capture=move |ev: web_sys::MouseEvent| {
                if modal_open() {
                    ev.prevent_default();
                    ev.stop_propagation();
                }
            }
            on:contextmenu:capture=move |ev: web_sys::MouseEvent| {
                if modal_open() {
                    ev.prevent_default();
                    ev.stop_propagation();
                }
            }
            on:mousedown=move |ev| ev.stop_propagation()
            on:keydown=on_taskbar_keydown
        >
            <div class="taskbar-left">
                <button
                    id="taskbar-start-button"
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
                            id=taskbar_window_button_dom_id(win.id)
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
                                id="taskbar-overflow-button"
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

                            <super::menus::OverflowMenu
                                state
                                runtime
                                viewport_width
                                clock_config
                                selected_running_window
                                window_context_menu
                                overflow_menu_open
                                clock_menu_open
                            />
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
                        id="taskbar-clock-button"
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

                    <super::menus::ClockMenu clock_config clock_menu_open />
                </div>
            </div>

            <super::menus::StartMenu
                state
                runtime
                window_context_menu
                overflow_menu_open
                clock_menu_open
            />

            <super::menus::TaskbarWindowContextMenu
                state
                runtime
                selected_running_window
                window_context_menu
            />
        </footer>
    }
}
