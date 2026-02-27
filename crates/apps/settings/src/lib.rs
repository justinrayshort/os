//! Built-in System Settings desktop app for display/theme preferences.
//!
//! The app exposes desktop wallpaper, shell skin, and accessibility toggles through
//! [`desktop_app_contract::AppServices`] so settings can be changed from a standard
//! managed app window instead of a shell modal.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

use desktop_app_contract::AppServices;
use leptos::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum SettingsTab {
    Display,
    Accessibility,
}

impl SettingsTab {
    fn label(self) -> &'static str {
        match self {
            Self::Display => "Display",
            Self::Accessibility => "Accessibility",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SettingsAppState {
    active_tab: SettingsTab,
}

impl Default for SettingsAppState {
    fn default() -> Self {
        Self {
            active_tab: SettingsTab::Display,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WallpaperPreset {
    id: &'static str,
    label: &'static str,
    note: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SkinPreset {
    id: &'static str,
    label: &'static str,
    note: &'static str,
}

const WALLPAPER_PRESETS: [WallpaperPreset; 6] = [
    WallpaperPreset {
        id: "teal-solid",
        label: "Solid Teal",
        note: "Classic single-color desktop fill",
    },
    WallpaperPreset {
        id: "teal-grid",
        label: "Teal Grid",
        note: "Subtle tiled grid on teal",
    },
    WallpaperPreset {
        id: "woven-steel",
        label: "Woven Steel",
        note: "Crosshatch weave pattern",
    },
    WallpaperPreset {
        id: "cloud-bands",
        label: "Cloud Bands",
        note: "Soft sky bands and clouds",
    },
    WallpaperPreset {
        id: "green-hills",
        label: "Green Hills",
        note: "Rolling hills and blue sky",
    },
    WallpaperPreset {
        id: "sunset-lake",
        label: "Sunset Lake",
        note: "Warm dusk landscape scene",
    },
];

const SKIN_PRESETS: [SkinPreset; 3] = [
    SkinPreset {
        id: "modern-adaptive",
        label: "Modern Adaptive",
        note: "Dark-first modern skin with adaptive mapping",
    },
    SkinPreset {
        id: "classic-xp",
        label: "Classic XP",
        note: "Nostalgic XP-inspired shell palette and controls",
    },
    SkinPreset {
        id: "classic-95",
        label: "Classic 95",
        note: "Nostalgic Windows 95-inspired shell palette and controls",
    },
];

fn launch_bool(launch_params: &Value, key: &str, default: bool) -> bool {
    launch_params
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

fn launch_string(launch_params: &Value, key: &str, default: &str) -> String {
    launch_params
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}

#[component]
/// Settings app window contents.
///
/// This view prioritizes touch-sized targets and immediate feedback: selecting a value sends a
/// command through the manager host, and state is persisted as manager-owned app state.
pub fn SettingsApp(
    /// Launch parameters with optional initial theme values.
    launch_params: Value,
    /// Manager-restored app state payload.
    restored_state: Option<Value>,
    /// Optional app-host bridge for manager-owned commands.
    services: Option<AppServices>,
) -> impl IntoView {
    let settings_state = create_rw_signal(SettingsAppState::default());
    let wallpaper_id =
        create_rw_signal(launch_string(&launch_params, "wallpaper_id", "teal-solid"));
    let skin_id = create_rw_signal(launch_string(&launch_params, "skin_id", "modern-adaptive"));
    let high_contrast = create_rw_signal(launch_bool(&launch_params, "high_contrast", false));
    let reduced_motion = create_rw_signal(launch_bool(&launch_params, "reduced_motion", false));

    if let Some(restored_state) = restored_state {
        if let Ok(restored) = serde_json::from_value::<SettingsAppState>(restored_state) {
            settings_state.set(restored);
        }
    }

    if let Some(services) = services {
        create_effect(move |_| {
            if let Ok(serialized) = serde_json::to_value(settings_state.get()) {
                services.state.persist_window_state(serialized);
            }
        });
    }

    let set_active_tab = move |tab: SettingsTab| {
        settings_state.update(|settings| settings.active_tab = tab);
    };

    let select_wallpaper = move |selected_wallpaper_id: &'static str| {
        wallpaper_id.set(selected_wallpaper_id.to_string());
        if let Some(services) = services {
            services.theme.set_wallpaper(selected_wallpaper_id);
        }
    };

    let select_skin = move |selected_skin_id: &'static str| {
        skin_id.set(selected_skin_id.to_string());
        if let Some(services) = services {
            services.theme.set_skin(selected_skin_id);
        }
    };

    let set_high_contrast = move |enabled: bool| {
        high_contrast.set(enabled);
        if let Some(services) = services {
            services.theme.set_high_contrast(enabled);
        }
    };

    let set_reduced_motion = move |enabled: bool| {
        reduced_motion.set(enabled);
        if let Some(services) = services {
            services.theme.set_reduced_motion(enabled);
        }
    };

    view! {
        <div class="app-shell app-settings-shell">
            <div class="app-menubar settings-tablist" role="tablist" aria-label="Settings sections">
                <button
                    type="button"
                    class=move || if settings_state.get().active_tab == SettingsTab::Display {
                        "app-action settings-tab active"
                    } else {
                        "app-action settings-tab"
                    }
                    role="tab"
                    aria-selected=move || settings_state.get().active_tab == SettingsTab::Display
                    on:click=move |_| set_active_tab(SettingsTab::Display)
                >
                    {SettingsTab::Display.label()}
                </button>
                <button
                    type="button"
                    class=move || if settings_state.get().active_tab == SettingsTab::Accessibility {
                        "app-action settings-tab active"
                    } else {
                        "app-action settings-tab"
                    }
                    role="tab"
                    aria-selected=move || settings_state.get().active_tab == SettingsTab::Accessibility
                    on:click=move |_| set_active_tab(SettingsTab::Accessibility)
                >
                    {SettingsTab::Accessibility.label()}
                </button>
            </div>

            <div class="settings-content">
                <Show when=move || settings_state.get().active_tab == SettingsTab::Display fallback=|| ()>
                    <section class="settings-panel" aria-label="Display settings">
                        <h3 class="settings-heading">"Wallpaper"</h3>
                        <div class="settings-option-grid">
                            <For each=move || WALLPAPER_PRESETS.into_iter() key=|preset| preset.id let:preset>
                                <button
                                    type="button"
                                    class=move || if wallpaper_id.get() == preset.id {
                                        "settings-option selected"
                                    } else {
                                        "settings-option"
                                    }
                                    on:click=move |_| select_wallpaper(preset.id)
                                >
                                    <span class="settings-option-title">{preset.label}</span>
                                    <span class="settings-option-note">{preset.note}</span>
                                </button>
                            </For>
                        </div>

                        <h3 class="settings-heading">"Desktop skin"</h3>
                        <div class="settings-option-grid">
                            <For each=move || SKIN_PRESETS.into_iter() key=|preset| preset.id let:preset>
                                <button
                                    type="button"
                                    class=move || if skin_id.get() == preset.id {
                                        "settings-option selected"
                                    } else {
                                        "settings-option"
                                    }
                                    on:click=move |_| select_skin(preset.id)
                                >
                                    <span class="settings-option-title">{preset.label}</span>
                                    <span class="settings-option-note">{preset.note}</span>
                                </button>
                            </For>
                        </div>
                    </section>
                </Show>

                <Show when=move || settings_state.get().active_tab == SettingsTab::Accessibility fallback=|| ()>
                    <section class="settings-panel" aria-label="Accessibility settings">
                        <h3 class="settings-heading">"Accessibility"</h3>
                        <label class="settings-toggle">
                            <input
                                type="checkbox"
                                checked=move || high_contrast.get()
                                on:change=move |ev| set_high_contrast(event_target_checked(&ev))
                            />
                            <span>
                                <strong>"High contrast"</strong>
                                <small>"Increase shell contrast for stronger visual separation."</small>
                            </span>
                        </label>
                        <label class="settings-toggle">
                            <input
                                type="checkbox"
                                checked=move || reduced_motion.get()
                                on:change=move |ev| set_reduced_motion(event_target_checked(&ev))
                            />
                            <span>
                                <strong>"Reduced motion"</strong>
                                <small>"Minimize motion and transition intensity."</small>
                            </span>
                        </label>
                    </section>
                </Show>
            </div>

            <footer class="app-statusbar">
                <span>{move || format!("Skin: {}", skin_id.get())}</span>
                <span>{move || format!("Wallpaper: {}", wallpaper_id.get())}</span>
                <span>
                    {move || {
                        format!(
                            "A11y: contrast {} | motion {}",
                            if high_contrast.get() { "on" } else { "off" },
                            if reduced_motion.get() { "reduced" } else { "full" }
                        )
                    }}
                </span>
            </footer>
        </div>
    }
}
