use desktop_app_contract::{
    ResolvedWallpaperSource, WallpaperAssetRecord, WallpaperCollection, WallpaperImportRequest,
    WallpaperLibrarySnapshot, WallpaperSelection,
};
use platform_host::{
    AppStateEnvelope, AppStateStore, AppStateStoreFuture, ContentCache, ContentCacheFuture,
    ExplorerBackendStatus, ExplorerFileReadResult, ExplorerFsFuture, ExplorerFsService,
    ExplorerListResult, ExplorerMetadata, ExplorerPermissionMode, ExplorerPermissionState,
    NoopAppStateStore, NoopContentCache, NoopExplorerFsService, NoopPrefsStore,
    NoopWallpaperAssetService, PrefsStore, PrefsStoreFuture, WallpaperAssetFuture,
    WallpaperAssetMetadataPatch, WallpaperAssetService,
};
use platform_host_web::{
    TauriAppStateStore, TauriContentCache, TauriExplorerFsService, TauriPrefsStore,
    WebAppStateStore, WebContentCache, WebExplorerFsService, WebPrefsStore,
    WebWallpaperAssetService,
};
use serde::{de::DeserializeOwned, Serialize};

#[cfg(all(feature = "desktop-host-stub", feature = "desktop-host-tauri"))]
compile_error!(
    "features `desktop-host-stub` and `desktop-host-tauri` are mutually exclusive; enable only one"
);

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Compile-time selected host strategy for [`platform_storage`] adapters.
pub enum HostStrategy {
    /// Browser-backed adapters from `platform_host_web`.
    Browser,
    /// Tauri desktop transport adapters for app-state, prefs, cache, and explorer domains.
    DesktopTauri,
    /// Desktop placeholder adapters used while native transport is being introduced.
    DesktopStub,
}

pub const fn selected_host_strategy() -> HostStrategy {
    #[cfg(feature = "desktop-host-tauri")]
    {
        HostStrategy::DesktopTauri
    }

    #[cfg(feature = "desktop-host-stub")]
    {
        HostStrategy::DesktopStub
    }

    #[cfg(not(any(feature = "desktop-host-stub", feature = "desktop-host-tauri")))]
    {
        HostStrategy::Browser
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum AppStateStoreAdapter {
    Browser(WebAppStateStore),
    DesktopTauri(TauriAppStateStore),
    DesktopStub(NoopAppStateStore),
}

impl AppStateStore for AppStateStoreAdapter {
    fn load_app_state_envelope<'a>(
        &'a self,
        namespace: &'a str,
    ) -> AppStateStoreFuture<'a, Result<Option<AppStateEnvelope>, String>> {
        match self {
            Self::Browser(store) => store.load_app_state_envelope(namespace),
            Self::DesktopTauri(store) => store.load_app_state_envelope(namespace),
            Self::DesktopStub(store) => store.load_app_state_envelope(namespace),
        }
    }

    fn save_app_state_envelope<'a>(
        &'a self,
        envelope: &'a AppStateEnvelope,
    ) -> AppStateStoreFuture<'a, Result<(), String>> {
        match self {
            Self::Browser(store) => store.save_app_state_envelope(envelope),
            Self::DesktopTauri(store) => store.save_app_state_envelope(envelope),
            Self::DesktopStub(store) => store.save_app_state_envelope(envelope),
        }
    }

    fn delete_app_state<'a>(
        &'a self,
        namespace: &'a str,
    ) -> AppStateStoreFuture<'a, Result<(), String>> {
        match self {
            Self::Browser(store) => store.delete_app_state(namespace),
            Self::DesktopTauri(store) => store.delete_app_state(namespace),
            Self::DesktopStub(store) => store.delete_app_state(namespace),
        }
    }

    fn list_app_state_namespaces<'a>(
        &'a self,
    ) -> AppStateStoreFuture<'a, Result<Vec<String>, String>> {
        match self {
            Self::Browser(store) => store.list_app_state_namespaces(),
            Self::DesktopTauri(store) => store.list_app_state_namespaces(),
            Self::DesktopStub(store) => store.list_app_state_namespaces(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ContentCacheAdapter {
    Browser(WebContentCache),
    DesktopTauri(TauriContentCache),
    DesktopStub(NoopContentCache),
}

impl ContentCache for ContentCacheAdapter {
    fn put_text<'a>(
        &'a self,
        cache_name: &'a str,
        key: &'a str,
        value: &'a str,
    ) -> ContentCacheFuture<'a, Result<(), String>> {
        match self {
            Self::Browser(store) => store.put_text(cache_name, key, value),
            Self::DesktopTauri(store) => store.put_text(cache_name, key, value),
            Self::DesktopStub(store) => store.put_text(cache_name, key, value),
        }
    }

    fn get_text<'a>(
        &'a self,
        cache_name: &'a str,
        key: &'a str,
    ) -> ContentCacheFuture<'a, Result<Option<String>, String>> {
        match self {
            Self::Browser(store) => store.get_text(cache_name, key),
            Self::DesktopTauri(store) => store.get_text(cache_name, key),
            Self::DesktopStub(store) => store.get_text(cache_name, key),
        }
    }

    fn delete<'a>(
        &'a self,
        cache_name: &'a str,
        key: &'a str,
    ) -> ContentCacheFuture<'a, Result<(), String>> {
        match self {
            Self::Browser(store) => store.delete(cache_name, key),
            Self::DesktopTauri(store) => store.delete(cache_name, key),
            Self::DesktopStub(store) => store.delete(cache_name, key),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ExplorerFsServiceAdapter {
    Browser(WebExplorerFsService),
    DesktopTauri(TauriExplorerFsService),
    DesktopStub(NoopExplorerFsService),
}

impl ExplorerFsService for ExplorerFsServiceAdapter {
    fn status<'a>(&'a self) -> ExplorerFsFuture<'a, Result<ExplorerBackendStatus, String>> {
        match self {
            Self::Browser(store) => store.status(),
            Self::DesktopTauri(store) => store.status(),
            Self::DesktopStub(store) => store.status(),
        }
    }

    fn pick_native_directory<'a>(
        &'a self,
    ) -> ExplorerFsFuture<'a, Result<ExplorerBackendStatus, String>> {
        match self {
            Self::Browser(store) => store.pick_native_directory(),
            Self::DesktopTauri(store) => store.pick_native_directory(),
            Self::DesktopStub(store) => store.pick_native_directory(),
        }
    }

    fn request_permission<'a>(
        &'a self,
        mode: ExplorerPermissionMode,
    ) -> ExplorerFsFuture<'a, Result<ExplorerPermissionState, String>> {
        match self {
            Self::Browser(store) => store.request_permission(mode),
            Self::DesktopTauri(store) => store.request_permission(mode),
            Self::DesktopStub(store) => store.request_permission(mode),
        }
    }

    fn list_dir<'a>(
        &'a self,
        path: &'a str,
    ) -> ExplorerFsFuture<'a, Result<ExplorerListResult, String>> {
        match self {
            Self::Browser(store) => store.list_dir(path),
            Self::DesktopTauri(store) => store.list_dir(path),
            Self::DesktopStub(store) => store.list_dir(path),
        }
    }

    fn read_text_file<'a>(
        &'a self,
        path: &'a str,
    ) -> ExplorerFsFuture<'a, Result<ExplorerFileReadResult, String>> {
        match self {
            Self::Browser(store) => store.read_text_file(path),
            Self::DesktopTauri(store) => store.read_text_file(path),
            Self::DesktopStub(store) => store.read_text_file(path),
        }
    }

    fn write_text_file<'a>(
        &'a self,
        path: &'a str,
        text: &'a str,
    ) -> ExplorerFsFuture<'a, Result<ExplorerMetadata, String>> {
        match self {
            Self::Browser(store) => store.write_text_file(path, text),
            Self::DesktopTauri(store) => store.write_text_file(path, text),
            Self::DesktopStub(store) => store.write_text_file(path, text),
        }
    }

    fn create_dir<'a>(
        &'a self,
        path: &'a str,
    ) -> ExplorerFsFuture<'a, Result<ExplorerMetadata, String>> {
        match self {
            Self::Browser(store) => store.create_dir(path),
            Self::DesktopTauri(store) => store.create_dir(path),
            Self::DesktopStub(store) => store.create_dir(path),
        }
    }

    fn create_file<'a>(
        &'a self,
        path: &'a str,
        text: &'a str,
    ) -> ExplorerFsFuture<'a, Result<ExplorerMetadata, String>> {
        match self {
            Self::Browser(store) => store.create_file(path, text),
            Self::DesktopTauri(store) => store.create_file(path, text),
            Self::DesktopStub(store) => store.create_file(path, text),
        }
    }

    fn delete<'a>(
        &'a self,
        path: &'a str,
        recursive: bool,
    ) -> ExplorerFsFuture<'a, Result<(), String>> {
        match self {
            Self::Browser(store) => store.delete(path, recursive),
            Self::DesktopTauri(store) => store.delete(path, recursive),
            Self::DesktopStub(store) => store.delete(path, recursive),
        }
    }

    fn stat<'a>(&'a self, path: &'a str) -> ExplorerFsFuture<'a, Result<ExplorerMetadata, String>> {
        match self {
            Self::Browser(store) => store.stat(path),
            Self::DesktopTauri(store) => store.stat(path),
            Self::DesktopStub(store) => store.stat(path),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PrefsStoreAdapter {
    Browser(WebPrefsStore),
    DesktopTauri(TauriPrefsStore),
    DesktopStub(NoopPrefsStore),
}

impl PrefsStoreAdapter {
    pub(crate) fn load_typed<T: DeserializeOwned>(self, key: &str) -> Option<T> {
        match self {
            Self::Browser(store) => store.load_typed(key),
            Self::DesktopTauri(_) => None,
            Self::DesktopStub(_) => None,
        }
    }

    pub(crate) fn save_typed<T: Serialize>(self, key: &str, value: &T) -> Result<(), String> {
        match self {
            Self::Browser(store) => store.save_typed(key, value),
            Self::DesktopTauri(_) => {
                let _ = (key, value);
                Ok(())
            }
            Self::DesktopStub(_) => {
                let _ = (key, value);
                Ok(())
            }
        }
    }
}

impl PrefsStore for PrefsStoreAdapter {
    fn load_pref<'a>(
        &'a self,
        key: &'a str,
    ) -> PrefsStoreFuture<'a, Result<Option<String>, String>> {
        match self {
            Self::Browser(store) => store.load_pref(key),
            Self::DesktopTauri(store) => store.load_pref(key),
            Self::DesktopStub(store) => store.load_pref(key),
        }
    }

    fn save_pref<'a>(
        &'a self,
        key: &'a str,
        raw_json: &'a str,
    ) -> PrefsStoreFuture<'a, Result<(), String>> {
        match self {
            Self::Browser(store) => store.save_pref(key, raw_json),
            Self::DesktopTauri(store) => store.save_pref(key, raw_json),
            Self::DesktopStub(store) => store.save_pref(key, raw_json),
        }
    }

    fn delete_pref<'a>(&'a self, key: &'a str) -> PrefsStoreFuture<'a, Result<(), String>> {
        match self {
            Self::Browser(store) => store.delete_pref(key),
            Self::DesktopTauri(store) => store.delete_pref(key),
            Self::DesktopStub(store) => store.delete_pref(key),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum WallpaperAssetServiceAdapter {
    Browser(WebWallpaperAssetService),
    DesktopTauri(WebWallpaperAssetService),
    DesktopStub(NoopWallpaperAssetService),
}

impl WallpaperAssetService for WallpaperAssetServiceAdapter {
    fn import_from_picker<'a>(
        &'a self,
        request: WallpaperImportRequest,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperAssetRecord, String>> {
        match self {
            Self::Browser(service) | Self::DesktopTauri(service) => {
                service.import_from_picker(request)
            }
            Self::DesktopStub(service) => service.import_from_picker(request),
        }
    }

    fn list_library<'a>(
        &'a self,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperLibrarySnapshot, String>> {
        match self {
            Self::Browser(service) | Self::DesktopTauri(service) => service.list_library(),
            Self::DesktopStub(service) => service.list_library(),
        }
    }

    fn update_asset_metadata<'a>(
        &'a self,
        asset_id: &'a str,
        patch: WallpaperAssetMetadataPatch,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperAssetRecord, String>> {
        match self {
            Self::Browser(service) | Self::DesktopTauri(service) => {
                service.update_asset_metadata(asset_id, patch)
            }
            Self::DesktopStub(service) => service.update_asset_metadata(asset_id, patch),
        }
    }

    fn create_collection<'a>(
        &'a self,
        display_name: &'a str,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperCollection, String>> {
        match self {
            Self::Browser(service) | Self::DesktopTauri(service) => {
                service.create_collection(display_name)
            }
            Self::DesktopStub(service) => service.create_collection(display_name),
        }
    }

    fn rename_collection<'a>(
        &'a self,
        collection_id: &'a str,
        display_name: &'a str,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperCollection, String>> {
        match self {
            Self::Browser(service) | Self::DesktopTauri(service) => {
                service.rename_collection(collection_id, display_name)
            }
            Self::DesktopStub(service) => service.rename_collection(collection_id, display_name),
        }
    }

    fn delete_collection<'a>(
        &'a self,
        collection_id: &'a str,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperLibrarySnapshot, String>> {
        match self {
            Self::Browser(service) | Self::DesktopTauri(service) => {
                service.delete_collection(collection_id)
            }
            Self::DesktopStub(service) => service.delete_collection(collection_id),
        }
    }

    fn delete_asset<'a>(
        &'a self,
        asset_id: &'a str,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperLibrarySnapshot, String>> {
        match self {
            Self::Browser(service) | Self::DesktopTauri(service) => service.delete_asset(asset_id),
            Self::DesktopStub(service) => service.delete_asset(asset_id),
        }
    }

    fn resolve_source<'a>(
        &'a self,
        selection: WallpaperSelection,
    ) -> WallpaperAssetFuture<'a, Result<Option<ResolvedWallpaperSource>, String>> {
        match self {
            Self::Browser(service) | Self::DesktopTauri(service) => {
                service.resolve_source(selection)
            }
            Self::DesktopStub(service) => service.resolve_source(selection),
        }
    }
}

fn app_state_store_for(strategy: HostStrategy) -> AppStateStoreAdapter {
    match strategy {
        HostStrategy::Browser => AppStateStoreAdapter::Browser(WebAppStateStore),
        HostStrategy::DesktopTauri => AppStateStoreAdapter::DesktopTauri(TauriAppStateStore),
        HostStrategy::DesktopStub => AppStateStoreAdapter::DesktopStub(NoopAppStateStore),
    }
}

pub(crate) fn app_state_store() -> AppStateStoreAdapter {
    app_state_store_for(selected_host_strategy())
}

fn content_cache_for(strategy: HostStrategy) -> ContentCacheAdapter {
    match strategy {
        HostStrategy::Browser => ContentCacheAdapter::Browser(WebContentCache),
        HostStrategy::DesktopTauri => ContentCacheAdapter::DesktopTauri(TauriContentCache),
        HostStrategy::DesktopStub => ContentCacheAdapter::DesktopStub(NoopContentCache),
    }
}

pub(crate) fn content_cache() -> ContentCacheAdapter {
    content_cache_for(selected_host_strategy())
}

fn explorer_fs_service_for(strategy: HostStrategy) -> ExplorerFsServiceAdapter {
    match strategy {
        HostStrategy::Browser => ExplorerFsServiceAdapter::Browser(WebExplorerFsService),
        HostStrategy::DesktopTauri => {
            ExplorerFsServiceAdapter::DesktopTauri(TauriExplorerFsService)
        }
        HostStrategy::DesktopStub => ExplorerFsServiceAdapter::DesktopStub(NoopExplorerFsService),
    }
}

pub(crate) fn explorer_fs_service() -> ExplorerFsServiceAdapter {
    explorer_fs_service_for(selected_host_strategy())
}

fn prefs_store_for(strategy: HostStrategy) -> PrefsStoreAdapter {
    match strategy {
        HostStrategy::Browser => PrefsStoreAdapter::Browser(WebPrefsStore),
        HostStrategy::DesktopTauri => PrefsStoreAdapter::DesktopTauri(TauriPrefsStore),
        HostStrategy::DesktopStub => PrefsStoreAdapter::DesktopStub(NoopPrefsStore),
    }
}

pub(crate) fn prefs_store() -> PrefsStoreAdapter {
    prefs_store_for(selected_host_strategy())
}

fn wallpaper_asset_service_for(strategy: HostStrategy) -> WallpaperAssetServiceAdapter {
    match strategy {
        HostStrategy::Browser => WallpaperAssetServiceAdapter::Browser(WebWallpaperAssetService),
        HostStrategy::DesktopTauri => {
            WallpaperAssetServiceAdapter::DesktopTauri(WebWallpaperAssetService)
        }
        HostStrategy::DesktopStub => {
            WallpaperAssetServiceAdapter::DesktopStub(NoopWallpaperAssetService)
        }
    }
}

pub(crate) fn wallpaper_asset_service() -> WallpaperAssetServiceAdapter {
    wallpaper_asset_service_for(selected_host_strategy())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_host_strategy_matches_build_feature() {
        #[cfg(feature = "desktop-host-tauri")]
        assert_eq!(selected_host_strategy(), HostStrategy::DesktopTauri);

        #[cfg(feature = "desktop-host-stub")]
        assert_eq!(selected_host_strategy(), HostStrategy::DesktopStub);

        #[cfg(not(any(feature = "desktop-host-stub", feature = "desktop-host-tauri")))]
        assert_eq!(selected_host_strategy(), HostStrategy::Browser);
    }

    #[test]
    fn adapters_follow_selected_strategy() {
        let strategy = selected_host_strategy();
        match strategy {
            HostStrategy::Browser => {
                assert!(matches!(
                    app_state_store(),
                    AppStateStoreAdapter::Browser(_)
                ));
                assert!(matches!(content_cache(), ContentCacheAdapter::Browser(_)));
                assert!(matches!(
                    explorer_fs_service(),
                    ExplorerFsServiceAdapter::Browser(_)
                ));
                assert!(matches!(prefs_store(), PrefsStoreAdapter::Browser(_)));
            }
            HostStrategy::DesktopTauri => {
                assert!(matches!(
                    app_state_store(),
                    AppStateStoreAdapter::DesktopTauri(_)
                ));
                assert!(matches!(
                    content_cache(),
                    ContentCacheAdapter::DesktopTauri(_)
                ));
                assert!(matches!(
                    explorer_fs_service(),
                    ExplorerFsServiceAdapter::DesktopTauri(_)
                ));
                assert!(matches!(prefs_store(), PrefsStoreAdapter::DesktopTauri(_)));
            }
            HostStrategy::DesktopStub => {
                assert!(matches!(
                    app_state_store(),
                    AppStateStoreAdapter::DesktopStub(_)
                ));
                assert!(matches!(
                    content_cache(),
                    ContentCacheAdapter::DesktopStub(_)
                ));
                assert!(matches!(
                    explorer_fs_service(),
                    ExplorerFsServiceAdapter::DesktopStub(_)
                ));
                assert!(matches!(prefs_store(), PrefsStoreAdapter::DesktopStub(_)));
            }
        }
    }

    #[test]
    fn desktop_stub_pref_adapter_noops_typed_calls() {
        let prefs = prefs_store_for(HostStrategy::DesktopStub);
        assert!(prefs.save_typed("example", &42_u32).is_ok());
        assert_eq!(prefs.load_typed::<u32>("example"), None);
    }

    #[test]
    fn desktop_tauri_pref_adapter_noops_typed_calls() {
        let prefs = prefs_store_for(HostStrategy::DesktopTauri);
        assert!(prefs.save_typed("example", &42_u32).is_ok());
        assert_eq!(prefs.load_typed::<u32>("example"), None);
    }
}
