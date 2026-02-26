//! Browser (`wasm32`) implementations of [`platform_host`] service contracts.
//!
//! This phase exports app-state, cache, prefs, and explorer/filesystem implementations while
//! `platform_storage` remains the temporary compatibility facade.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

mod bridge;
pub mod cache;
pub mod fs;
pub mod storage;

pub use cache::cache_api::WebContentCache;
pub use fs::explorer::WebExplorerFsService;
pub use storage::indexed_db::WebAppStateStore;
pub use storage::local_prefs::WebPrefsStore;
