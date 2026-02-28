use leptos::{logging, spawn_local, SignalGetUntracked};
use platform_host::WallpaperAssetService;
use platform_host_web::wallpaper_asset_service;

use crate::{components::DesktopRuntimeContext, reducer::DesktopAction, wallpaper};

pub(super) fn load_library(runtime: DesktopRuntimeContext) {
    spawn_local(async move {
        match wallpaper_asset_service().list_library().await {
            Ok(snapshot) => {
                runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded { snapshot });
            }
            Err(err) => logging::warn!("wallpaper library load failed: {err}"),
        }
    });
}

pub(super) fn import_from_picker(
    runtime: DesktopRuntimeContext,
    request: desktop_app_contract::WallpaperImportRequest,
) {
    spawn_local(async move {
        match wallpaper_asset_service().import_from_picker(request.clone()).await {
            Ok(asset) => {
                let snapshot = match wallpaper_asset_service().list_library().await {
                    Ok(snapshot) => snapshot,
                    Err(err) => {
                        logging::warn!("wallpaper library refresh failed after import: {err}");
                        return;
                    }
                };
                runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded { snapshot });
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

pub(super) fn update_asset_metadata(
    runtime: DesktopRuntimeContext,
    asset_id: String,
    patch: platform_host::WallpaperAssetMetadataPatch,
) {
    spawn_local(async move {
        if let Err(err) = wallpaper_asset_service()
            .update_asset_metadata(&asset_id, patch)
            .await
        {
            logging::warn!("wallpaper metadata update failed: {err}");
            return;
        }
        match wallpaper_asset_service().list_library().await {
            Ok(snapshot) => {
                runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded { snapshot });
            }
            Err(err) => logging::warn!("wallpaper library refresh failed: {err}"),
        }
    });
}

pub(super) fn create_collection(runtime: DesktopRuntimeContext, display_name: String) {
    spawn_local(async move {
        if let Err(err) = wallpaper_asset_service()
            .create_collection(&display_name)
            .await
        {
            logging::warn!("wallpaper collection create failed: {err}");
            return;
        }
        match wallpaper_asset_service().list_library().await {
            Ok(snapshot) => {
                runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded { snapshot });
            }
            Err(err) => logging::warn!("wallpaper library refresh failed: {err}"),
        }
    });
}

pub(super) fn rename_collection(
    runtime: DesktopRuntimeContext,
    collection_id: String,
    display_name: String,
) {
    spawn_local(async move {
        if let Err(err) = wallpaper_asset_service()
            .rename_collection(&collection_id, &display_name)
            .await
        {
            logging::warn!("wallpaper collection rename failed: {err}");
            return;
        }
        match wallpaper_asset_service().list_library().await {
            Ok(snapshot) => {
                runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded { snapshot });
            }
            Err(err) => logging::warn!("wallpaper library refresh failed: {err}"),
        }
    });
}

pub(super) fn delete_collection(runtime: DesktopRuntimeContext, collection_id: String) {
    spawn_local(async move {
        match wallpaper_asset_service().delete_collection(&collection_id).await {
            Ok(snapshot) => {
                runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded { snapshot });
            }
            Err(err) => logging::warn!("wallpaper collection delete failed: {err}"),
        }
    });
}

pub(super) fn delete_asset(runtime: DesktopRuntimeContext, asset_id: String) {
    spawn_local(async move {
        let desktop = runtime.state.get_untracked();
        let current_matches = match &desktop.wallpaper.selection {
            desktop_app_contract::WallpaperSelection::Imported { asset_id: current_id } => {
                current_id == &asset_id
            }
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
        match wallpaper_asset_service().delete_asset(&asset_id).await {
            Ok(snapshot) => {
                runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded { snapshot });
            }
            Err(err) => logging::warn!("wallpaper asset delete failed: {err}"),
        }
    });
}
