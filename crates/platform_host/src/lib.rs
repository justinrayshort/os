//! Typed host-domain contracts and shared models used across runtime and browser adapters.
//!
//! This crate is the API-first boundary for platform services. Phase 1a exposes shared
//! persistence/explorer models, time/session helpers, and app-state service traits while existing
//! browser implementations remain in `platform_storage`.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

pub mod cache;
pub mod fs;
pub mod session;
pub mod storage;
pub mod time;

pub use cache::{
    cache_get_json_with, cache_put_json_with, ContentCache, ContentCacheFuture,
    MemoryContentCache, NoopContentCache,
};
pub use fs::path::normalize_virtual_path;
pub use fs::service::{ExplorerFsFuture, ExplorerFsService, NoopExplorerFsService};
pub use fs::types::{
    ExplorerBackend, ExplorerBackendStatus, ExplorerEntry, ExplorerEntryKind,
    ExplorerFileReadResult, ExplorerListResult, ExplorerMetadata, ExplorerPermissionMode,
    ExplorerPermissionState, ExplorerPrefs, EXPLORER_CACHE_NAME, EXPLORER_PREFS_KEY,
    explorer_preview_cache_key,
};
pub use session::{session_store, MemorySessionStore};
pub use storage::app_state::{
    build_app_state_envelope, migrate_envelope_payload, AppStateEnvelope, AppStateStore,
    AppStateStoreFuture, MemoryAppStateStore, NoopAppStateStore, APP_STATE_ENVELOPE_VERSION,
    CALCULATOR_STATE_NAMESPACE, DESKTOP_STATE_NAMESPACE, EXPLORER_STATE_NAMESPACE,
    NOTEPAD_STATE_NAMESPACE, PAINT_STATE_NAMESPACE, TERMINAL_STATE_NAMESPACE,
};
pub use storage::prefs::{
    load_pref_with, save_pref_with, MemoryPrefsStore, NoopPrefsStore, PrefsStore, PrefsStoreFuture,
};
pub use time::{next_monotonic_timestamp_ms, unix_time_ms_now};
