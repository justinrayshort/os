use platform_host::{
    AppStateEnvelope, AppStateStore, AppStateStoreFuture, ContentCache, ContentCacheFuture,
    ExplorerBackendStatus, ExplorerFileReadResult, ExplorerFsFuture, ExplorerFsService,
    ExplorerListResult, ExplorerMetadata, ExplorerPermissionMode, ExplorerPermissionState,
    NoopAppStateStore, NoopContentCache, NoopExplorerFsService, NoopPrefsStore, PrefsStore,
    PrefsStoreFuture,
};
use platform_host_web::{WebAppStateStore, WebContentCache, WebExplorerFsService, WebPrefsStore};
use serde::{de::DeserializeOwned, Serialize};

#[cfg_attr(not(any(test, feature = "desktop-host-stub")), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Compile-time selected host strategy for [`platform_storage`] adapters.
pub(crate) enum HostStrategy {
    /// Browser-backed adapters from `platform_host_web`.
    Browser,
    /// Desktop placeholder adapters used while native transport is being introduced.
    DesktopStub,
}

pub(crate) const fn selected_host_strategy() -> HostStrategy {
    #[cfg(feature = "desktop-host-stub")]
    {
        HostStrategy::DesktopStub
    }

    #[cfg(not(feature = "desktop-host-stub"))]
    {
        HostStrategy::Browser
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum AppStateStoreAdapter {
    Browser(WebAppStateStore),
    DesktopStub(NoopAppStateStore),
}

impl AppStateStore for AppStateStoreAdapter {
    fn load_app_state_envelope<'a>(
        &'a self,
        namespace: &'a str,
    ) -> AppStateStoreFuture<'a, Result<Option<AppStateEnvelope>, String>> {
        match self {
            Self::Browser(store) => store.load_app_state_envelope(namespace),
            Self::DesktopStub(store) => store.load_app_state_envelope(namespace),
        }
    }

    fn save_app_state_envelope<'a>(
        &'a self,
        envelope: &'a AppStateEnvelope,
    ) -> AppStateStoreFuture<'a, Result<(), String>> {
        match self {
            Self::Browser(store) => store.save_app_state_envelope(envelope),
            Self::DesktopStub(store) => store.save_app_state_envelope(envelope),
        }
    }

    fn delete_app_state<'a>(
        &'a self,
        namespace: &'a str,
    ) -> AppStateStoreFuture<'a, Result<(), String>> {
        match self {
            Self::Browser(store) => store.delete_app_state(namespace),
            Self::DesktopStub(store) => store.delete_app_state(namespace),
        }
    }

    fn list_app_state_namespaces<'a>(
        &'a self,
    ) -> AppStateStoreFuture<'a, Result<Vec<String>, String>> {
        match self {
            Self::Browser(store) => store.list_app_state_namespaces(),
            Self::DesktopStub(store) => store.list_app_state_namespaces(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ContentCacheAdapter {
    Browser(WebContentCache),
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
            Self::DesktopStub(store) => store.delete(cache_name, key),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ExplorerFsServiceAdapter {
    Browser(WebExplorerFsService),
    DesktopStub(NoopExplorerFsService),
}

impl ExplorerFsService for ExplorerFsServiceAdapter {
    fn status<'a>(&'a self) -> ExplorerFsFuture<'a, Result<ExplorerBackendStatus, String>> {
        match self {
            Self::Browser(store) => store.status(),
            Self::DesktopStub(store) => store.status(),
        }
    }

    fn pick_native_directory<'a>(
        &'a self,
    ) -> ExplorerFsFuture<'a, Result<ExplorerBackendStatus, String>> {
        match self {
            Self::Browser(store) => store.pick_native_directory(),
            Self::DesktopStub(store) => store.pick_native_directory(),
        }
    }

    fn request_permission<'a>(
        &'a self,
        mode: ExplorerPermissionMode,
    ) -> ExplorerFsFuture<'a, Result<ExplorerPermissionState, String>> {
        match self {
            Self::Browser(store) => store.request_permission(mode),
            Self::DesktopStub(store) => store.request_permission(mode),
        }
    }

    fn list_dir<'a>(
        &'a self,
        path: &'a str,
    ) -> ExplorerFsFuture<'a, Result<ExplorerListResult, String>> {
        match self {
            Self::Browser(store) => store.list_dir(path),
            Self::DesktopStub(store) => store.list_dir(path),
        }
    }

    fn read_text_file<'a>(
        &'a self,
        path: &'a str,
    ) -> ExplorerFsFuture<'a, Result<ExplorerFileReadResult, String>> {
        match self {
            Self::Browser(store) => store.read_text_file(path),
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
            Self::DesktopStub(store) => store.write_text_file(path, text),
        }
    }

    fn create_dir<'a>(
        &'a self,
        path: &'a str,
    ) -> ExplorerFsFuture<'a, Result<ExplorerMetadata, String>> {
        match self {
            Self::Browser(store) => store.create_dir(path),
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
            Self::DesktopStub(store) => store.delete(path, recursive),
        }
    }

    fn stat<'a>(&'a self, path: &'a str) -> ExplorerFsFuture<'a, Result<ExplorerMetadata, String>> {
        match self {
            Self::Browser(store) => store.stat(path),
            Self::DesktopStub(store) => store.stat(path),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PrefsStoreAdapter {
    Browser(WebPrefsStore),
    DesktopStub(NoopPrefsStore),
}

impl PrefsStoreAdapter {
    pub(crate) fn load_typed<T: DeserializeOwned>(self, key: &str) -> Option<T> {
        match self {
            Self::Browser(store) => store.load_typed(key),
            Self::DesktopStub(_) => None,
        }
    }

    pub(crate) fn save_typed<T: Serialize>(self, key: &str, value: &T) -> Result<(), String> {
        match self {
            Self::Browser(store) => store.save_typed(key, value),
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
            Self::DesktopStub(store) => store.save_pref(key, raw_json),
        }
    }

    fn delete_pref<'a>(&'a self, key: &'a str) -> PrefsStoreFuture<'a, Result<(), String>> {
        match self {
            Self::Browser(store) => store.delete_pref(key),
            Self::DesktopStub(store) => store.delete_pref(key),
        }
    }
}

fn app_state_store_for(strategy: HostStrategy) -> AppStateStoreAdapter {
    match strategy {
        HostStrategy::Browser => AppStateStoreAdapter::Browser(WebAppStateStore),
        HostStrategy::DesktopStub => AppStateStoreAdapter::DesktopStub(NoopAppStateStore),
    }
}

pub(crate) fn app_state_store() -> AppStateStoreAdapter {
    app_state_store_for(selected_host_strategy())
}

fn content_cache_for(strategy: HostStrategy) -> ContentCacheAdapter {
    match strategy {
        HostStrategy::Browser => ContentCacheAdapter::Browser(WebContentCache),
        HostStrategy::DesktopStub => ContentCacheAdapter::DesktopStub(NoopContentCache),
    }
}

pub(crate) fn content_cache() -> ContentCacheAdapter {
    content_cache_for(selected_host_strategy())
}

fn explorer_fs_service_for(strategy: HostStrategy) -> ExplorerFsServiceAdapter {
    match strategy {
        HostStrategy::Browser => ExplorerFsServiceAdapter::Browser(WebExplorerFsService),
        HostStrategy::DesktopStub => ExplorerFsServiceAdapter::DesktopStub(NoopExplorerFsService),
    }
}

pub(crate) fn explorer_fs_service() -> ExplorerFsServiceAdapter {
    explorer_fs_service_for(selected_host_strategy())
}

fn prefs_store_for(strategy: HostStrategy) -> PrefsStoreAdapter {
    match strategy {
        HostStrategy::Browser => PrefsStoreAdapter::Browser(WebPrefsStore),
        HostStrategy::DesktopStub => PrefsStoreAdapter::DesktopStub(NoopPrefsStore),
    }
}

pub(crate) fn prefs_store() -> PrefsStoreAdapter {
    prefs_store_for(selected_host_strategy())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_host_strategy_matches_build_feature() {
        #[cfg(feature = "desktop-host-stub")]
        assert_eq!(selected_host_strategy(), HostStrategy::DesktopStub);

        #[cfg(not(feature = "desktop-host-stub"))]
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
}
