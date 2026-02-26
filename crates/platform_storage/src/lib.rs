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
//! let envelope = build_app_state_envelope("app.example", 1, &serde_json::json!({ "ok": true }))
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

mod wasm_bridge;

#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};
use std::{cell::Cell, cell::RefCell, collections::HashMap, rc::Rc};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

/// Version for [`AppStateEnvelope`] metadata serialization.
pub const APP_STATE_ENVELOPE_VERSION: u32 = 1;
/// Namespace used by the desktop runtime durable snapshot.
pub const DESKTOP_STATE_NAMESPACE: &str = "system.desktop";
/// Namespace used by the calculator app state.
pub const CALCULATOR_STATE_NAMESPACE: &str = "app.calculator";
/// Namespace used by the notepad app state.
pub const NOTEPAD_STATE_NAMESPACE: &str = "app.notepad";
/// Namespace used by the explorer app state.
pub const EXPLORER_STATE_NAMESPACE: &str = "app.explorer";
/// Namespace used by the terminal app state.
pub const TERMINAL_STATE_NAMESPACE: &str = "app.terminal";
/// Namespace used by the paint placeholder app state.
pub const PAINT_STATE_NAMESPACE: &str = "app.paint";

/// Cache API cache name used for explorer text previews.
pub const EXPLORER_CACHE_NAME: &str = "retrodesk-explorer-cache-v1";
/// localStorage key used for explorer UI preferences.
pub const EXPLORER_PREFS_KEY: &str = "retrodesk.explorer.prefs.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Versioned envelope for persisted app state payloads.
pub struct AppStateEnvelope {
    /// Envelope schema version.
    pub envelope_version: u32,
    /// Namespace identifying the owning app/domain.
    pub namespace: String,
    /// App-defined schema version for the payload.
    pub schema_version: u32,
    /// Last update time in unix milliseconds.
    pub updated_at_unix_ms: u64,
    /// Serialized app payload.
    pub payload: Value,
}

impl AppStateEnvelope {
    /// Creates a new envelope and stamps it with a monotonic timestamp.
    pub fn new(namespace: impl Into<String>, schema_version: u32, payload: Value) -> Self {
        Self {
            envelope_version: APP_STATE_ENVELOPE_VERSION,
            namespace: namespace.into(),
            schema_version,
            updated_at_unix_ms: next_monotonic_timestamp_ms(),
            payload,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
/// Explorer backend implementation currently serving requests.
pub enum ExplorerBackend {
    /// Browser native File System Access API.
    NativeFsAccess,
    /// IndexedDB-backed virtual filesystem implementation.
    #[default]
    IndexedDbVirtual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
/// Explorer directory entry kind.
pub enum ExplorerEntryKind {
    /// File entry.
    File,
    /// Directory entry.
    Directory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
/// Effective explorer permission state for a backend/path.
pub enum ExplorerPermissionState {
    /// Access is granted.
    Granted,
    /// Browser will prompt for permission.
    Prompt,
    /// Access is denied.
    Denied,
    /// Capability is unsupported in this browser context.
    Unsupported,
    /// Virtual filesystem backend (permission concept is synthetic and allowed).
    Virtual,
}

impl ExplorerPermissionState {
    /// Returns `true` when reads are allowed.
    pub fn can_read(self) -> bool {
        matches!(self, Self::Granted | Self::Virtual)
    }

    /// Returns `true` when writes are allowed.
    pub fn can_write(self) -> bool {
        matches!(self, Self::Granted | Self::Virtual)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
/// Permission mode requested from the explorer backend.
pub enum ExplorerPermissionMode {
    /// Read-only access.
    Read,
    /// Read/write access.
    Readwrite,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Current backend capability and permission status for the explorer app.
pub struct ExplorerBackendStatus {
    /// Active backend.
    pub backend: ExplorerBackend,
    /// Whether native File System Access is supported.
    pub native_supported: bool,
    /// Whether a native directory root is already connected.
    pub has_native_root: bool,
    /// Effective permission state.
    pub permission: ExplorerPermissionState,
    /// Optional user-facing root path hint.
    pub root_path_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Directory entry returned by explorer listing operations.
pub struct ExplorerEntry {
    /// Base name of the entry.
    pub name: String,
    /// Full normalized path.
    pub path: String,
    /// File or directory kind.
    pub kind: ExplorerEntryKind,
    /// File size in bytes (files only).
    pub size: Option<u64>,
    /// Last-modified time in unix milliseconds when available.
    pub modified_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Metadata describing a single explorer path.
pub struct ExplorerMetadata {
    /// Base name of the path.
    pub name: String,
    /// Full normalized path.
    pub path: String,
    /// File or directory kind.
    pub kind: ExplorerEntryKind,
    /// Backend that produced the metadata.
    pub backend: ExplorerBackend,
    /// File size in bytes (files only).
    pub size: Option<u64>,
    /// Last-modified time in unix milliseconds when available.
    pub modified_at_unix_ms: Option<u64>,
    /// Effective permission state.
    pub permission: ExplorerPermissionState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Result payload for directory listing operations.
pub struct ExplorerListResult {
    /// Normalized directory path that was listed.
    pub cwd: String,
    /// Backend that served the list request.
    pub backend: ExplorerBackend,
    /// Effective permission state for the listing.
    pub permission: ExplorerPermissionState,
    /// Child entries in the directory.
    pub entries: Vec<ExplorerEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Result payload for reading a text file in the explorer.
pub struct ExplorerFileReadResult {
    /// Backend that served the read request.
    pub backend: ExplorerBackend,
    /// Normalized file path.
    pub path: String,
    /// UTF-8 text content returned by the backend.
    pub text: String,
    /// File metadata snapshot captured at read time.
    pub metadata: ExplorerMetadata,
    /// Cache key suitable for storing/retrieving a preview copy.
    pub cached_preview_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// User preferences for the explorer app UI.
pub struct ExplorerPrefs {
    /// Preferred backend selection.
    pub preferred_backend: ExplorerBackend,
    /// Whether details columns/panels should be shown.
    pub details_visible: bool,
    /// Whether hidden files should be shown.
    pub show_hidden: bool,
}

impl Default for ExplorerPrefs {
    fn default() -> Self {
        Self {
            preferred_backend: ExplorerBackend::IndexedDbVirtual,
            details_visible: true,
            show_hidden: true,
        }
    }
}

#[derive(Debug, Clone)]
/// In-memory session-scoped key/value JSON store used by non-durable UI state.
pub struct MemorySessionStore {
    inner: Rc<RefCell<HashMap<String, Value>>>,
}

impl Default for MemorySessionStore {
    fn default() -> Self {
        Self {
            inner: Rc::new(RefCell::new(HashMap::new())),
        }
    }
}

impl MemorySessionStore {
    /// Stores a raw JSON value by key.
    pub fn set_json(&self, key: impl Into<String>, value: Value) {
        self.inner.borrow_mut().insert(key.into(), value);
    }

    /// Reads a raw JSON value by key.
    pub fn get_json(&self, key: &str) -> Option<Value> {
        self.inner.borrow().get(key).cloned()
    }

    /// Removes a value by key.
    pub fn remove(&self, key: &str) {
        self.inner.borrow_mut().remove(key);
    }

    /// Serializes and stores a typed value.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` cannot be serialized to JSON.
    pub fn set<T: Serialize>(&self, key: impl Into<String>, value: &T) -> Result<(), String> {
        let json = serde_json::to_value(value).map_err(|e| e.to_string())?;
        self.set_json(key, json);
        Ok(())
    }

    /// Reads and deserializes a typed value.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.get_json(key)
            .and_then(|value| serde_json::from_value(value).ok())
    }
}

thread_local! {
    static GLOBAL_SESSION_STORE: MemorySessionStore = MemorySessionStore::default();
    static LAST_ENVELOPE_TIMESTAMP_MS: Cell<u64> = const { Cell::new(0) };
}

/// Returns the process-local session store instance.
pub fn session_store() -> MemorySessionStore {
    GLOBAL_SESSION_STORE.with(|store| store.clone())
}

/// Returns the current unix timestamp in milliseconds.
pub fn unix_time_ms_now() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now().max(0.0) as u64
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

/// Returns a monotonic unix millisecond timestamp for envelope updates.
///
/// Values are monotonic within the current process even when the system clock does not advance.
pub fn next_monotonic_timestamp_ms() -> u64 {
    let now = unix_time_ms_now();
    LAST_ENVELOPE_TIMESTAMP_MS.with(|last| {
        let next = now.max(last.get().saturating_add(1));
        last.set(next);
        next
    })
}

/// Loads a typed preference value from localStorage on WASM targets.
///
/// Returns `None` when the key is absent, localStorage is unavailable, or deserialization fails.
pub fn load_local_pref<T: DeserializeOwned>(key: &str) -> Option<T> {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = web_sys::window()?.local_storage().ok().flatten()?;
        let raw = storage.get_item(key).ok().flatten()?;
        serde_json::from_str(&raw).ok()
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
        None
    }
}

/// Saves a typed preference value to localStorage on WASM targets.
///
/// # Errors
///
/// Returns an error when localStorage is unavailable or serialization/storage fails.
pub fn save_local_pref<T: Serialize>(key: &str, value: &T) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .ok_or_else(|| "localStorage unavailable".to_string())?;
        let raw = serde_json::to_string(value).map_err(|e| e.to_string())?;
        storage
            .set_item(key, &raw)
            .map_err(|e| format!("localStorage set_item failed: {e:?}"))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (key, value);
        Ok(())
    }
}

/// Loads a persisted app-state envelope by namespace.
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn load_app_state_envelope(namespace: &str) -> Result<Option<AppStateEnvelope>, String> {
    wasm_bridge::load_app_state_envelope(namespace).await
}

/// Saves a full app-state envelope.
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn save_app_state_envelope(envelope: &AppStateEnvelope) -> Result<(), String> {
    wasm_bridge::save_app_state_envelope(envelope).await
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

/// Builds a versioned [`AppStateEnvelope`] from a serializable payload.
///
/// # Errors
///
/// Returns an error when `payload` cannot be converted to JSON.
pub fn build_app_state_envelope<T: Serialize>(
    namespace: &str,
    schema_version: u32,
    payload: &T,
) -> Result<AppStateEnvelope, String> {
    let payload = serde_json::to_value(payload).map_err(|e| e.to_string())?;
    Ok(AppStateEnvelope::new(
        namespace.to_string(),
        schema_version,
        payload,
    ))
}

/// Deletes persisted app state for a namespace.
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn delete_app_state(namespace: &str) -> Result<(), String> {
    wasm_bridge::delete_app_state(namespace).await
}

/// Lists namespaces currently present in the app-state store.
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn list_app_state_namespaces() -> Result<Vec<String>, String> {
    wasm_bridge::list_app_state_namespaces().await
}

/// Stores text content in the Cache API under `cache_name` and `key`.
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn cache_put_text(cache_name: &str, key: &str, value: &str) -> Result<(), String> {
    wasm_bridge::cache_put_text(cache_name, key, value).await
}

/// Reads text content from the Cache API.
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn cache_get_text(cache_name: &str, key: &str) -> Result<Option<String>, String> {
    wasm_bridge::cache_get_text(cache_name, key).await
}

/// Deletes a cached value from the Cache API.
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn cache_delete(cache_name: &str, key: &str) -> Result<(), String> {
    wasm_bridge::cache_delete(cache_name, key).await
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
    let raw = serde_json::to_string(value).map_err(|e| e.to_string())?;
    cache_put_text(cache_name, key, &raw).await
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
    let Some(raw) = cache_get_text(cache_name, key).await? else {
        return Ok(None);
    };
    let decoded = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    Ok(Some(decoded))
}

/// Returns the current explorer backend status and capability information.
///
/// # Errors
///
/// Returns an error when the browser storage bridge fails.
pub async fn explorer_status() -> Result<ExplorerBackendStatus, String> {
    wasm_bridge::explorer_status().await
}

/// Opens the browser native-directory picker and returns updated backend status.
///
/// # Errors
///
/// Returns an error when the picker/bridge operation fails.
pub async fn explorer_pick_native_directory() -> Result<ExplorerBackendStatus, String> {
    wasm_bridge::explorer_pick_native_directory().await
}

/// Requests explorer permissions for the active backend.
///
/// # Errors
///
/// Returns an error when the permission request fails.
pub async fn explorer_request_permission(
    mode: ExplorerPermissionMode,
) -> Result<ExplorerPermissionState, String> {
    wasm_bridge::explorer_request_permission(mode).await
}

/// Lists a directory using the active explorer backend.
///
/// # Errors
///
/// Returns an error when the list operation fails.
pub async fn explorer_list_dir(path: &str) -> Result<ExplorerListResult, String> {
    wasm_bridge::explorer_list_dir(path).await
}

/// Reads a text file using the active explorer backend.
///
/// # Errors
///
/// Returns an error when the read operation fails.
pub async fn explorer_read_text_file(path: &str) -> Result<ExplorerFileReadResult, String> {
    wasm_bridge::explorer_read_text_file(path).await
}

/// Writes a text file using the active explorer backend.
///
/// # Errors
///
/// Returns an error when the write operation fails.
pub async fn explorer_write_text_file(path: &str, text: &str) -> Result<ExplorerMetadata, String> {
    wasm_bridge::explorer_write_text_file(path, text).await
}

/// Creates a directory using the active explorer backend.
///
/// # Errors
///
/// Returns an error when the create operation fails.
pub async fn explorer_create_dir(path: &str) -> Result<ExplorerMetadata, String> {
    wasm_bridge::explorer_create_dir(path).await
}

/// Creates a text file using the active explorer backend.
///
/// # Errors
///
/// Returns an error when the create operation fails.
pub async fn explorer_create_file(path: &str, text: &str) -> Result<ExplorerMetadata, String> {
    wasm_bridge::explorer_create_file(path, text).await
}

/// Deletes a file or directory using the active explorer backend.
///
/// When `recursive` is `true`, directory deletion may remove descendants.
///
/// # Errors
///
/// Returns an error when the delete operation fails.
pub async fn explorer_delete(path: &str, recursive: bool) -> Result<(), String> {
    wasm_bridge::explorer_delete(path, recursive).await
}

/// Retrieves metadata for a path using the active explorer backend.
///
/// # Errors
///
/// Returns an error when the stat operation fails.
pub async fn explorer_stat(path: &str) -> Result<ExplorerMetadata, String> {
    wasm_bridge::explorer_stat(path).await
}

/// Builds the Cache API key used for explorer file previews.
pub fn explorer_preview_cache_key(path: &str) -> String {
    let normalized = if path.is_empty() { "/" } else { path };
    format!("file-preview:{}", normalized)
}

/// Deserializes an envelope payload into a target type.
///
/// # Errors
///
/// Returns an error when deserialization fails.
pub fn migrate_envelope_payload<T: DeserializeOwned>(
    envelope: &AppStateEnvelope,
) -> Result<T, String> {
    serde_json::from_value(envelope.payload.clone()).map_err(|e| e.to_string())
}
