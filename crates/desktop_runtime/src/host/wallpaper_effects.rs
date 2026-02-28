use leptos::{logging, spawn_local, SignalGetUntracked};
use platform_host::{
    WallpaperAnimationPolicy, WallpaperConfig, WallpaperDisplayMode, WallpaperImportRequest,
    WallpaperMediaKind, WallpaperPosition, WallpaperSelection,
};

use crate::{
    components::DesktopRuntimeContext, host::DesktopHostContext, reducer::DesktopAction, wallpaper,
};

pub(super) fn load_library(host: DesktopHostContext, runtime: DesktopRuntimeContext) {
    spawn_local(async move {
        match host.wallpaper_asset_service().list_library().await {
            Ok(snapshot) => {
                runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded { snapshot });
            }
            Err(err) => logging::warn!("wallpaper library load failed: {err}"),
        }
    });
}

pub(super) fn import_from_picker(
    host: DesktopHostContext,
    runtime: DesktopRuntimeContext,
    request: WallpaperImportRequest,
) {
    spawn_local(async move {
        let wallpaper = host.wallpaper_asset_service();
        match wallpaper.import_from_picker(request.clone()).await {
            Ok(asset) => {
                let snapshot = match wallpaper.list_library().await {
                    Ok(snapshot) => snapshot,
                    Err(err) => {
                        logging::warn!("wallpaper library refresh failed after import: {err}");
                        return;
                    }
                };
                runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded { snapshot });
                let config = request.default_config.unwrap_or_else(|| {
                    let animation = match asset.media_kind {
                        WallpaperMediaKind::AnimatedImage | WallpaperMediaKind::Video => {
                            WallpaperAnimationPolicy::LoopMuted
                        }
                        _ => WallpaperAnimationPolicy::None,
                    };
                    WallpaperConfig {
                        selection: WallpaperSelection::Imported {
                            asset_id: asset.asset_id,
                        },
                        display_mode: WallpaperDisplayMode::Fill,
                        position: WallpaperPosition::Center,
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
    host: DesktopHostContext,
    runtime: DesktopRuntimeContext,
    asset_id: String,
    patch: platform_host::WallpaperAssetMetadataPatch,
) {
    spawn_local(async move {
        let wallpaper = host.wallpaper_asset_service();
        match wallpaper.update_asset_metadata(&asset_id, patch).await {
            Ok(asset) => {
                runtime.dispatch_action(DesktopAction::WallpaperAssetUpdated { asset });
            }
            Err(err) => logging::warn!("wallpaper metadata update failed: {err}"),
        }
    });
}

pub(super) fn create_collection(
    host: DesktopHostContext,
    runtime: DesktopRuntimeContext,
    display_name: String,
) {
    spawn_local(async move {
        let wallpaper = host.wallpaper_asset_service();
        match wallpaper.create_collection(&display_name).await {
            Ok(collection) => {
                runtime.dispatch_action(DesktopAction::WallpaperCollectionUpdated { collection });
            }
            Err(err) => logging::warn!("wallpaper collection create failed: {err}"),
        }
    });
}

pub(super) fn rename_collection(
    host: DesktopHostContext,
    runtime: DesktopRuntimeContext,
    collection_id: String,
    display_name: String,
) {
    spawn_local(async move {
        let wallpaper = host.wallpaper_asset_service();
        match wallpaper
            .rename_collection(&collection_id, &display_name)
            .await
        {
            Ok(collection) => {
                runtime.dispatch_action(DesktopAction::WallpaperCollectionUpdated { collection });
            }
            Err(err) => logging::warn!("wallpaper collection rename failed: {err}"),
        }
    });
}

pub(super) fn delete_collection(
    host: DesktopHostContext,
    runtime: DesktopRuntimeContext,
    collection_id: String,
) {
    spawn_local(async move {
        match host
            .wallpaper_asset_service()
            .delete_collection(&collection_id)
            .await
        {
            Ok(snapshot) => {
                runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded { snapshot });
            }
            Err(err) => logging::warn!("wallpaper collection delete failed: {err}"),
        }
    });
}

pub(super) fn delete_asset(
    host: DesktopHostContext,
    runtime: DesktopRuntimeContext,
    asset_id: String,
) {
    spawn_local(async move {
        let desktop = runtime.state.get_untracked();
        let current_matches = match &desktop.wallpaper.selection {
            WallpaperSelection::Imported {
                asset_id: current_id,
            } => current_id == &asset_id,
            WallpaperSelection::BuiltIn { .. } => false,
        };
        let preview_matches = desktop
            .wallpaper_preview
            .as_ref()
            .map(|config| match &config.selection {
                WallpaperSelection::Imported {
                    asset_id: preview_id,
                } => preview_id == &asset_id,
                WallpaperSelection::BuiltIn { .. } => false,
            })
            .unwrap_or(false);
        if current_matches || preview_matches {
            runtime.dispatch_action(DesktopAction::SetCurrentWallpaper {
                config: wallpaper::builtin_wallpaper_by_id("cloud-bands")
                    .map(|_| WallpaperConfig::default())
                    .unwrap_or_default(),
            });
        }
        match host.wallpaper_asset_service().delete_asset(&asset_id).await {
            Ok(snapshot) => {
                runtime.dispatch_action(DesktopAction::WallpaperLibraryLoaded { snapshot });
            }
            Err(err) => logging::warn!("wallpaper asset delete failed: {err}"),
        }
    });
}
