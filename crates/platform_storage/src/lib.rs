//! Cross-platform browser storage helpers for app state, cache data, and explorer filesystem APIs.
//!
//! This crate provides the persistence/reference layer used by the desktop apps and runtime. It
//! wraps browser capabilities (IndexedDB, Cache API, localStorage, and File System Access API)
//! behind Rust-friendly types and async functions.
//!
//! # Example
//!
//! ```rust
//! use platform_storage::{build_app_state_envelope, explorer_preview_cache_key, MemorySessionStore};
//!
//! let envelope = build_app_state_envelope("app.example", 1, &3_u32)
//!     .expect("envelope should serialize");
//! assert_eq!(envelope.namespace, "app.example");
//!
//! let key = explorer_preview_cache_key("/Documents/readme.txt");
//! assert_eq!(key, "file-preview:/Documents/readme.txt");
//!
//! let store = MemorySessionStore::default();
//! store.set("counter", &3_u32).expect("serialize");
//! assert_eq!(store.get::<u32>("counter"), Some(3));
//! ```

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

mod host_adapters;

use serde::{de::DeserializeOwned, Serialize};

pub use platform_host::{
    build_app_state_envelope, cache_get_json_with, cache_put_json_with, explorer_preview_cache_key,
    load_pref_with, migrate_envelope_payload, next_monotonic_timestamp_ms, normalize_virtual_path,
    save_pref_with, session_store, unix_time_ms_now, AppStateEnvelope, AppStateStore,
    AppStateStoreFuture, ContentCache, ContentCacheFuture, ExplorerBackend, ExplorerBackendStatus,
    ExplorerEntry, ExplorerEntryKind, ExplorerFileReadResult, ExplorerFsFuture, ExplorerFsService,
    ExplorerListResult, ExplorerMetadata, ExplorerPermissionMode, ExplorerPermissionState,
    ExplorerPrefs, MemoryAppStateStore, MemoryContentCache, MemoryPrefsStore, MemorySessionStore,
    NoopAppStateStore, NoopContentCache, NoopExplorerFsService, NoopPrefsStore, PrefsStore,
    PrefsStoreFuture, APP_STATE_ENVELOPE_VERSION, CALCULATOR_STATE_NAMESPACE,
    DESKTOP_STATE_NAMESPACE, EXPLORER_CACHE_NAME, EXPLORER_PREFS_KEY, EXPLORER_STATE_NAMESPACE,
    NOTEPAD_STATE_NAMESPACE, PAINT_STATE_NAMESPACE, TERMINAL_STATE_NAMESPACE,
};

/// Loads a typed preference value from localStorage on WASM targets.
///
/// Returns `None` when the key is absent, localStorage is unavailable, or deserialization fails.
pub fn load_local_pref<T: DeserializeOwned>(key: &str) -> Option<T> {
    host_adapters::prefs_store().load_typed(key)
}

/// Saves a typed preference value to localStorage on WASM targets.
///
/// # Errors
///
/// Returns an error when localStorage is unavailable or serialization/storage fails.
pub fn save_local_pref<T: Serialize>(key: &str, value: &T) -> Result<(), String> {
    host_adapters::prefs_store().save_typed(key, value)
}

/// Loads a persisted app-state envelope by namespace.
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn load_app_state_envelope(namespace: &str) -> Result<Option<AppStateEnvelope>, String> {
    let store = host_adapters::app_state_store();
    store.load_app_state_envelope(namespace).await
}

/// Saves a full app-state envelope.
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn save_app_state_envelope(envelope: &AppStateEnvelope) -> Result<(), String> {
    let store = host_adapters::app_state_store();
    store.save_app_state_envelope(envelope).await
}

/// Serializes and saves an app-state payload under `namespace`.
///
/// # Errors
///
/// Returns an error when payload serialization or storage fails.
pub async fn save_app_state<T: Serialize>(
    namespace: &str,
    schema_version: u32,
    payload: &T,
) -> Result<(), String> {
    let envelope = build_app_state_envelope(namespace, schema_version, payload)?;
    save_app_state_envelope(&envelope).await
}

/// Deletes persisted app state for a namespace.
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn delete_app_state(namespace: &str) -> Result<(), String> {
    let store = host_adapters::app_state_store();
    store.delete_app_state(namespace).await
}

/// Lists namespaces currently present in the app-state store.
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn list_app_state_namespaces() -> Result<Vec<String>, String> {
    let store = host_adapters::app_state_store();
    store.list_app_state_namespaces().await
}

/// Stores text content in the Cache API under `cache_name` and `key`.
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn cache_put_text(cache_name: &str, key: &str, value: &str) -> Result<(), String> {
    let cache = host_adapters::content_cache();
    cache.put_text(cache_name, key, value).await
}

/// Reads text content from the Cache API.
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn cache_get_text(cache_name: &str, key: &str) -> Result<Option<String>, String> {
    let cache = host_adapters::content_cache();
    cache.get_text(cache_name, key).await
}

/// Deletes a cached value from the Cache API.
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn cache_delete(cache_name: &str, key: &str) -> Result<(), String> {
    let cache = host_adapters::content_cache();
    cache.delete(cache_name, key).await
}

/// Serializes and stores a JSON value in the Cache API.
///
/// # Errors
///
/// Returns an error when serialization or cache storage fails.
pub async fn cache_put_json<T: Serialize>(
    cache_name: &str,
    key: &str,
    value: &T,
) -> Result<(), String> {
    let cache = host_adapters::content_cache();
    cache_put_json_with(&cache, cache_name, key, value).await
}

/// Reads and deserializes a JSON value from the Cache API.
///
/// # Errors
///
/// Returns an error when cache access or JSON deserialization fails.
pub async fn cache_get_json<T: DeserializeOwned>(
    cache_name: &str,
    key: &str,
) -> Result<Option<T>, String> {
    let cache = host_adapters::content_cache();
    cache_get_json_with(&cache, cache_name, key).await
}

/// Returns the current explorer backend status and capability information.
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn explorer_status() -> Result<ExplorerBackendStatus, String> {
    let fs = host_adapters::explorer_fs_service();
    fs.status().await
}

/// Opens the browser native-directory picker and returns updated backend status.
///
/// # Errors
///
/// Returns an error when the picker/bridge operation fails.
pub async fn explorer_pick_native_directory() -> Result<ExplorerBackendStatus, String> {
    let fs = host_adapters::explorer_fs_service();
    fs.pick_native_directory().await
}

/// Requests explorer permissions for the active backend.
///
/// # Errors
///
/// Returns an error when the permission request fails.
pub async fn explorer_request_permission(
    mode: ExplorerPermissionMode,
) -> Result<ExplorerPermissionState, String> {
    let fs = host_adapters::explorer_fs_service();
    fs.request_permission(mode).await
}

/// Lists a directory using the active explorer backend.
///
/// # Errors
///
/// Returns an error when the list operation fails.
pub async fn explorer_list_dir(path: &str) -> Result<ExplorerListResult, String> {
    let fs = host_adapters::explorer_fs_service();
    fs.list_dir(path).await
}

/// Reads a text file using the active explorer backend.
///
/// # Errors
///
/// Returns an error when the read operation fails.
pub async fn explorer_read_text_file(path: &str) -> Result<ExplorerFileReadResult, String> {
    let fs = host_adapters::explorer_fs_service();
    fs.read_text_file(path).await
}

/// Writes a text file using the active explorer backend.
///
/// # Errors
///
/// Returns an error when the write operation fails.
pub async fn explorer_write_text_file(path: &str, text: &str) -> Result<ExplorerMetadata, String> {
    let fs = host_adapters::explorer_fs_service();
    fs.write_text_file(path, text).await
}

/// Creates a directory using the active explorer backend.
///
/// # Errors
///
/// Returns an error when the create operation fails.
pub async fn explorer_create_dir(path: &str) -> Result<ExplorerMetadata, String> {
    let fs = host_adapters::explorer_fs_service();
    fs.create_dir(path).await
}

/// Creates a text file using the active explorer backend.
///
/// # Errors
///
/// Returns an error when the create operation fails.
pub async fn explorer_create_file(path: &str, text: &str) -> Result<ExplorerMetadata, String> {
    let fs = host_adapters::explorer_fs_service();
    fs.create_file(path, text).await
}

/// Deletes a file or directory using the active explorer backend.
///
/// When `recursive` is `true`, directory deletion may remove descendants.
///
/// # Errors
///
/// Returns an error when the delete operation fails.
pub async fn explorer_delete(path: &str, recursive: bool) -> Result<(), String> {
    let fs = host_adapters::explorer_fs_service();
    fs.delete(path, recursive).await
}

/// Retrieves metadata for a path using the active explorer backend.
///
/// # Errors
///
/// Returns an error when the stat operation fails.
pub async fn explorer_stat(path: &str) -> Result<ExplorerMetadata, String> {
    let fs = host_adapters::explorer_fs_service();
    fs.stat(path).await
}
