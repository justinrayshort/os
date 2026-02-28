//! Desktop runtime persistence adapters for boot hydration and lightweight local preferences.

use crate::model::{DesktopSnapshot, DesktopState, DesktopTheme};
use desktop_app_contract::{WallpaperConfig, WallpaperSelection};
use platform_host::{
    load_app_state_with_migration, load_pref_with, migrate_envelope_payload,
    save_app_state_with, save_pref_with, AppStateEnvelope, DESKTOP_STATE_NAMESPACE,
};
#[cfg(test)]
use platform_host::build_app_state_envelope;
use platform_host_web::{app_state_store, prefs_store};
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
const SNAPSHOT_KEY: &str = "retrodesk.layout.v1";
const LEGACY_THEME_KEY: &str = "retrodesk.theme.v1";
const THEME_KEY: &str = "system.desktop_theme.v2";
const WALLPAPER_KEY: &str = "system.desktop_wallpaper.v1";
const TERMINAL_HISTORY_KEY: &str = "retrodesk.terminal_history.v1";
/// Persisted runtime policy overlay key for app capability grants.
pub const APP_POLICY_KEY: &str = "system.app_policy.v1";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Persisted capability policy overlay keyed by app id.
pub struct AppPolicyOverlay {
    /// App ids treated as privileged by shell policy.
    pub privileged_app_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LegacyThemePayload {
    #[serde(default)]
    skin: crate::model::DesktopSkin,
    wallpaper_id: String,
    high_contrast: bool,
    reduced_motion: bool,
    audio_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct LegacyDesktopSnapshotV1 {
    schema_version: u32,
    theme: LegacyThemePayload,
    preferences: crate::model::DesktopPreferences,
    windows: Vec<crate::model::WindowRecord>,
    last_explorer_path: Option<String>,
    last_notepad_slug: Option<String>,
    terminal_history: Vec<String>,
    #[serde(default)]
    app_shared_state: std::collections::BTreeMap<String, serde_json::Value>,
}

fn migrate_desktop_snapshot(
    schema_version: u32,
    envelope: &AppStateEnvelope,
) -> Result<Option<DesktopSnapshot>, String> {
    match schema_version {
        0 => migrate_envelope_payload(envelope).map(Some),
        1 => {
            let legacy = migrate_envelope_payload::<LegacyDesktopSnapshotV1>(envelope)?;
            Ok(Some(DesktopSnapshot {
                schema_version: crate::model::DESKTOP_LAYOUT_SCHEMA_VERSION,
                preferences: legacy.preferences,
                windows: legacy.windows,
                last_explorer_path: legacy.last_explorer_path,
                last_notepad_slug: legacy.last_notepad_slug,
                terminal_history: legacy.terminal_history,
                app_shared_state: legacy.app_shared_state,
            }))
        }
        _ => Ok(None),
    }
}

/// Loads the compatibility boot snapshot and terminal history if present.
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
        let terminal_history = match load_pref_with(&prefs_store(), TERMINAL_HISTORY_KEY).await {
                Ok(history) => history,
                Err(err) => {
                    leptos::logging::warn!("terminal history compatibility load failed: {err}");
                    None
                }
            };

        match (snapshot, terminal_history) {
            (None, None) => None,
            (Some(mut snapshot), history) => {
                if let Some(history) = history {
                    snapshot.terminal_history = history;
                }
                Some(snapshot)
            }
            (None, Some(history)) => Some(DesktopSnapshot {
                schema_version: crate::model::DESKTOP_LAYOUT_SCHEMA_VERSION,
                preferences: Default::default(),
                windows: Vec::new(),
                last_explorer_path: None,
                last_notepad_slug: None,
                terminal_history: history,
                app_shared_state: Default::default(),
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
    let store = app_state_store();
    match load_app_state_with_migration(
        &store,
        DESKTOP_STATE_NAMESPACE,
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
    let store = app_state_store();
    save_app_state_with(
        &store,
        DESKTOP_STATE_NAMESPACE,
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
    save_pref_with(&prefs_store(), THEME_KEY, theme).await
}

/// Loads the current desktop theme from typed prefs, falling back to the legacy combined payload.
pub async fn load_theme() -> Option<DesktopTheme> {
    match load_pref_with(&prefs_store(), THEME_KEY).await {
        Ok(Some(theme)) => Some(theme),
        Ok(None) => load_legacy_theme().await.map(|legacy| DesktopTheme {
            skin: legacy.skin,
            high_contrast: legacy.high_contrast,
            reduced_motion: legacy.reduced_motion,
            audio_enabled: legacy.audio_enabled,
        }),
        Err(err) => {
            leptos::logging::warn!("desktop theme load failed: {err}");
            None
        }
    }
}

/// Persists the current wallpaper configuration through typed host prefs storage.
pub async fn persist_wallpaper(wallpaper: &WallpaperConfig) -> Result<(), String> {
    save_pref_with(&prefs_store(), WALLPAPER_KEY, wallpaper).await
}

/// Loads the current wallpaper configuration from typed prefs, falling back to the legacy theme payload.
pub async fn load_wallpaper() -> Option<WallpaperConfig> {
    match load_pref_with(&prefs_store(), WALLPAPER_KEY).await {
        Ok(Some(wallpaper)) => Some(normalize_wallpaper(wallpaper)),
        Ok(None) => load_legacy_theme().await.map(|legacy| WallpaperConfig {
            selection: WallpaperSelection::BuiltIn {
                wallpaper_id: normalize_legacy_wallpaper_id(&legacy.wallpaper_id),
            },
            ..WallpaperConfig::default()
        }),
        Err(err) => {
            leptos::logging::warn!("desktop wallpaper load failed: {err}");
            None
        }
    }
}

fn normalize_legacy_wallpaper_id(raw: &str) -> String {
    match raw.trim() {
        "slate-grid" => "teal-grid".to_string(),
        "" => "cloud-bands".to_string(),
        other => other.to_string(),
    }
}

fn normalize_wallpaper(mut wallpaper: WallpaperConfig) -> WallpaperConfig {
    if let WallpaperSelection::BuiltIn { wallpaper_id } = &mut wallpaper.selection {
        *wallpaper_id = normalize_legacy_wallpaper_id(wallpaper_id);
    }
    wallpaper
}

async fn load_legacy_theme() -> Option<LegacyThemePayload> {
    match load_pref_with(&prefs_store(), LEGACY_THEME_KEY).await {
        Ok(value) => value,
        Err(err) => {
            leptos::logging::warn!("legacy theme compatibility load failed: {err}");
            None
        }
    }
}

/// Persists the terminal history list through typed host prefs storage.
pub async fn persist_terminal_history(history: &[String]) -> Result<(), String> {
    save_pref_with(&prefs_store(), TERMINAL_HISTORY_KEY, &history).await
}

/// Loads app capability policy overlay from typed host prefs storage.
pub async fn load_app_policy_overlay() -> Option<AppPolicyOverlay> {
    match load_pref_with(&prefs_store(), APP_POLICY_KEY).await {
        Ok(value) => value,
        Err(err) => {
            leptos::logging::warn!("app policy overlay load failed: {err}");
            None
        }
    }
}

/// Persists app capability policy overlay through typed host prefs storage.
pub async fn persist_app_policy_overlay(policy: &AppPolicyOverlay) -> Result<(), String> {
    save_pref_with(&prefs_store(), APP_POLICY_KEY, policy).await
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
        let envelope = build_app_state_envelope(
            DESKTOP_STATE_NAMESPACE,
            0,
            &snapshot,
        )
        .expect("build envelope");

        let migrated =
            migrate_desktop_snapshot(0, &envelope).expect("schema-zero migration should succeed");
        assert!(migrated.is_some(), "expected migrated desktop snapshot");
    }
}
