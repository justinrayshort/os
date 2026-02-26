use platform_host_web::{WebAppStateStore, WebContentCache, WebExplorerFsService, WebPrefsStore};

pub(crate) fn app_state_store() -> WebAppStateStore {
    WebAppStateStore
}

pub(crate) fn content_cache() -> WebContentCache {
    WebContentCache
}

pub(crate) fn explorer_fs_service() -> WebExplorerFsService {
    WebExplorerFsService
}

pub(crate) fn prefs_store() -> WebPrefsStore {
    WebPrefsStore
}
