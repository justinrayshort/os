use leptos::{create_effect, logging, spawn_local, Callable, Callback};
use platform_host::WallpaperAssetService;
use platform_host_web::wallpaper_asset_service;

use crate::{persistence, reducer::DesktopAction};

pub(super) fn install_boot_hydration(dispatch: Callback<DesktopAction>) {
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
                if let Err(err) = persistence::persist_durable_layout_snapshot(&migrated_state).await
                {
                    logging::warn!("migrate legacy snapshot to durable store failed: {err}");
                }
            }

            match wallpaper_asset_service().list_library().await {
                Ok(snapshot) => {
                    dispatch.call(DesktopAction::WallpaperLibraryLoaded { snapshot });
                }
                Err(err) => logging::warn!("wallpaper library load failed: {err}"),
            }
        });
    });
}
