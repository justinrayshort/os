mod wasm_bridge;

#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};
use std::{cell::Cell, cell::RefCell, collections::HashMap, rc::Rc};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

pub const APP_STATE_ENVELOPE_VERSION: u32 = 1;
pub const DESKTOP_STATE_NAMESPACE: &str = "system.desktop";
pub const CALCULATOR_STATE_NAMESPACE: &str = "app.calculator";
pub const NOTEPAD_STATE_NAMESPACE: &str = "app.notepad";
pub const EXPLORER_STATE_NAMESPACE: &str = "app.explorer";
pub const TERMINAL_STATE_NAMESPACE: &str = "app.terminal";
pub const PAINT_STATE_NAMESPACE: &str = "app.paint";

pub const EXPLORER_CACHE_NAME: &str = "retrodesk-explorer-cache-v1";
pub const EXPLORER_PREFS_KEY: &str = "retrodesk.explorer.prefs.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppStateEnvelope {
    pub envelope_version: u32,
    pub namespace: String,
    pub schema_version: u32,
    pub updated_at_unix_ms: u64,
    pub payload: Value,
}

impl AppStateEnvelope {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExplorerBackend {
    NativeFsAccess,
    IndexedDbVirtual,
}

impl Default for ExplorerBackend {
    fn default() -> Self {
        Self::IndexedDbVirtual
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExplorerEntryKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExplorerPermissionState {
    Granted,
    Prompt,
    Denied,
    Unsupported,
    Virtual,
}

impl ExplorerPermissionState {
    pub fn can_read(self) -> bool {
        matches!(self, Self::Granted | Self::Virtual)
    }

    pub fn can_write(self) -> bool {
        matches!(self, Self::Granted | Self::Virtual)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExplorerPermissionMode {
    Read,
    Readwrite,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplorerBackendStatus {
    pub backend: ExplorerBackend,
    pub native_supported: bool,
    pub has_native_root: bool,
    pub permission: ExplorerPermissionState,
    pub root_path_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplorerEntry {
    pub name: String,
    pub path: String,
    pub kind: ExplorerEntryKind,
    pub size: Option<u64>,
    pub modified_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplorerMetadata {
    pub name: String,
    pub path: String,
    pub kind: ExplorerEntryKind,
    pub backend: ExplorerBackend,
    pub size: Option<u64>,
    pub modified_at_unix_ms: Option<u64>,
    pub permission: ExplorerPermissionState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplorerListResult {
    pub cwd: String,
    pub backend: ExplorerBackend,
    pub permission: ExplorerPermissionState,
    pub entries: Vec<ExplorerEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplorerFileReadResult {
    pub backend: ExplorerBackend,
    pub path: String,
    pub text: String,
    pub metadata: ExplorerMetadata,
    pub cached_preview_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplorerPrefs {
    pub preferred_backend: ExplorerBackend,
    pub details_visible: bool,
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
    pub fn set_json(&self, key: impl Into<String>, value: Value) {
        self.inner.borrow_mut().insert(key.into(), value);
    }

    pub fn get_json(&self, key: &str) -> Option<Value> {
        self.inner.borrow().get(key).cloned()
    }

    pub fn remove(&self, key: &str) {
        self.inner.borrow_mut().remove(key);
    }

    pub fn set<T: Serialize>(&self, key: impl Into<String>, value: &T) -> Result<(), String> {
        let json = serde_json::to_value(value).map_err(|e| e.to_string())?;
        self.set_json(key, json);
        Ok(())
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.get_json(key)
            .and_then(|value| serde_json::from_value(value).ok())
    }
}

thread_local! {
    static GLOBAL_SESSION_STORE: MemorySessionStore = MemorySessionStore::default();
    static LAST_ENVELOPE_TIMESTAMP_MS: Cell<u64> = const { Cell::new(0) };
}

pub fn session_store() -> MemorySessionStore {
    GLOBAL_SESSION_STORE.with(|store| store.clone())
}

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

pub fn next_monotonic_timestamp_ms() -> u64 {
    let now = unix_time_ms_now();
    LAST_ENVELOPE_TIMESTAMP_MS.with(|last| {
        let next = now.max(last.get().saturating_add(1));
        last.set(next);
        next
    })
}

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

pub async fn load_app_state_envelope(namespace: &str) -> Result<Option<AppStateEnvelope>, String> {
    wasm_bridge::load_app_state_envelope(namespace).await
}

pub async fn save_app_state_envelope(envelope: &AppStateEnvelope) -> Result<(), String> {
    wasm_bridge::save_app_state_envelope(envelope).await
}

pub async fn save_app_state<T: Serialize>(
    namespace: &str,
    schema_version: u32,
    payload: &T,
) -> Result<(), String> {
    let envelope = build_app_state_envelope(namespace, schema_version, payload)?;
    save_app_state_envelope(&envelope).await
}

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

pub async fn delete_app_state(namespace: &str) -> Result<(), String> {
    wasm_bridge::delete_app_state(namespace).await
}

pub async fn list_app_state_namespaces() -> Result<Vec<String>, String> {
    wasm_bridge::list_app_state_namespaces().await
}

pub async fn cache_put_text(cache_name: &str, key: &str, value: &str) -> Result<(), String> {
    wasm_bridge::cache_put_text(cache_name, key, value).await
}

pub async fn cache_get_text(cache_name: &str, key: &str) -> Result<Option<String>, String> {
    wasm_bridge::cache_get_text(cache_name, key).await
}

pub async fn cache_delete(cache_name: &str, key: &str) -> Result<(), String> {
    wasm_bridge::cache_delete(cache_name, key).await
}

pub async fn cache_put_json<T: Serialize>(
    cache_name: &str,
    key: &str,
    value: &T,
) -> Result<(), String> {
    let raw = serde_json::to_string(value).map_err(|e| e.to_string())?;
    cache_put_text(cache_name, key, &raw).await
}

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

pub async fn explorer_status() -> Result<ExplorerBackendStatus, String> {
    wasm_bridge::explorer_status().await
}

pub async fn explorer_pick_native_directory() -> Result<ExplorerBackendStatus, String> {
    wasm_bridge::explorer_pick_native_directory().await
}

pub async fn explorer_request_permission(
    mode: ExplorerPermissionMode,
) -> Result<ExplorerPermissionState, String> {
    wasm_bridge::explorer_request_permission(mode).await
}

pub async fn explorer_list_dir(path: &str) -> Result<ExplorerListResult, String> {
    wasm_bridge::explorer_list_dir(path).await
}

pub async fn explorer_read_text_file(path: &str) -> Result<ExplorerFileReadResult, String> {
    wasm_bridge::explorer_read_text_file(path).await
}

pub async fn explorer_write_text_file(path: &str, text: &str) -> Result<ExplorerMetadata, String> {
    wasm_bridge::explorer_write_text_file(path, text).await
}

pub async fn explorer_create_dir(path: &str) -> Result<ExplorerMetadata, String> {
    wasm_bridge::explorer_create_dir(path).await
}

pub async fn explorer_create_file(path: &str, text: &str) -> Result<ExplorerMetadata, String> {
    wasm_bridge::explorer_create_file(path, text).await
}

pub async fn explorer_delete(path: &str, recursive: bool) -> Result<(), String> {
    wasm_bridge::explorer_delete(path, recursive).await
}

pub async fn explorer_stat(path: &str) -> Result<ExplorerMetadata, String> {
    wasm_bridge::explorer_stat(path).await
}

pub fn explorer_preview_cache_key(path: &str) -> String {
    let normalized = if path.is_empty() { "/" } else { path };
    format!("file-preview:{}", normalized)
}

pub fn migrate_envelope_payload<T: DeserializeOwned>(
    envelope: &AppStateEnvelope,
) -> Result<T, String> {
    serde_json::from_value(envelope.payload.clone()).map_err(|e| e.to_string())
}
