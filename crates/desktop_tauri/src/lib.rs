//! Tauri desktop shell bootstrap for the shared runtime/app crates.
//!
//! This crate introduces the Stage 2 desktop host shell and keeps command registration localized
//! so future host-domain IPC handlers can be added without coupling the runtime layer directly to
//! Tauri internals.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

mod app_state;
mod cache;
#[doc(hidden)]
pub mod explorer;
mod external_url;
mod notifications;
mod prefs;

/// Starts the Tauri desktop host process.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            app_state::app_state_load,
            app_state::app_state_save,
            app_state::app_state_delete,
            app_state::app_state_namespaces,
            cache::cache_put_text,
            cache::cache_get_text,
            cache::cache_delete,
            explorer::explorer_status,
            explorer::explorer_pick_root,
            explorer::explorer_request_permission,
            explorer::explorer_list_dir,
            explorer::explorer_read_text_file,
            explorer::explorer_write_text_file,
            explorer::explorer_create_dir,
            explorer::explorer_create_file,
            explorer::explorer_delete,
            explorer::explorer_stat,
            external_url::external_open_url,
            notifications::notify_send,
            prefs::prefs_load,
            prefs::prefs_save,
            prefs::prefs_delete
        ])
        .run(tauri::generate_context!())
        .expect("desktop_tauri failed to run Tauri application");
}
