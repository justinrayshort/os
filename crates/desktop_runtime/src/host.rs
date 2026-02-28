//! Host-side runtime helpers for executing reducer effects and querying browser environment state.
//!
//! This module is the first extraction point for desktop shell side effects. It keeps reducer
//! semantics unchanged while moving effect execution and viewport/window queries behind a typed
//! boundary that can later be injected and mocked.

#[cfg(target_arch = "wasm32")]
use desktop_app_contract::window_primary_input_dom_id;
use leptos::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{closure::Closure, JsCast};

use crate::{
    app_runtime::{
        deliver_window_event, publish_topic_event, set_window_lifecycle, subscribe_window_topic,
        unsubscribe_window_topic,
    },
    components::DesktopRuntimeContext,
    model::WindowRect,
    persistence,
    reducer::{build_open_request_from_deeplink, DesktopAction, RuntimeEffect},
    wallpaper,
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

                if let Some(theme) = persistence::load_theme().await {
                    dispatch.call(DesktopAction::HydrateTheme { theme });
                }

                if let Some(wallpaper) = persistence::load_wallpaper().await {
                    dispatch.call(DesktopAction::HydrateWallpaper { wallpaper });
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

                match platform_storage::wallpaper_list_library().await {
                    Ok(snapshot) => {
                        dispatch.call(DesktopAction::WallpaperLibraryLoaded { snapshot });
                    }
                    Err(err) => logging::warn!("wallpaper library load failed: {err}"),
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
                            runtime.dispatch_action(DesktopAction::ActivateApp {
                                app_id,
                                viewport: Some(runtime.host.desktop_viewport_rect(38)),
                            });
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
            RuntimeEffect::PersistWallpaper => {
                let wallpaper = runtime.state.get_untracked().wallpaper;
                spawn_local(async move {
                    if let Err(err) = persistence::persist_wallpaper(&wallpaper).await {
                        logging::warn!("persist wallpaper failed: {err}");
                    }
                });
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
                correlation_id,
                reply_to,
            } => {
                publish_topic_event(
                    runtime.app_runtime,
                    source_window_id,
                    &topic,
                    payload,
                    correlation_id,
                    reply_to,
                );
            }
            RuntimeEffect::SaveConfig {
                namespace,
                key,
                value,
            } => {
                let pref_key = format!("{}.{}", namespace, key);
                spawn_local(async move {
                    if let Err(err) = platform_storage::save_pref_typed(&pref_key, &value).await {
                        logging::warn!("persist config preference failed: {err}");
                    }
                });
            }
            RuntimeEffect::LoadWallpaperLibrary => {
                spawn_local(async move {
                    match platform_storage::wallpaper_list_library().await {
                        Ok(snapshot) => {
                            runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded {
                                snapshot,
                            });
                        }
                        Err(err) => logging::warn!("wallpaper library load failed: {err}"),
                    }
                });
            }
            RuntimeEffect::ImportWallpaperFromPicker { request } => {
                spawn_local(async move {
                    match platform_storage::wallpaper_import_from_picker(request.clone()).await {
                        Ok(asset) => {
                            let snapshot = match platform_storage::wallpaper_list_library().await {
                                Ok(snapshot) => snapshot,
                                Err(err) => {
                                    logging::warn!(
                                        "wallpaper library refresh failed after import: {err}"
                                    );
                                    return;
                                }
                            };
                            runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded {
                                snapshot,
                            });
                            let config = request.default_config.unwrap_or_else(|| {
                                let animation = match asset.media_kind {
                                    desktop_app_contract::WallpaperMediaKind::AnimatedImage
                                    | desktop_app_contract::WallpaperMediaKind::Video => {
                                        desktop_app_contract::WallpaperAnimationPolicy::LoopMuted
                                    }
                                    _ => desktop_app_contract::WallpaperAnimationPolicy::None,
                                };
                                desktop_app_contract::WallpaperConfig {
                                    selection: desktop_app_contract::WallpaperSelection::Imported {
                                        asset_id: asset.asset_id,
                                    },
                                    display_mode: desktop_app_contract::WallpaperDisplayMode::Fill,
                                    position: desktop_app_contract::WallpaperPosition::Center,
                                    animation,
                                }
                            });
                            runtime.dispatch_action(DesktopAction::PreviewWallpaper { config });
                        }
                        Err(err) => logging::warn!("wallpaper import failed: {err}"),
                    }
                });
            }
            RuntimeEffect::UpdateWallpaperAssetMetadata { asset_id, patch } => {
                spawn_local(async move {
                    if let Err(err) =
                        platform_storage::wallpaper_update_asset_metadata(&asset_id, patch).await
                    {
                        logging::warn!("wallpaper metadata update failed: {err}");
                        return;
                    }
                    match platform_storage::wallpaper_list_library().await {
                        Ok(snapshot) => {
                            runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded {
                                snapshot,
                            });
                        }
                        Err(err) => logging::warn!("wallpaper library refresh failed: {err}"),
                    }
                });
            }
            RuntimeEffect::CreateWallpaperCollection { display_name } => {
                spawn_local(async move {
                    if let Err(err) =
                        platform_storage::wallpaper_create_collection(&display_name).await
                    {
                        logging::warn!("wallpaper collection create failed: {err}");
                        return;
                    }
                    match platform_storage::wallpaper_list_library().await {
                        Ok(snapshot) => {
                            runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded {
                                snapshot,
                            });
                        }
                        Err(err) => logging::warn!("wallpaper library refresh failed: {err}"),
                    }
                });
            }
            RuntimeEffect::RenameWallpaperCollection {
                collection_id,
                display_name,
            } => {
                spawn_local(async move {
                    if let Err(err) =
                        platform_storage::wallpaper_rename_collection(&collection_id, &display_name)
                            .await
                    {
                        logging::warn!("wallpaper collection rename failed: {err}");
                        return;
                    }
                    match platform_storage::wallpaper_list_library().await {
                        Ok(snapshot) => {
                            runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded {
                                snapshot,
                            });
                        }
                        Err(err) => logging::warn!("wallpaper library refresh failed: {err}"),
                    }
                });
            }
            RuntimeEffect::DeleteWallpaperCollection { collection_id } => {
                spawn_local(async move {
                    match platform_storage::wallpaper_delete_collection(&collection_id).await {
                        Ok(snapshot) => {
                            runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded {
                                snapshot,
                            });
                        }
                        Err(err) => logging::warn!("wallpaper collection delete failed: {err}"),
                    }
                });
            }
            RuntimeEffect::DeleteWallpaperAsset { asset_id } => {
                spawn_local(async move {
                    let desktop = runtime.state.get_untracked();
                    let current_matches = match &desktop.wallpaper.selection {
                        desktop_app_contract::WallpaperSelection::Imported {
                            asset_id: current_id,
                        } => current_id == &asset_id,
                        desktop_app_contract::WallpaperSelection::BuiltIn { .. } => false,
                    };
                    let preview_matches = desktop
                        .wallpaper_preview
                        .as_ref()
                        .map(|config| match &config.selection {
                            desktop_app_contract::WallpaperSelection::Imported {
                                asset_id: preview_id,
                            } => preview_id == &asset_id,
                            desktop_app_contract::WallpaperSelection::BuiltIn { .. } => false,
                        })
                        .unwrap_or(false);
                    if current_matches || preview_matches {
                        runtime.dispatch_action(DesktopAction::SetCurrentWallpaper {
                            config: wallpaper::builtin_wallpaper_by_id("cloud-bands")
                                .map(|_| desktop_app_contract::WallpaperConfig::default())
                                .unwrap_or_default(),
                        });
                    }
                    match platform_storage::wallpaper_delete_asset(&asset_id).await {
                        Ok(snapshot) => {
                            runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded {
                                snapshot,
                            });
                        }
                        Err(err) => logging::warn!("wallpaper asset delete failed: {err}"),
                    }
                });
            }
            RuntimeEffect::Notify { title, body } => {
                spawn_local(async move {
                    if let Err(err) = platform_storage::notify_send(&title, &body).await {
                        logging::warn!("notification dispatch failed: {err}");
                    }
                });
            }
        }
    }

    /// Handles a request to focus the active window's primary input.
    ///
    /// The reducer emits this intent when a window opens or is focused. Apps opt in by rendering
    /// [`desktop_app_contract::window_primary_input_dom_id`] on their primary text field.
    pub fn focus_window_input(self, window_id: crate::model::WindowId) {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(window) = web_sys::window() else {
                return;
            };
            let Some(document) = window.document() else {
                return;
            };
            let Some(element) =
                document.get_element_by_id(&window_primary_input_dom_id(window_id.0))
            else {
                return;
            };
            let Ok(element) = element.dyn_into::<web_sys::HtmlElement>() else {
                return;
            };
            // Defer focus until after the current effect/render turn so app blur/focus handlers do
            // not re-enter runtime updates while the shell is still processing reducer effects.
            let callback = Closure::once_into_js(move || {
                let _ = element.focus();
            });
            let _ = window
                .set_timeout_with_callback_and_timeout_and_arguments_0(callback.unchecked_ref(), 0);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = window_id;
    }

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
                    w: width.max(320),
                    h: (height - taskbar_height_px).max(220),
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
