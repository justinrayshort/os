//! Desktop runtime persistence adapters for boot hydration and lightweight local preferences.

use crate::model::{DesktopSnapshot, DesktopState, DesktopTheme};

#[cfg(target_arch = "wasm32")]
const SNAPSHOT_KEY: &str = "retrodesk.layout.v1";
#[cfg(target_arch = "wasm32")]
const THEME_KEY: &str = "retrodesk.theme.v1";
#[cfg(target_arch = "wasm32")]
const TERMINAL_HISTORY_KEY: &str = "retrodesk.terminal_history.v1";

/// Loads the legacy local-storage boot snapshot, theme override, and terminal history if present.
///
/// On non-WASM targets this returns `None`.
pub fn load_boot_snapshot() -> Option<DesktopSnapshot> {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = local_storage()?;
        let snapshot = storage
            .get_item(SNAPSHOT_KEY)
            .ok()
            .flatten()
            .and_then(|raw| serde_json::from_str::<DesktopSnapshot>(&raw).ok());

        let theme = storage
            .get_item(THEME_KEY)
            .ok()
            .flatten()
            .and_then(|raw| serde_json::from_str::<DesktopTheme>(&raw).ok());

        let terminal_history = storage
            .get_item(TERMINAL_HISTORY_KEY)
            .ok()
            .flatten()
            .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok());

        match (snapshot, theme, terminal_history) {
            (None, None, None) => None,
            (Some(mut snapshot), theme, history) => {
                if let Some(theme) = theme {
                    snapshot.theme = theme;
                }
                if let Some(history) = history {
                    snapshot.terminal_history = history;
                }
                Some(snapshot)
            }
            (None, Some(theme), history) => Some(DesktopSnapshot {
                schema_version: crate::model::DESKTOP_LAYOUT_SCHEMA_VERSION,
                theme,
                preferences: Default::default(),
                windows: Vec::new(),
                last_explorer_path: None,
                last_notepad_slug: None,
                terminal_history: history.unwrap_or_default(),
            }),
            (None, None, Some(history)) => Some(DesktopSnapshot {
                schema_version: crate::model::DESKTOP_LAYOUT_SCHEMA_VERSION,
                theme: Default::default(),
                preferences: Default::default(),
                windows: Vec::new(),
                last_explorer_path: None,
                last_notepad_slug: None,
                terminal_history: history,
            }),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

/// Loads the durable boot snapshot from [`platform_storage`] (IndexedDB-backed).
pub async fn load_durable_boot_snapshot() -> Option<DesktopSnapshot> {
    let envelope =
        match platform_storage::load_app_state_envelope(platform_storage::DESKTOP_STATE_NAMESPACE)
            .await
        {
            Ok(value) => value?,
            Err(err) => {
                leptos::logging::warn!("durable boot snapshot load failed: {err}");
                return None;
            }
        };

    serde_json::from_value::<DesktopSnapshot>(envelope.payload).ok()
}

/// Persists a durable desktop layout snapshot through [`platform_storage`].
pub async fn persist_durable_layout_snapshot(state: &DesktopState) -> Result<(), String> {
    let envelope = build_durable_layout_snapshot_envelope(state)?;
    persist_durable_layout_snapshot_envelope(&envelope).await
}

/// Builds a durable app-state envelope for the current desktop layout snapshot.
pub fn build_durable_layout_snapshot_envelope(
    state: &DesktopState,
) -> Result<platform_storage::AppStateEnvelope, String> {
    let snapshot = state.snapshot();
    platform_storage::build_app_state_envelope(
        platform_storage::DESKTOP_STATE_NAMESPACE,
        crate::model::DESKTOP_LAYOUT_SCHEMA_VERSION,
        &snapshot,
    )
}

/// Persists a prebuilt durable desktop layout envelope.
pub async fn persist_durable_layout_snapshot_envelope(
    envelope: &platform_storage::AppStateEnvelope,
) -> Result<(), String> {
    platform_storage::save_app_state_envelope(envelope).await
}

/// Persists compatibility layout state.
///
/// The current implementation keeps full layout persistence in [`platform_storage`] and reserves
/// localStorage for lightweight compatibility/prefs state.
pub fn persist_layout_snapshot(state: &DesktopState) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        // Full desktop layout is durably persisted in IndexedDB via `platform_storage`.
        // Keep localStorage reserved for lightweight compatibility/prefs paths.
        let _ = state;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = state;
    }

    Ok(())
}

/// Persists the desktop theme to localStorage on WASM targets.
pub fn persist_theme(theme: &DesktopTheme) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = local_storage().ok_or_else(|| "localStorage unavailable".to_string())?;
        let serialized = serde_json::to_string(theme).map_err(|e| e.to_string())?;
        storage
            .set_item(THEME_KEY, &serialized)
            .map_err(|e| format!("set theme failed: {e:?}"))?;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = theme;
    }

    Ok(())
}

/// Persists the terminal history list to localStorage on WASM targets.
pub fn persist_terminal_history(history: &[String]) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = local_storage().ok_or_else(|| "localStorage unavailable".to_string())?;
        let serialized = serde_json::to_string(history).map_err(|e| e.to_string())?;
        storage
            .set_item(TERMINAL_HISTORY_KEY, &serialized)
            .map_err(|e| format!("set terminal history failed: {e:?}"))?;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = history;
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}
