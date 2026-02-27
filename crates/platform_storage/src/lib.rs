//! Cross-platform browser storage helpers for app state, cache data, and explorer filesystem APIs.
//!
//! This crate provides the persistence/reference layer used by the desktop apps and runtime. It
//! wraps browser capabilities (IndexedDB, Cache API, localStorage, and File System Access API)
//! behind Rust-friendly types and async functions.
//!
//! Adapter selection is explicit through `host_adapters`: browser-backed services are the default,
//! desktop Tauri app-state/prefs/cache/explorer strategies are available for staged native command
//! integration, and a desktop stub strategy remains available for placeholder domains.
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

/// Loads and deserializes a typed preference value through the selected host adapter.
///
/// # Errors
///
/// Returns an error when host storage or JSON deserialization fails.
pub async fn load_pref_typed<T: DeserializeOwned>(key: &str) -> Result<Option<T>, String> {
    let store = host_adapters::prefs_store();
    load_pref_with(&store, key).await
}

/// Serializes and saves a typed preference value through the selected host adapter.
///
/// # Errors
///
/// Returns an error when serialization or host storage fails.
pub async fn save_pref_typed<T: Serialize>(key: &str, value: &T) -> Result<(), String> {
    let store = host_adapters::prefs_store();
    save_pref_with(&store, key, value).await
}

/// Deletes a preference key through the selected host adapter.
///
/// # Errors
///
/// Returns an error when host storage delete fails.
pub async fn delete_pref_typed(key: &str) -> Result<(), String> {
    let store = host_adapters::prefs_store();
    store.delete_pref(key).await
}

async fn load_app_state_envelope_raw(namespace: &str) -> Result<Option<AppStateEnvelope>, String> {
    let store = host_adapters::app_state_store();
    store.load_app_state_envelope(namespace).await
}

async fn save_app_state_envelope_raw(envelope: &AppStateEnvelope) -> Result<(), String> {
    let store = host_adapters::app_state_store();
    store.save_app_state_envelope(envelope).await
}

/// Loads a persisted app-state envelope by namespace.
///
/// This is a low-level compatibility API for storage/host boundary adapters and migration
/// internals. App/runtime callers should prefer [`load_app_state_typed`] or
/// [`load_app_state_with_migration`].
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn load_app_state_envelope_low_level(
    namespace: &str,
) -> Result<Option<AppStateEnvelope>, String> {
    load_app_state_envelope_raw(namespace).await
}

/// Saves a full app-state envelope.
///
/// This is a low-level compatibility API for storage/host boundary adapters and migration
/// internals. App/runtime callers should prefer [`save_app_state`].
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn save_app_state_envelope_low_level(envelope: &AppStateEnvelope) -> Result<(), String> {
    save_app_state_envelope_raw(envelope).await
}

/// Loads a persisted app-state envelope by namespace.
///
/// Deprecated compatibility alias for [`load_app_state_envelope_low_level`].
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
#[deprecated(
    since = "0.1.0",
    note = "Low-level envelope API. Use load_app_state_envelope_low_level at boundary adapters, or load_app_state_typed/load_app_state_with_migration in app/runtime callers."
)]
pub async fn load_app_state_envelope(namespace: &str) -> Result<Option<AppStateEnvelope>, String> {
    load_app_state_envelope_low_level(namespace).await
}

/// Saves a full app-state envelope.
///
/// Deprecated compatibility alias for [`save_app_state_envelope_low_level`].
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
#[deprecated(
    since = "0.1.0",
    note = "Low-level envelope API. Use save_app_state_envelope_low_level at boundary adapters, or save_app_state in app/runtime callers."
)]
pub async fn save_app_state_envelope(envelope: &AppStateEnvelope) -> Result<(), String> {
    save_app_state_envelope_low_level(envelope).await
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
    save_app_state_envelope_raw(&envelope).await
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Schema compatibility policy used by [`load_app_state_typed`].
pub enum AppStateSchemaPolicy {
    /// Accept only an exact schema version.
    Exact(u32),
    /// Accept any schema version up to and including this maximum.
    UpTo(u32),
    /// Accept any schema version.
    Any,
}

impl AppStateSchemaPolicy {
    const fn allows(self, schema_version: u32) -> bool {
        match self {
            Self::Exact(expected) => schema_version == expected,
            Self::UpTo(max_supported) => schema_version <= max_supported,
            Self::Any => true,
        }
    }
}

/// Loads and deserializes typed app-state data from an envelope namespace.
///
/// This helper enforces envelope-version compatibility and caller-selected schema policy before
/// deserializing payloads.
///
/// Returns `Ok(None)` when:
/// - the namespace is not present
/// - envelope metadata version is incompatible
/// - the persisted schema version does not satisfy `schema_policy`
///
/// # Errors
///
/// Returns an error when the underlying storage load fails or payload deserialization fails.
pub async fn load_app_state_typed<T: DeserializeOwned>(
    namespace: &str,
    schema_policy: AppStateSchemaPolicy,
) -> Result<Option<T>, String> {
    let Some(envelope) = load_app_state_envelope_raw(namespace).await? else {
        return Ok(None);
    };
    decode_typed_app_state_envelope(&envelope, schema_policy)
}

/// Loads typed app-state data while applying explicit legacy-schema migration hooks.
///
/// This is the preferred API for app/runtime hydration. It enforces envelope compatibility and
/// requires callers to handle legacy schemas intentionally instead of relying on broad
/// schema-policy acceptance.
///
/// Behavior:
/// - `schema == current_schema_version`: deserialize as current type
/// - `schema < current_schema_version`: call `migrate_legacy`
/// - `schema > current_schema_version`: return `Ok(None)`
///
/// # Errors
///
/// Returns an error when storage access fails, current-schema deserialization fails, or a caller
/// migration hook returns an error.
pub async fn load_app_state_with_migration<T, F>(
    namespace: &str,
    current_schema_version: u32,
    migrate_legacy: F,
) -> Result<Option<T>, String>
where
    T: DeserializeOwned,
    F: Fn(u32, &AppStateEnvelope) -> Result<Option<T>, String>,
{
    let Some(envelope) = load_app_state_envelope_raw(namespace).await? else {
        return Ok(None);
    };
    decode_typed_app_state_with_migration(&envelope, current_schema_version, migrate_legacy)
}

fn decode_typed_app_state_envelope<T: DeserializeOwned>(
    envelope: &AppStateEnvelope,
    schema_policy: AppStateSchemaPolicy,
) -> Result<Option<T>, String> {
    if envelope.envelope_version != APP_STATE_ENVELOPE_VERSION {
        return Ok(None);
    }
    if !schema_policy.allows(envelope.schema_version) {
        return Ok(None);
    }
    migrate_envelope_payload(envelope).map(Some)
}

fn decode_typed_app_state_with_migration<T, F>(
    envelope: &AppStateEnvelope,
    current_schema_version: u32,
    migrate_legacy: F,
) -> Result<Option<T>, String>
where
    T: DeserializeOwned,
    F: Fn(u32, &AppStateEnvelope) -> Result<Option<T>, String>,
{
    if envelope.envelope_version != APP_STATE_ENVELOPE_VERSION {
        return Ok(None);
    }

    if envelope.schema_version == current_schema_version {
        return migrate_envelope_payload(envelope).map(Some);
    }
    if envelope.schema_version > current_schema_version {
        return Ok(None);
    }
    migrate_legacy(envelope.schema_version, envelope)
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

#[cfg(test)]
mod tests {
    use super::{
        build_app_state_envelope, decode_typed_app_state_envelope,
        decode_typed_app_state_with_migration, AppStateSchemaPolicy,
    };

    #[test]
    fn app_state_schema_policy_matching_is_consistent() {
        assert!(AppStateSchemaPolicy::Exact(3).allows(3));
        assert!(!AppStateSchemaPolicy::Exact(3).allows(2));
        assert!(!AppStateSchemaPolicy::Exact(3).allows(4));

        assert!(AppStateSchemaPolicy::UpTo(3).allows(1));
        assert!(AppStateSchemaPolicy::UpTo(3).allows(3));
        assert!(!AppStateSchemaPolicy::UpTo(3).allows(4));

        assert!(AppStateSchemaPolicy::Any.allows(0));
        assert!(AppStateSchemaPolicy::Any.allows(42));
    }

    #[test]
    fn typed_envelope_decode_returns_none_for_incompatible_envelope_version() {
        let mut envelope =
            build_app_state_envelope("app.example", 1, &42_u32).expect("build envelope");
        envelope.envelope_version = super::APP_STATE_ENVELOPE_VERSION + 1;

        let decoded =
            decode_typed_app_state_envelope::<u32>(&envelope, AppStateSchemaPolicy::UpTo(1))
                .expect("decode should not fail");
        assert_eq!(decoded, None);
    }

    #[test]
    fn typed_envelope_decode_returns_none_for_incompatible_schema() {
        let envelope = build_app_state_envelope("app.example", 2, &42_u32).expect("build envelope");

        let decoded =
            decode_typed_app_state_envelope::<u32>(&envelope, AppStateSchemaPolicy::Exact(1))
                .expect("decode should not fail");
        assert_eq!(decoded, None);
    }

    #[test]
    fn typed_envelope_decode_deserializes_compatible_payload() {
        let envelope = build_app_state_envelope("app.example", 2, &42_u32).expect("build envelope");

        let decoded =
            decode_typed_app_state_envelope::<u32>(&envelope, AppStateSchemaPolicy::UpTo(2))
                .expect("decode should succeed");
        assert_eq!(decoded, Some(42));
    }

    #[test]
    fn typed_envelope_decode_surfaces_payload_type_errors() {
        let envelope = build_app_state_envelope("app.example", 1, &"text").expect("build envelope");

        let err = decode_typed_app_state_envelope::<u32>(&envelope, AppStateSchemaPolicy::Any)
            .expect_err("decode should fail for incompatible payload type");
        assert!(
            err.contains("invalid type"),
            "error should include type-mismatch context: {err}"
        );
    }

    #[test]
    fn migration_decode_uses_hook_for_legacy_schema() {
        let envelope = build_app_state_envelope("app.example", 0, &42_u32).expect("build envelope");

        let migrated =
            decode_typed_app_state_with_migration::<u32, _>(&envelope, 1, |legacy_schema, env| {
                assert_eq!(legacy_schema, 0);
                super::migrate_envelope_payload(env).map(Some)
            })
            .expect("migration decode should succeed");
        assert_eq!(migrated, Some(42));
    }

    #[test]
    fn migration_decode_rejects_future_schema() {
        let envelope = build_app_state_envelope("app.example", 2, &42_u32).expect("build envelope");

        let migrated =
            decode_typed_app_state_with_migration::<u32, _>(&envelope, 1, |_legacy, _| Ok(Some(0)))
                .expect("decode should not fail");
        assert_eq!(migrated, None);
    }

    #[test]
    fn migration_decode_uses_current_schema_deserialize() {
        let envelope = build_app_state_envelope("app.example", 1, &42_u32).expect("build envelope");

        let migrated =
            decode_typed_app_state_with_migration::<u32, _>(&envelope, 1, |_legacy, _| {
                panic!("legacy migration should not be called when schema is current");
            })
            .expect("decode should succeed");
        assert_eq!(migrated, Some(42));
    }
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
