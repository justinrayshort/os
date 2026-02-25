use leptos::*;

use crate::{
    apps,
    model::{DesktopState, InteractionState, WindowId},
    persistence,
    reducer::{build_open_request_from_deeplink, reduce_desktop, DesktopAction, RuntimeEffect},
};

#[derive(Clone, Copy)]
pub struct DesktopRuntimeContext {
    pub state: RwSignal<DesktopState>,
    pub interaction: RwSignal<InteractionState>,
    pub effects: RwSignal<Vec<RuntimeEffect>>,
    pub dispatch: Callback<DesktopAction>,
}

impl DesktopRuntimeContext {
    pub fn dispatch_action(&self, action: DesktopAction) {
        self.dispatch.call(action);
    }
}

#[component]
pub fn DesktopProvider(children: Children) -> impl IntoView {
    let state = create_rw_signal(DesktopState::default());
    let interaction = create_rw_signal(InteractionState::default());
    let effects = create_rw_signal(Vec::<RuntimeEffect>::new());

    let dispatch = Callback::new(move |action: DesktopAction| {
        state.update(|desktop| {
            interaction.update(|ui| match reduce_desktop(desktop, ui, action) {
                Ok(new_effects) => effects.update(|queue| queue.extend(new_effects)),
                Err(err) => {
                    logging::warn!("desktop reducer error: {err}");
                }
            });
        });
    });

    provide_context(DesktopRuntimeContext {
        state,
        interaction,
        effects,
        dispatch,
    });

    create_effect(move |_| {
        if let Some(snapshot) = persistence::load_boot_snapshot() {
            dispatch.call(DesktopAction::HydrateSnapshot { snapshot });
        }
    });

    children().into_view()
}

pub fn use_desktop_runtime() -> DesktopRuntimeContext {
    use_context::<DesktopRuntimeContext>().expect("DesktopRuntimeContext not provided")
}

#[component]
pub fn DesktopShell() -> impl IntoView {
    let runtime = use_desktop_runtime();
    let state = runtime.state;

    let on_toggle_menu = move |_| runtime.dispatch_action(DesktopAction::ToggleStartMenu);

    // Runtime effect runner: clear current queue before processing so nested dispatches enqueue a
    // fresh batch instead of getting wiped.
    create_effect(move |_| {
        let queued = runtime.effects.get();
        if queued.is_empty() {
            return;
        }

        runtime.effects.set(Vec::new());

        for effect in queued {
            match effect {
                RuntimeEffect::ParseAndOpenDeepLink(deep_link) => {
                    for target in deep_link.open {
                        runtime.dispatch_action(DesktopAction::OpenWindow(
                            build_open_request_from_deeplink(target),
                        ));
                    }
                }
                RuntimeEffect::PersistLayout => {
                    let snapshot_state = runtime.state.get_untracked();
                    if let Err(err) = persistence::persist_layout_snapshot(&snapshot_state) {
                        logging::warn!("persist layout failed: {err}");
                    }
                }
                RuntimeEffect::PersistTheme => {
                    let theme = runtime.state.get_untracked().theme;
                    if let Err(err) = persistence::persist_theme(&theme) {
                        logging::warn!("persist theme failed: {err}");
                    }
                }
                RuntimeEffect::PersistTerminalHistory => {
                    let history = runtime.state.get_untracked().terminal_history;
                    if let Err(err) = persistence::persist_terminal_history(&history) {
                        logging::warn!("persist terminal history failed: {err}");
                    }
                }
                RuntimeEffect::OpenExternalUrl(url) => {
                    logging::log!("open external url requested: {url}");
                }
                RuntimeEffect::FocusWindowInput(_) | RuntimeEffect::PlaySound(_) => {}
            }
        }
    });

    view! {
        <div class="desktop-shell" data-theme=move || state.get().theme.name>
            <div class="desktop-wallpaper">
                <div class="desktop-icons">
                    <For each=move || apps::desktop_icon_apps() key=|app| app.app_id as u8 let:app>
                        <button
                            class="desktop-icon"
                            on:click=move |_| {
                                runtime.dispatch_action(DesktopAction::OpenWindow(
                                    apps::default_open_request(app.app_id),
                                ));
                            }
                        >
                            <span class="icon">{app_icon_glyph(app.app_id)}</span>
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
            </div>

            <Taskbar on_toggle_menu=on_toggle_menu />
        </div>
    }
}

fn app_icon_glyph(app_id: crate::model::AppId) -> &'static str {
    match app_id {
        crate::model::AppId::Explorer => "[ ]",
        crate::model::AppId::Notepad => "|_|",
        crate::model::AppId::Paint => "o~",
        crate::model::AppId::Terminal => ">_",
        crate::model::AppId::Dialup => "()",
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

    let focus = move |_| runtime.dispatch_action(DesktopAction::FocusWindow { window_id });
    let minimize = move |_| runtime.dispatch_action(DesktopAction::MinimizeWindow { window_id });
    let close = move |_| runtime.dispatch_action(DesktopAction::CloseWindow { window_id });

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

                view! {
                    <section
                        class=format!("desktop-window{}{}", focused_class, minimized_class)
                        style=style
                        on:mousedown=focus
                        role="dialog"
                        aria-label=win.title.clone()
                    >
                        <header class="titlebar">
                            <div class="titlebar-title">{win.title.clone()}</div>
                            <div class="titlebar-controls">
                                <button on:click=minimize>"_"</button>
                                <button on:click=close>"X"</button>
                            </div>
                        </header>
                        <div class="window-body">
                            <WindowBody window_id=window_id />
                        </div>
                    </section>
                }
                    .into_view()
            }}
        </Show>
    }
}

#[component]
fn WindowBody(window_id: WindowId) -> impl IntoView {
    let runtime = use_desktop_runtime();
    let state = runtime.state;

    view! {
        <div class="window-body-content">
            {move || {
                state
                    .get()
                    .windows
                    .into_iter()
                    .find(|w| w.id == window_id)
                    .map(|w| apps::render_window_contents(&w))
                    .unwrap_or_else(|| view! { <p>"Closed"</p> }.into_view())
            }}
        </div>
    }
}

#[component]
fn Taskbar<F>(on_toggle_menu: F) -> impl IntoView
where
    F: Fn(web_sys::MouseEvent) + Clone + 'static,
{
    let runtime = use_desktop_runtime();
    let state = runtime.state;

    view! {
        <footer class="taskbar" role="toolbar" aria-label="Desktop taskbar">
            <button class="start-button" on:click=on_toggle_menu.clone()>
                "Launcher"
            </button>

            <div class="taskbar-windows">
                <For
                    each=move || state.get().windows
                    key=|win| win.id.0
                    let:win
                >
                    <button
                        class=move || {
                            if win.is_focused { "taskbar-app focused" } else { "taskbar-app" }
                        }
                        on:click=move |_| {
                            runtime.dispatch_action(DesktopAction::ToggleTaskbarWindow {
                                window_id: win.id,
                            });
                        }
                    >
                        {win.title.clone()}
                    </button>
                </For>
            </div>

            <Show
                when=move || state.get().start_menu_open
                fallback=|| ()
            >
                <div class="start-menu" role="menu" aria-label="Launcher menu">
                    <For each=move || apps::launcher_apps() key=|app| app.app_id as u8 let:app>
                        <button
                            role="menuitem"
                            on:click=move |_| {
                                runtime.dispatch_action(DesktopAction::OpenWindow(
                                    apps::default_open_request(app.app_id),
                                ));
                            }
                        >
                            {format!("Open {}", app.launcher_label)}
                        </button>
                    </For>
                    <button
                        role="menuitem"
                        on:click=move |_| {
                            runtime.dispatch_action(DesktopAction::CloseStartMenu);
                        }
                    >
                        "Close"
                    </button>
                </div>
            </Show>
        </footer>
    }
}
