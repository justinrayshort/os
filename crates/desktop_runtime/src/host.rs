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

use std::rc::Rc;

use leptos::{logging, spawn_local, Callback};
use platform_host::{
    AppStateStore, ContentCache, ExplorerFsService, ExternalUrlService, NotificationService,
    PrefsStore, WallpaperAssetService,
};
use platform_host_web::{
    app_state_store, content_cache, explorer_fs_service, external_url_service, host_strategy_name,
    notification_service, prefs_store, wallpaper_asset_service,
};

use crate::{
    components::DesktopRuntimeContext,
    model::WindowRect,
    persistence,
    reducer::{DesktopAction, RuntimeEffect},
};

#[derive(Clone)]
/// Host service bundle for desktop runtime side effects.
pub struct DesktopHostContext {
    app_state: Rc<dyn AppStateStore>,
    prefs: Rc<dyn PrefsStore>,
    explorer: Rc<dyn ExplorerFsService>,
    cache: Rc<dyn ContentCache>,
    external_urls: Rc<dyn ExternalUrlService>,
    notifications: Rc<dyn NotificationService>,
    wallpaper: Rc<dyn WallpaperAssetService>,
    host_strategy_name: &'static str,
}

impl Default for DesktopHostContext {
    fn default() -> Self {
        Self {
            app_state: Rc::new(app_state_store()),
            prefs: Rc::new(prefs_store()),
            explorer: Rc::new(explorer_fs_service()),
            cache: Rc::new(content_cache()),
            external_urls: Rc::new(external_url_service()),
            notifications: Rc::new(notification_service()),
            wallpaper: Rc::new(wallpaper_asset_service()),
            host_strategy_name: host_strategy_name(),
        }
    }
}

impl DesktopHostContext {
    /// Returns the configured app-state persistence service.
    pub fn app_state_store(&self) -> Rc<dyn AppStateStore> {
        self.app_state.clone()
    }

    /// Returns the configured lightweight preference service.
    pub fn prefs_store(&self) -> Rc<dyn PrefsStore> {
        self.prefs.clone()
    }

    /// Returns the configured explorer/filesystem service.
    pub fn explorer_fs_service(&self) -> Rc<dyn ExplorerFsService> {
        self.explorer.clone()
    }

    /// Returns the configured content cache service.
    pub fn content_cache(&self) -> Rc<dyn ContentCache> {
        self.cache.clone()
    }

    /// Returns the configured external URL service.
    pub fn external_url_service(&self) -> Rc<dyn ExternalUrlService> {
        self.external_urls.clone()
    }

    /// Returns the configured notification delivery service.
    pub fn notification_service(&self) -> Rc<dyn NotificationService> {
        self.notifications.clone()
    }

    /// Returns the configured wallpaper asset/library service.
    pub fn wallpaper_asset_service(&self) -> Rc<dyn WallpaperAssetService> {
        self.wallpaper.clone()
    }

    /// Returns the stable name of the selected host strategy.
    pub fn host_strategy_name(&self) -> &'static str {
        self.host_strategy_name
    }

    /// Installs boot hydration/migration side effects for the desktop provider.
    ///
    /// This preserves the current boot sequence:
    /// 1. hydrate from compatibility snapshot first (if present)
    /// 2. asynchronously hydrate from durable storage if present
    /// 3. otherwise migrate the legacy snapshot into durable storage
    pub fn install_boot_hydration(&self, dispatch: Callback<DesktopAction>) {
        boot::install_boot_hydration(self.clone(), dispatch);
    }

    /// Executes a single [`RuntimeEffect`] emitted by the reducer.
    pub fn run_runtime_effect(&self, runtime: DesktopRuntimeContext, effect: RuntimeEffect) {
        match effect {
            RuntimeEffect::ParseAndOpenDeepLink(deep_link) => {
                host_ui::open_deep_link(runtime, deep_link)
            }
            RuntimeEffect::PersistLayout => {
                persistence_effects::persist_layout(self.clone(), runtime)
            }
            RuntimeEffect::PersistTheme => {
                persistence_effects::persist_theme(self.clone(), runtime)
            }
            RuntimeEffect::PersistWallpaper => {
                persistence_effects::persist_wallpaper(self.clone(), runtime)
            }
            RuntimeEffect::PersistTerminalHistory => {
                persistence_effects::persist_terminal_history(self.clone(), runtime)
            }
            RuntimeEffect::OpenExternalUrl(url) => host_ui::open_external_url(self.clone(), &url),
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
            } => persistence_effects::save_config(self.clone(), namespace, key, value),
            RuntimeEffect::LoadWallpaperLibrary => {
                wallpaper_effects::load_library(self.clone(), runtime)
            }
            RuntimeEffect::ImportWallpaperFromPicker { request } => {
                wallpaper_effects::import_from_picker(self.clone(), runtime, request);
            }
            RuntimeEffect::UpdateWallpaperAssetMetadata { asset_id, patch } => {
                wallpaper_effects::update_asset_metadata(self.clone(), runtime, asset_id, patch);
            }
            RuntimeEffect::CreateWallpaperCollection { display_name } => {
                wallpaper_effects::create_collection(self.clone(), runtime, display_name);
            }
            RuntimeEffect::RenameWallpaperCollection {
                collection_id,
                display_name,
            } => {
                wallpaper_effects::rename_collection(
                    self.clone(),
                    runtime,
                    collection_id,
                    display_name,
                );
            }
            RuntimeEffect::DeleteWallpaperCollection { collection_id } => {
                wallpaper_effects::delete_collection(self.clone(), runtime, collection_id);
            }
            RuntimeEffect::DeleteWallpaperAsset { asset_id } => {
                wallpaper_effects::delete_asset(self.clone(), runtime, asset_id);
            }
            RuntimeEffect::Notify { title, body } => host_ui::notify(self.clone(), title, body),
        }
    }

    /// Handles a request to focus the active window's primary input.
    ///
    /// The reducer emits this intent when a window opens or is focused. Apps opt in by rendering
    /// [`desktop_app_contract::window_primary_input_dom_id`] on their primary text field.
    pub fn focus_window_input(&self, window_id: crate::model::WindowId) {
        host_ui::focus_window_input(window_id);
    }

    /// Handles requests to open a URL outside the desktop shell.
    ///
    /// This is intentionally a host hook so browser integration can evolve independently of the UI
    /// reducer/effect pipeline.
    pub fn open_external_url(&self, url: &str) {
        host_ui::open_external_url(self.clone(), url);
    }

    fn persist_durable_snapshot(&self, state: crate::model::DesktopState, cause: &str) {
        let cause = cause.to_string();
        let host = self.clone();
        spawn_local(async move {
            if let Err(err) = persistence::persist_durable_layout_snapshot(&host, &state).await {
                logging::warn!("persist durable {cause} snapshot failed: {err}");
            }
        });
    }

    /// Returns the current desktop viewport rect available to the shell window manager.
    pub fn desktop_viewport_rect(&self, taskbar_height_px: i32) -> WindowRect {
        host_ui::desktop_viewport_rect(taskbar_height_px)
    }
}
