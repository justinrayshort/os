//! Host-side runtime helpers for executing reducer effects and querying browser environment state.
//!
//! This module is the first extraction point for desktop shell side effects. It keeps reducer
//! semantics unchanged while moving effect execution and viewport/window queries behind a typed
//! boundary that can later be injected and mocked.

use leptos::*;

use crate::{
    components::DesktopRuntimeContext,
    model::WindowRect,
    persistence,
    reducer::{build_open_request_from_deeplink, DesktopAction, RuntimeEffect},
};

#[derive(Debug, Clone, Copy, Default)]
/// Host service bundle for desktop runtime side effects.
pub struct DesktopHostContext;

impl DesktopHostContext {
    /// Executes a single [`RuntimeEffect`] emitted by the reducer.
    pub fn run_runtime_effect(self, runtime: DesktopRuntimeContext, effect: RuntimeEffect) {
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
                self.persist_durable_snapshot(snapshot_state, "layout");
            }
            RuntimeEffect::PersistTheme => {
                let theme = runtime.state.get_untracked().theme;
                if let Err(err) = persistence::persist_theme(&theme) {
                    logging::warn!("persist theme failed: {err}");
                }
                self.persist_durable_snapshot(runtime.state.get_untracked(), "theme");
            }
            RuntimeEffect::PersistTerminalHistory => {
                let history = runtime.state.get_untracked().terminal_history;
                if let Err(err) = persistence::persist_terminal_history(&history) {
                    logging::warn!("persist terminal history failed: {err}");
                }
                self.persist_durable_snapshot(runtime.state.get_untracked(), "terminal");
            }
            RuntimeEffect::OpenExternalUrl(url) => {
                logging::log!("open external url requested: {url}");
            }
            RuntimeEffect::FocusWindowInput(_) | RuntimeEffect::PlaySound(_) => {}
        }
    }

    fn persist_durable_snapshot(self, state: crate::model::DesktopState, cause: &str) {
        let durable_envelope = persistence::build_durable_layout_snapshot_envelope(&state);
        match durable_envelope {
            Ok(envelope) => {
                let cause = cause.to_string();
                spawn_local(async move {
                    if let Err(err) =
                        persistence::persist_durable_layout_snapshot_envelope(&envelope).await
                    {
                        logging::warn!("persist durable {cause} snapshot failed: {err}");
                    }
                });
            }
            Err(err) => logging::warn!("build durable {cause} envelope failed: {err}"),
        }
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

