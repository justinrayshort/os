//! Browser (`wasm32`) implementations of [`platform_host`] service contracts.
//!
//! This phase exports app-state, cache, prefs, and explorer/filesystem implementations while
//! `platform_storage` remains the temporary compatibility facade.
//!
//! Bridge bindings are split by domain under `bridge/`:
//! - `bridge::app_state`
//! - `bridge::cache`
//! - `bridge::fs`
//! - `bridge::prefs`
//! - `bridge::interop` (shared wasm/non-wasm transport glue)

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

mod bridge;
pub mod cache;
pub mod fs;
pub mod storage;

pub use cache::cache_api::WebContentCache;
pub use cache::tauri_cache_api::TauriContentCache;
pub use fs::explorer::{TauriExplorerFsService, WebExplorerFsService};
pub use storage::indexed_db::WebAppStateStore;
pub use storage::local_prefs::WebPrefsStore;
pub use storage::tauri_app_state::TauriAppStateStore;
pub use storage::tauri_prefs::TauriPrefsStore;
