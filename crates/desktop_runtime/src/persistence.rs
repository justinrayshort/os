//! Desktop runtime persistence adapters for boot hydration and lightweight local preferences.

use crate::model::{DesktopSnapshot, DesktopState, DesktopTheme};

#[cfg(target_arch = "wasm32")]
const SNAPSHOT_KEY: &str = "retrodesk.layout.v1";
const THEME_KEY: &str = "retrodesk.theme.v1";
const TERMINAL_HISTORY_KEY: &str = "retrodesk.terminal_history.v1";

fn migrate_desktop_snapshot(
    schema_version: u32,
    envelope: &platform_storage::AppStateEnvelope,
) -> Result<Option<DesktopSnapshot>, String> {
    match schema_version {
        0 => platform_storage::migrate_envelope_payload(envelope).map(Some),
        _ => Ok(None),
    }
}

/// Loads the compatibility boot snapshot, theme override, and terminal history if present.
///
/// On non-WASM targets this returns `None`.
pub async fn load_boot_snapshot() -> Option<DesktopSnapshot> {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = local_storage()?;
        let snapshot = storage
            .get_item(SNAPSHOT_KEY)
            .ok()
            .flatten()
            .and_then(|raw| serde_json::from_str::<DesktopSnapshot>(&raw).ok());
        let theme = match platform_storage::load_pref_typed::<DesktopTheme>(THEME_KEY).await {
            Ok(theme) => theme,
            Err(err) => {
                leptos::logging::warn!("theme compatibility load failed: {err}");
                None
            }
        };

        let terminal_history =
            match platform_storage::load_pref_typed::<Vec<String>>(TERMINAL_HISTORY_KEY).await {
                Ok(history) => history,
                Err(err) => {
                    leptos::logging::warn!("terminal history compatibility load failed: {err}");
                    None
                }
            };

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
    match platform_storage::load_app_state_with_migration::<DesktopSnapshot, _>(
        platform_storage::DESKTOP_STATE_NAMESPACE,
        crate::model::DESKTOP_LAYOUT_SCHEMA_VERSION,
        migrate_desktop_snapshot,
    )
    .await
    {
        Ok(snapshot) => snapshot,
        Err(err) => {
            leptos::logging::warn!("durable boot snapshot load failed: {err}");
            None
        }
    }
}

/// Persists a durable desktop layout snapshot through [`platform_storage`].
pub async fn persist_durable_layout_snapshot(state: &DesktopState) -> Result<(), String> {
    let snapshot = state.snapshot();
    platform_storage::save_app_state(
        platform_storage::DESKTOP_STATE_NAMESPACE,
        crate::model::DESKTOP_LAYOUT_SCHEMA_VERSION,
        &snapshot,
    )
    .await
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

/// Persists the desktop theme through typed host prefs storage.
pub async fn persist_theme(theme: &DesktopTheme) -> Result<(), String> {
    platform_storage::save_pref_typed(THEME_KEY, theme).await
}

/// Persists the terminal history list through typed host prefs storage.
pub async fn persist_terminal_history(history: &[String]) -> Result<(), String> {
    platform_storage::save_pref_typed(TERMINAL_HISTORY_KEY, &history).await
}

#[cfg(target_arch = "wasm32")]
fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_namespace_migration_supports_schema_zero() {
        let snapshot = DesktopState::default().snapshot();
        let envelope = platform_storage::build_app_state_envelope(
            platform_storage::DESKTOP_STATE_NAMESPACE,
            0,
            &snapshot,
        )
        .expect("build envelope");

        let migrated =
            migrate_desktop_snapshot(0, &envelope).expect("schema-zero migration should succeed");
        assert!(migrated.is_some(), "expected migrated desktop snapshot");
    }
}
