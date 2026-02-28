//! Host-side runtime helpers for executing reducer effects and querying browser environment state.
//!
//! This module is the first extraction point for desktop shell side effects. It keeps reducer
//! semantics unchanged while moving effect execution and viewport/window queries behind a typed
//! boundary that can later be injected and mocked.

mod app_bus;
mod boot;
mod host_ui;
mod persistence_effects;
mod wallpaper_effects;

use leptos::{logging, spawn_local, Callback};

use crate::{
    components::DesktopRuntimeContext,
    model::WindowRect,
    persistence,
    reducer::{DesktopAction, RuntimeEffect},
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
        boot::install_boot_hydration(dispatch);
    }

    /// Executes a single [`RuntimeEffect`] emitted by the reducer.
    pub fn run_runtime_effect(self, runtime: DesktopRuntimeContext, effect: RuntimeEffect) {
        match effect {
            RuntimeEffect::ParseAndOpenDeepLink(deep_link) => host_ui::open_deep_link(runtime, deep_link),
            RuntimeEffect::PersistLayout => persistence_effects::persist_layout(self, runtime),
            RuntimeEffect::PersistTheme => persistence_effects::persist_theme(self, runtime),
            RuntimeEffect::PersistWallpaper => persistence_effects::persist_wallpaper(runtime),
            RuntimeEffect::PersistTerminalHistory => {
                persistence_effects::persist_terminal_history(self, runtime)
            }
            RuntimeEffect::OpenExternalUrl(url) => host_ui::open_external_url(&url),
            RuntimeEffect::FocusWindowInput(window_id) => self.focus_window_input(window_id),
            RuntimeEffect::PlaySound(_) => {}
            RuntimeEffect::DispatchLifecycle { window_id, event } => {
                app_bus::dispatch_lifecycle(runtime, window_id, event);
            }
            RuntimeEffect::DeliverAppEvent { window_id, event } => {
                app_bus::deliver_app_event(runtime, window_id, event);
            }
            RuntimeEffect::SubscribeWindowTopic { window_id, topic } => {
                app_bus::subscribe_topic(runtime, window_id, topic);
            }
            RuntimeEffect::UnsubscribeWindowTopic { window_id, topic } => {
                app_bus::unsubscribe_topic(runtime, window_id, topic);
            }
            RuntimeEffect::PublishTopicEvent {
                source_window_id,
                topic,
                payload,
                correlation_id,
                reply_to,
            } => app_bus::publish_event(
                runtime,
                source_window_id,
                topic,
                payload,
                correlation_id,
                reply_to,
            ),
            RuntimeEffect::SaveConfig {
                namespace,
                key,
                value,
            } => persistence_effects::save_config(namespace, key, value),
            RuntimeEffect::LoadWallpaperLibrary => wallpaper_effects::load_library(runtime),
            RuntimeEffect::ImportWallpaperFromPicker { request } => {
                wallpaper_effects::import_from_picker(runtime, request);
            }
            RuntimeEffect::UpdateWallpaperAssetMetadata { asset_id, patch } => {
                wallpaper_effects::update_asset_metadata(runtime, asset_id, patch);
            }
            RuntimeEffect::CreateWallpaperCollection { display_name } => {
                wallpaper_effects::create_collection(runtime, display_name);
            }
            RuntimeEffect::RenameWallpaperCollection {
                collection_id,
                display_name,
            } => {
                wallpaper_effects::rename_collection(runtime, collection_id, display_name);
            }
            RuntimeEffect::DeleteWallpaperCollection { collection_id } => {
                wallpaper_effects::delete_collection(runtime, collection_id);
            }
            RuntimeEffect::DeleteWallpaperAsset { asset_id } => {
                wallpaper_effects::delete_asset(runtime, asset_id);
            }
            RuntimeEffect::Notify { title, body } => host_ui::notify(title, body),
        }
    }

    /// Handles a request to focus the active window's primary input.
    ///
    /// The reducer emits this intent when a window opens or is focused. Apps opt in by rendering
    /// [`desktop_app_contract::window_primary_input_dom_id`] on their primary text field.
    pub fn focus_window_input(self, window_id: crate::model::WindowId) {
        host_ui::focus_window_input(window_id);
    }

    /// Handles requests to open a URL outside the desktop shell.
    ///
    /// This is intentionally a host hook so browser integration can evolve independently of the UI
    /// reducer/effect pipeline.
    pub fn open_external_url(self, url: &str) {
        host_ui::open_external_url(url);
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
        host_ui::desktop_viewport_rect(taskbar_height_px)
    }
}
