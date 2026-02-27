//! Host-side runtime helpers for executing reducer effects and querying browser environment state.
//!
//! This module is the first extraction point for desktop shell side effects. It keeps reducer
//! semantics unchanged while moving effect execution and viewport/window queries behind a typed
//! boundary that can later be injected and mocked.

use leptos::*;

use crate::{
    app_runtime::{
        deliver_window_event, publish_topic_event, set_window_lifecycle, subscribe_window_topic,
        unsubscribe_window_topic,
    },
    components::DesktopRuntimeContext,
    model::WindowRect,
    persistence,
    reducer::{build_open_request_from_deeplink, DesktopAction, RuntimeEffect},
};

#[derive(Debug, Clone, Copy, Default)]
/// Host service bundle for desktop runtime side effects.
pub struct DesktopHostContext;

impl DesktopHostContext {
    /// Installs boot hydration/migration side effects for the desktop provider.
    ///
    /// This preserves the current boot sequence:
    /// 1. hydrate from compatibility snapshot first (if present)
    /// 2. asynchronously hydrate from durable storage if present
    /// 3. otherwise migrate the legacy snapshot into durable storage
    pub fn install_boot_hydration(self, dispatch: Callback<DesktopAction>) {
        create_effect(move |_| {
            let dispatch = dispatch;
            spawn_local(async move {
                let legacy_snapshot = persistence::load_boot_snapshot().await;
                if let Some(snapshot) = legacy_snapshot.clone() {
                    dispatch.call(DesktopAction::HydrateSnapshot { snapshot });
                }

                if let Some(snapshot) = persistence::load_durable_boot_snapshot().await {
                    dispatch.call(DesktopAction::HydrateSnapshot { snapshot });
                } else if let Some(snapshot) = legacy_snapshot {
                    let migrated_state = crate::model::DesktopState::from_snapshot(snapshot);
                    if let Err(err) =
                        persistence::persist_durable_layout_snapshot(&migrated_state).await
                    {
                        logging::warn!("migrate legacy snapshot to durable store failed: {err}");
                    }
                }
            });
        });
    }

    /// Executes a single [`RuntimeEffect`] emitted by the reducer.
    pub fn run_runtime_effect(self, runtime: DesktopRuntimeContext, effect: RuntimeEffect) {
        match effect {
            RuntimeEffect::ParseAndOpenDeepLink(deep_link) => {
                for target in deep_link.open {
                    match target {
                        crate::model::DeepLinkOpenTarget::App(app_id) => {
                            runtime.dispatch_action(DesktopAction::ActivateApp { app_id });
                        }
                        target => {
                            runtime.dispatch_action(DesktopAction::OpenWindow(
                                build_open_request_from_deeplink(target),
                            ));
                        }
                    }
                }
            }
            RuntimeEffect::PersistLayout => {
                let snapshot_state = runtime.state.get_untracked();
                if let Err(err) = persistence::persist_layout_snapshot(&snapshot_state) {
                    logging::warn!("persist layout failed: {err}");
                }
                self.persist_durable_snapshot(snapshot_state, "layout");
            }
            RuntimeEffect::PersistTheme => {
                let theme = runtime.state.get_untracked().theme;
                spawn_local(async move {
                    if let Err(err) = persistence::persist_theme(&theme).await {
                        logging::warn!("persist theme failed: {err}");
                    }
                });
                self.persist_durable_snapshot(runtime.state.get_untracked(), "theme");
            }
            RuntimeEffect::PersistTerminalHistory => {
                let history = runtime.state.get_untracked().terminal_history;
                spawn_local(async move {
                    if let Err(err) = persistence::persist_terminal_history(&history).await {
                        logging::warn!("persist terminal history failed: {err}");
                    }
                });
                self.persist_durable_snapshot(runtime.state.get_untracked(), "terminal");
            }
            RuntimeEffect::OpenExternalUrl(url) => self.open_external_url(&url),
            RuntimeEffect::FocusWindowInput(window_id) => self.focus_window_input(window_id),
            RuntimeEffect::PlaySound(_) => {}
            RuntimeEffect::DispatchLifecycle { window_id, event } => {
                set_window_lifecycle(runtime.app_runtime, window_id, event);
            }
            RuntimeEffect::DeliverAppEvent { window_id, event } => {
                deliver_window_event(runtime.app_runtime, window_id, event);
            }
            RuntimeEffect::SubscribeWindowTopic { window_id, topic } => {
                subscribe_window_topic(runtime.app_runtime, window_id, &topic);
            }
            RuntimeEffect::UnsubscribeWindowTopic { window_id, topic } => {
                unsubscribe_window_topic(runtime.app_runtime, window_id, &topic);
            }
            RuntimeEffect::PublishTopicEvent {
                source_window_id,
                topic,
                payload,
            } => {
                publish_topic_event(runtime.app_runtime, source_window_id, &topic, payload);
            }
        }
    }

    /// Handles a request to focus the active window's primary input.
    ///
    /// The reducer emits this intent when a window opens or is focused. The desktop shell does not
    /// yet assign stable DOM anchors per app window, so this remains a no-op host hook for now.
    pub fn focus_window_input(self, _window_id: crate::model::WindowId) {}

    /// Handles requests to open a URL outside the desktop shell.
    ///
    /// This is intentionally a host hook so browser integration can evolve independently of the UI
    /// reducer/effect pipeline.
    pub fn open_external_url(self, url: &str) {
        logging::log!("open external url requested: {url}");
    }

    fn persist_durable_snapshot(self, state: crate::model::DesktopState, cause: &str) {
        let cause = cause.to_string();
        spawn_local(async move {
            if let Err(err) = persistence::persist_durable_layout_snapshot(&state).await {
                logging::warn!("persist durable {cause} snapshot failed: {err}");
            }
        });
    }

    /// Returns the current desktop viewport rect available to the shell window manager.
    pub fn desktop_viewport_rect(self, taskbar_height_px: i32) -> WindowRect {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                let width = window
                    .inner_width()
                    .ok()
                    .and_then(|value| value.as_f64())
                    .map(|value| value as i32)
                    .unwrap_or(1024);
                let height = window
                    .inner_height()
                    .ok()
                    .and_then(|value| value.as_f64())
                    .map(|value| value as i32)
                    .unwrap_or(768);

                return WindowRect {
                    x: 0,
                    y: 0,
                    w: width.max(220),
                    h: (height - taskbar_height_px).max(140),
                };
            }
        }

        WindowRect {
            x: 0,
            y: 0,
            w: 1024,
            h: 768 - taskbar_height_px,
        }
    }
}
