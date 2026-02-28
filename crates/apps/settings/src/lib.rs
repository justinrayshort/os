//! Built-in System Settings desktop app for wallpaper, theme, and accessibility preferences.
//!
//! The app consumes the injected v2 service surface from [`desktop_app_contract::AppServices`]
//! so wallpaper and theme configuration stay synchronized with the desktop runtime.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

use desktop_app_contract::AppServices;
use leptos::*;
use platform_host::{
    WallpaperAnimationPolicy, WallpaperAssetRecord, WallpaperCollection, WallpaperConfig,
    WallpaperDisplayMode, WallpaperMediaKind, WallpaperPosition, WallpaperSelection,
    WallpaperSourceKind,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum SettingsTab {
    Wallpaper,
    Theme,
    Accessibility,
}

impl SettingsTab {
    fn label(self) -> &'static str {
        match self {
            Self::Wallpaper => "Wallpaper",
            Self::Theme => "Theme",
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
            active_tab: SettingsTab::Wallpaper,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SkinPreset {
    id: &'static str,
    label: &'static str,
    note: &'static str,
}

const SKIN_PRESETS: [SkinPreset; 3] = [
    SkinPreset {
        id: "modern-adaptive",
        label: "Modern Adaptive",
        note: "Modern Fluent-inspired shell styling",
    },
    SkinPreset {
        id: "classic-xp",
        label: "Classic XP",
        note: "Glossy nostalgic shell styling",
    },
    SkinPreset {
        id: "classic-95",
        label: "Classic 95",
        note: "Sharp retro shell styling",
    },
];

#[component]
/// Settings app window contents.
pub fn SettingsApp(
    /// Legacy launch params, retained for compatibility.
    _launch_params: Value,
    /// Manager-restored app state payload.
    restored_state: Option<Value>,
    /// Injected desktop services bundle.
    services: Option<AppServices>,
) -> impl IntoView {
    let services = services.expect("settings requires app services");
    let settings_state = create_rw_signal(SettingsAppState::default());
    let search = create_rw_signal(String::new());
    let selected_asset_id = create_rw_signal(String::new());
    let rename_value = create_rw_signal(String::new());
    let tags_value = create_rw_signal(String::new());
    let new_collection_name = create_rw_signal(String::new());

    if let Some(restored_state) = restored_state {
        if let Ok(restored) = serde_json::from_value::<SettingsAppState>(restored_state) {
            settings_state.set(restored);
        }
    }

    create_effect(move |_| {
        if let Ok(serialized) = serde_json::to_value(settings_state.get()) {
            services.state.persist_window_state(serialized);
        }
    });

    let active_wallpaper = Signal::derive(move || {
        services
            .wallpaper
            .preview
            .get()
            .unwrap_or_else(|| services.wallpaper.current.get())
    });
    let wallpaper_library = Signal::derive(move || services.wallpaper.library.get());
    let theme_skin_id = Signal::derive({
        let services = services.clone();
        move || services.theme.skin_id.get()
    });
    let theme_high_contrast = Signal::derive({
        let services = services.clone();
        move || services.theme.high_contrast.get()
    });
    let theme_reduced_motion = Signal::derive({
        let services = services.clone();
        move || services.theme.reduced_motion.get()
    });
    let services_for_skin_click = services.clone();
    let services_for_high_contrast = services.clone();
    let services_for_reduced_motion = services.clone();

    create_effect(move |_| {
        let library = wallpaper_library.get();
        if selected_asset_id.get_untracked().is_empty() {
            if let Some(asset) = library.assets.first() {
                selected_asset_id.set(asset.asset_id.clone());
                rename_value.set(asset.display_name.clone());
                tags_value.set(asset.tags.join(", "));
            }
            return;
        }

        if let Some(asset) = library
            .assets
            .iter()
            .find(|asset| asset.asset_id == selected_asset_id.get())
        {
            rename_value.set(asset.display_name.clone());
            tags_value.set(asset.tags.join(", "));
        }
    });

    let selected_asset = Signal::derive(move || {
        wallpaper_library
            .get()
            .assets
            .into_iter()
            .find(|asset| asset.asset_id == selected_asset_id.get())
    });

    let filtered_assets = Signal::derive(move || {
        let query = search.get().trim().to_ascii_lowercase();
        wallpaper_library
            .get()
            .assets
            .into_iter()
            .filter(|asset| {
                if query.is_empty() {
                    return true;
                }
                asset.display_name.to_ascii_lowercase().contains(&query)
                    || asset
                        .tags
                        .iter()
                        .any(|tag| tag.to_ascii_lowercase().contains(&query))
            })
            .collect::<Vec<_>>()
    });

    let preview_asset = move |asset: &WallpaperAssetRecord| {
        selected_asset_id.set(asset.asset_id.clone());
        rename_value.set(asset.display_name.clone());
        tags_value.set(asset.tags.join(", "));
        services
            .wallpaper
            .preview(asset_to_config(asset, &active_wallpaper.get_untracked()));
    };

    let preview_mode = move |display_mode: WallpaperDisplayMode| {
        let mut config = active_wallpaper.get_untracked();
        config.display_mode = display_mode;
        services.wallpaper.preview(config);
    };

    let preview_position = move |position: WallpaperPosition| {
        let mut config = active_wallpaper.get_untracked();
        config.position = position;
        services.wallpaper.preview(config);
    };

    let apply_preview = move |_| services.wallpaper.apply_preview();
    let revert_preview = move |_| services.wallpaper.clear_preview();
    let import_wallpaper = move |_| {
        services.wallpaper.import_from_picker(Default::default());
    };
    let save_rename = move |_| {
        if let Some(asset) = selected_asset.get_untracked() {
            services
                .wallpaper
                .rename_asset(asset.asset_id, rename_value.get_untracked());
        }
    };
    let save_tags = move |_| {
        if let Some(asset) = selected_asset.get_untracked() {
            let tags = tags_value
                .get_untracked()
                .split(',')
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            services.wallpaper.set_tags(asset.asset_id, tags);
        }
    };
    let toggle_favorite = move |_| {
        if let Some(asset) = selected_asset.get_untracked() {
            services
                .wallpaper
                .set_favorite(asset.asset_id, !asset.favorite);
        }
    };
    let delete_asset = move |_| {
        if let Some(asset) = selected_asset.get_untracked() {
            services.wallpaper.delete_asset(asset.asset_id);
            selected_asset_id.set(String::new());
        }
    };
    let create_collection = move |_| {
        let name = new_collection_name.get_untracked();
        if !name.trim().is_empty() {
            services.wallpaper.create_collection(name.trim());
            new_collection_name.set(String::new());
        }
    };

    view! {
        <div class="app-shell app-settings-shell">
            <div class="app-menubar settings-tablist" role="tablist" aria-label="Settings sections">
                <For each=move || [SettingsTab::Wallpaper, SettingsTab::Theme, SettingsTab::Accessibility] key=|tab| *tab as u8 let:tab>
                    <button
                        type="button"
                        class=move || if settings_state.get().active_tab == tab {
                            "app-action settings-tab active"
                        } else {
                            "app-action settings-tab"
                        }
                        role="tab"
                        aria-selected=move || settings_state.get().active_tab == tab
                        on:click=move |_| settings_state.update(|settings| settings.active_tab = tab)
                    >
                        {tab.label()}
                    </button>
                </For>
            </div>

            <div class="settings-content">
                <Show when=move || settings_state.get().active_tab == SettingsTab::Wallpaper fallback=|| ()>
                    <section class="settings-panel" aria-label="Wallpaper settings">
                        <div class="settings-toolbar">
                            <button type="button" class="app-action" on:click=import_wallpaper>"Import…"</button>
                            <button
                                type="button"
                                class="app-action"
                                disabled=move || services.wallpaper.preview.get().is_none()
                                on:click=apply_preview
                            >
                                "Apply"
                            </button>
                            <button
                                type="button"
                                class="app-action"
                                disabled=move || services.wallpaper.preview.get().is_none()
                                on:click=revert_preview
                            >
                                "Revert"
                            </button>
                            <input
                                type="search"
                                class="settings-search"
                                placeholder="Search wallpapers"
                                prop:value=move || search.get()
                                on:input=move |ev| search.set(event_target_value(&ev))
                            />
                        </div>

                        <div class="display-preview-screen">
                            <WallpaperPreview config=active_wallpaper />
                        </div>

                        <h3 class="settings-heading">"Display mode"</h3>
                        <div class="settings-option-grid">
                            <For each=move || wallpaper_display_modes() key=|mode| *mode as u8 let:mode>
                                <button
                                    type="button"
                                    class=move || if active_wallpaper.get().display_mode == mode {
                                        "settings-option selected"
                                    } else {
                                        "settings-option"
                                    }
                                    on:click=move |_| preview_mode(mode)
                                >
                                    <span class="settings-option-title">{wallpaper_display_mode_label(mode)}</span>
                                </button>
                            </For>
                        </div>

                        <h3 class="settings-heading">"Position"</h3>
                        <div class="settings-option-grid">
                            <For each=move || wallpaper_positions() key=|position| *position as u8 let:position>
                                <button
                                    type="button"
                                    class=move || if active_wallpaper.get().position == position {
                                        "settings-option selected"
                                    } else {
                                        "settings-option"
                                    }
                                    on:click=move |_| preview_position(position)
                                >
                                    <span class="settings-option-title">{wallpaper_position_label(position)}</span>
                                </button>
                            </For>
                        </div>

                        <h3 class="settings-heading">"Library"</h3>
                        <div class="wallpaper-picker-list">
                            <For each=move || filtered_assets.get() key=|asset| asset.asset_id.clone() let:asset>
                                <WallpaperLibraryItem
                                    asset=asset
                                    selected_asset_id=selected_asset_id
                                    on_preview=Callback::new(move |asset| preview_asset(&asset))
                                />
                            </For>
                        </div>

                        <Show when=move || selected_asset.get().is_some() fallback=|| ()>
                            <div class="settings-form-grid">
                                <label class="settings-field">
                                    <span>"Name"</span>
                                    <input
                                        type="text"
                                        prop:value=move || rename_value.get()
                                        on:input=move |ev| rename_value.set(event_target_value(&ev))
                                    />
                                </label>
                                <button type="button" class="app-action" on:click=save_rename>"Rename"</button>

                                <label class="settings-field">
                                    <span>"Tags"</span>
                                    <input
                                        type="text"
                                        placeholder="comma, separated, tags"
                                        prop:value=move || tags_value.get()
                                        on:input=move |ev| tags_value.set(event_target_value(&ev))
                                    />
                                </label>
                                <button type="button" class="app-action" on:click=save_tags>"Save tags"</button>

                                <button type="button" class="app-action" on:click=toggle_favorite>
                                    {move || if selected_asset.get().map(|asset| asset.favorite).unwrap_or(false) {
                                        "Unfavorite"
                                    } else {
                                        "Favorite"
                                    }}
                                </button>

                                <Show when=move || selected_asset.get().map(|asset| asset.source_kind == WallpaperSourceKind::Imported).unwrap_or(false) fallback=|| ()>
                                    <button type="button" class="app-action" on:click=delete_asset>"Delete imported asset"</button>
                                </Show>
                            </div>

                            <h3 class="settings-heading">"Collections"</h3>
                            <div class="settings-option-grid">
                                <For each=move || wallpaper_library.get().collections key=|collection| collection.collection_id.clone() let:collection>
                                    <WallpaperCollectionItem
                                        collection=collection
                                        selected_asset=selected_asset
                                        on_toggle=Callback::new(move |collection_id: String| {
                                            if let Some(asset) = selected_asset.get_untracked() {
                                                let mut collection_ids = asset.collection_ids.clone();
                                                if collection_ids.iter().any(|id| *id == collection_id) {
                                                    collection_ids.retain(|id| *id != collection_id);
                                                } else {
                                                    collection_ids.push(collection_id);
                                                }
                                                services.wallpaper.set_collections(asset.asset_id, collection_ids);
                                            }
                                        })
                                    />
                                </For>
                            </div>
                            <div class="settings-toolbar">
                                <input
                                    type="text"
                                    class="settings-search"
                                    placeholder="New collection"
                                    prop:value=move || new_collection_name.get()
                                    on:input=move |ev| new_collection_name.set(event_target_value(&ev))
                                />
                                <button type="button" class="app-action" on:click=create_collection>"Create collection"</button>
                            </div>
                        </Show>
                    </section>
                </Show>

                <Show when=move || settings_state.get().active_tab == SettingsTab::Theme fallback=|| ()>
                    <section class="settings-panel" aria-label="Theme settings">
                        <h3 class="settings-heading">"Desktop skin"</h3>
                        <div class="settings-option-grid">
                            <For each=move || SKIN_PRESETS.into_iter() key=|preset| preset.id let:preset>
                                <button
                                    type="button"
                                    class=move || if theme_skin_id.get() == preset.id {
                                        "settings-option selected"
                                    } else {
                                        "settings-option"
                                    }
                                    on:click=move |_| services_for_skin_click.theme.set_skin(preset.id)
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
                                checked=move || theme_high_contrast.get()
                                on:change=move |ev| services_for_high_contrast.theme.set_high_contrast(event_target_checked(&ev))
                            />
                            <span>
                                <strong>"High contrast"</strong>
                                <small>"Increase shell contrast for stronger visual separation."</small>
                            </span>
                        </label>
                        <label class="settings-toggle">
                            <input
                                type="checkbox"
                                checked=move || theme_reduced_motion.get()
                                on:change=move |ev| services_for_reduced_motion.theme.set_reduced_motion(event_target_checked(&ev))
                            />
                            <span>
                                <strong>"Reduced motion"</strong>
                                <small>"Render animated wallpaper media using static fallback."</small>
                            </span>
                        </label>
                    </section>
                </Show>
            </div>

            <footer class="app-statusbar">
                <span>{move || format!("Skin: {}", theme_skin_id.get())}</span>
                <span>
                    {move || {
                        let config = active_wallpaper.get();
                        match config.selection {
                            WallpaperSelection::BuiltIn { wallpaper_id } => format!("Wallpaper: {wallpaper_id}"),
                            WallpaperSelection::Imported { asset_id } => format!("Wallpaper: {asset_id}"),
                        }
                    }}
                </span>
                <span>{move || format!("Library assets: {}", wallpaper_library.get().assets.len())}</span>
            </footer>
        </div>
    }
}

fn asset_to_config(asset: &WallpaperAssetRecord, current: &WallpaperConfig) -> WallpaperConfig {
    let animation = match asset.media_kind {
        WallpaperMediaKind::AnimatedImage | WallpaperMediaKind::Video => {
            WallpaperAnimationPolicy::LoopMuted
        }
        _ => WallpaperAnimationPolicy::None,
    };
    WallpaperConfig {
        selection: match asset.source_kind {
            WallpaperSourceKind::BuiltIn => WallpaperSelection::BuiltIn {
                wallpaper_id: asset.asset_id.clone(),
            },
            WallpaperSourceKind::Imported => WallpaperSelection::Imported {
                asset_id: asset.asset_id.clone(),
            },
        },
        display_mode: current.display_mode,
        position: current.position,
        animation,
    }
}

fn wallpaper_display_modes() -> [WallpaperDisplayMode; 5] {
    [
        WallpaperDisplayMode::Fill,
        WallpaperDisplayMode::Fit,
        WallpaperDisplayMode::Stretch,
        WallpaperDisplayMode::Tile,
        WallpaperDisplayMode::Center,
    ]
}

fn wallpaper_display_mode_label(mode: WallpaperDisplayMode) -> &'static str {
    match mode {
        WallpaperDisplayMode::Fill => "Fill",
        WallpaperDisplayMode::Fit => "Fit",
        WallpaperDisplayMode::Stretch => "Stretch",
        WallpaperDisplayMode::Tile => "Tile",
        WallpaperDisplayMode::Center => "Center",
    }
}

fn wallpaper_positions() -> [WallpaperPosition; 9] {
    [
        WallpaperPosition::TopLeft,
        WallpaperPosition::Top,
        WallpaperPosition::TopRight,
        WallpaperPosition::Left,
        WallpaperPosition::Center,
        WallpaperPosition::Right,
        WallpaperPosition::BottomLeft,
        WallpaperPosition::Bottom,
        WallpaperPosition::BottomRight,
    ]
}

fn wallpaper_position_label(position: WallpaperPosition) -> &'static str {
    match position {
        WallpaperPosition::TopLeft => "Top left",
        WallpaperPosition::Top => "Top",
        WallpaperPosition::TopRight => "Top right",
        WallpaperPosition::Left => "Left",
        WallpaperPosition::Center => "Center",
        WallpaperPosition::Right => "Right",
        WallpaperPosition::BottomLeft => "Bottom left",
        WallpaperPosition::Bottom => "Bottom",
        WallpaperPosition::BottomRight => "Bottom right",
    }
}

fn asset_label_prefix(asset: &WallpaperAssetRecord) -> &'static str {
    match asset.source_kind {
        WallpaperSourceKind::BuiltIn => "Built-in",
        WallpaperSourceKind::Imported => "Imported",
    }
}

#[component]
fn WallpaperLibraryItem(
    asset: WallpaperAssetRecord,
    selected_asset_id: RwSignal<String>,
    on_preview: Callback<WallpaperAssetRecord>,
) -> impl IntoView {
    let asset_id = asset.asset_id.clone();
    let asset_for_click = asset.clone();
    let display_name = asset.display_name.clone();
    let meta = format!(
        "{}{}{}",
        asset_label_prefix(&asset),
        if asset.favorite { " | favorite" } else { "" },
        if asset.tags.is_empty() {
            String::new()
        } else {
            format!(" | {}", asset.tags.join(", "))
        }
    );

    view! {
        <button
            type="button"
            class=move || if selected_asset_id.get() == asset_id {
                "wallpaper-picker-item selected"
            } else {
                "wallpaper-picker-item"
            }
            on:click=move |_| on_preview.call(asset_for_click.clone())
        >
            <span class="wallpaper-preview-thumb">
                <WallpaperThumb asset=asset.clone() />
            </span>
            <span class="wallpaper-picker-item-copy">
                <span class="wallpaper-picker-item-label">{display_name}</span>
                <span class="wallpaper-picker-item-meta">{meta}</span>
            </span>
        </button>
    }
}

#[component]
fn WallpaperCollectionItem(
    collection: WallpaperCollection,
    selected_asset: Signal<Option<WallpaperAssetRecord>>,
    on_toggle: Callback<String>,
) -> impl IntoView {
    let collection_id = collection.collection_id.clone();

    view! {
        <button
            type="button"
            class=move || {
                let is_member = selected_asset
                    .get()
                    .map(|asset| asset.collection_ids.iter().any(|id| *id == collection_id))
                    .unwrap_or(false);
                if is_member {
                    "settings-option selected"
                } else {
                    "settings-option"
                }
            }
            on:click=move |_| on_toggle.call(collection.collection_id.clone())
        >
            <span class="settings-option-title">{collection.display_name}</span>
        </button>
    }
}

#[component]
fn WallpaperThumb(asset: WallpaperAssetRecord) -> impl IntoView {
    view! {
        <img src=asset.poster_url.unwrap_or(asset.primary_url) alt=asset.display_name />
    }
}

#[component]
fn WallpaperPreview(config: Signal<WallpaperConfig>) -> impl IntoView {
    view! {
        <div class="wallpaper-preview-thumb">
            {move || match config.get().selection {
                WallpaperSelection::BuiltIn { wallpaper_id } => view! {
                    <span>{format!("Built-in: {wallpaper_id}")}</span>
                }
                .into_view(),
                WallpaperSelection::Imported { asset_id } => view! {
                    <span>{format!("Imported: {asset_id}")}</span>
                }
                .into_view(),
            }}
            <small>
                {move || format!(
                    "{} / {}",
                    wallpaper_display_mode_label(config.get().display_mode),
                    wallpaper_position_label(config.get().position)
                )}
            </small>
        </div>
    }
}
