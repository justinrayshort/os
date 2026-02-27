use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DisplayPropertiesTab {
    Background,
    Appearance,
}

#[component]
pub(super) fn DisplayPropertiesDialog(
    state: RwSignal<DesktopState>,
    display_properties_open: RwSignal<bool>,
    wallpaper_selection: RwSignal<String>,
    skin_selection: RwSignal<DesktopSkin>,
    on_wallpaper_listbox_keydown: Callback<web_sys::KeyboardEvent>,
    on_skin_listbox_keydown: Callback<web_sys::KeyboardEvent>,
    preview_selected_wallpaper: Callback<()>,
    apply_selected_wallpaper: Callback<()>,
    preview_selected_skin: Callback<()>,
    apply_selected_skin: Callback<()>,
    close_display_properties_ok: Callback<()>,
    close_display_properties_cancel: Callback<()>,
) -> impl IntoView {
    let active_tab = create_rw_signal(DisplayPropertiesTab::Background);

    view! {
        <Show when=move || display_properties_open.get() fallback=|| ()>
            <div
                class="display-properties-overlay"
                on:mousedown=move |ev| ev.stop_propagation()
                on:click=move |ev| ev.stop_propagation()
            >
                <section
                    class="display-properties-dialog"
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="display-properties-title"
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        trap_tab_focus(
                            &ev,
                            &[
                                "display-properties-close-button",
                                "display-properties-tab-background",
                                "display-properties-tab-appearance",
                                "wallpaper-listbox",
                                "skin-listbox",
                                "display-properties-preview-button",
                                "display-properties-apply-button",
                                "display-properties-skin-preview-button",
                                "display-properties-skin-apply-button",
                                "display-properties-ok-button",
                                "display-properties-cancel-button",
                            ],
                        );
                    }
                    on:mousedown=move |ev| ev.stop_propagation()
                    on:click=move |ev| ev.stop_propagation()
                >
                    <header class="display-properties-titlebar">
                        <div id="display-properties-title" class="display-properties-title">
                            "Display Properties"
                        </div>
                        <button
                            id="display-properties-close-button"
                            class="display-properties-close"
                            aria-label="Close display properties"
                            on:click:undelegated=move |_| close_display_properties_cancel.call(())
                        >
                            <FluentIcon icon=IconName::Dismiss size=IconSize::Sm />
                        </button>
                    </header>

                    <div class="display-properties-body">
                        <div
                            class="display-properties-tabs"
                            role="tablist"
                            aria-label="Display settings"
                            aria-orientation="horizontal"
                        >
                            <button
                                id="display-properties-tab-background"
                                class=move || {
                                    if active_tab.get() == DisplayPropertiesTab::Background {
                                        "display-properties-tab active"
                                    } else {
                                        "display-properties-tab"
                                    }
                                }
                                role="tab"
                                aria-selected=move || active_tab.get() == DisplayPropertiesTab::Background
                                aria-controls="display-properties-tabpanel-background"
                                tabindex=move || {
                                    if active_tab.get() == DisplayPropertiesTab::Background {
                                        "0"
                                    } else {
                                        "-1"
                                    }
                                }
                                on:click=move |_| active_tab.set(DisplayPropertiesTab::Background)
                            >
                                "Background"
                            </button>
                            <button
                                id="display-properties-tab-appearance"
                                class=move || {
                                    if active_tab.get() == DisplayPropertiesTab::Appearance {
                                        "display-properties-tab active"
                                    } else {
                                        "display-properties-tab"
                                    }
                                }
                                role="tab"
                                aria-selected=move || active_tab.get() == DisplayPropertiesTab::Appearance
                                aria-controls="display-properties-tabpanel-appearance"
                                tabindex=move || {
                                    if active_tab.get() == DisplayPropertiesTab::Appearance {
                                        "0"
                                    } else {
                                        "-1"
                                    }
                                }
                                on:click=move |_| active_tab.set(DisplayPropertiesTab::Appearance)
                            >
                                "Appearance"
                            </button>
                            <button
                                id="display-properties-tab-effects"
                                class="display-properties-tab"
                                role="tab"
                                aria-selected="false"
                                aria-controls="display-properties-tabpanel-effects"
                                aria-disabled="true"
                                tabindex="-1"
                                disabled=true
                            >
                                "Effects"
                            </button>
                        </div>

                        <div
                            id="display-properties-tabpanel-background"
                            class="display-properties-content"
                            role="tabpanel"
                            aria-labelledby="display-properties-tab-background"
                            hidden=move || active_tab.get() != DisplayPropertiesTab::Background
                        >
                            <div class="display-preview-column">
                                <div class="display-preview-frame" aria-hidden="true">
                                    <div class="display-preview-monitor">
                                        <div
                                            class="display-preview-screen"
                                            data-wallpaper=move || {
                                                wallpaper_preset_by_id(&wallpaper_selection.get()).id
                                            }
                                        >
                                            <div class="display-preview-desktop-icon">
                                                "My Computer"
                                            </div>
                                        </div>
                                        <div class="display-preview-taskbar">
                                            <span class="display-preview-start">"Start"</span>
                                            <span class="display-preview-clock">"9:41 AM"</span>
                                        </div>
                                    </div>
                                </div>

                                <div class="display-preview-caption">
                                    {move || {
                                        let preset = wallpaper_preset_by_id(&wallpaper_selection.get());
                                        format!("{} ({})", preset.label, wallpaper_preset_kind_label(preset.kind))
                                    }}
                                </div>
                                <div class="display-preview-note">
                                    {move || wallpaper_preset_by_id(&wallpaper_selection.get()).note}
                                </div>
                            </div>

                            <div class="display-options-column">
                                <div id="wallpaper-listbox-label" class="display-list-label">
                                    "Wallpaper"
                                </div>
                                <div
                                    id="wallpaper-listbox"
                                    class="wallpaper-picker-list"
                                    role="listbox"
                                    tabindex="0"
                                    aria-labelledby="wallpaper-listbox-label"
                                    aria-activedescendant=move || {
                                        wallpaper_option_dom_id(&wallpaper_selection.get())
                                    }
                                    aria-orientation="vertical"
                                    on:keydown=move |ev| on_wallpaper_listbox_keydown.call(ev)
                                >
                                    <For
                                        each=move || desktop_wallpaper_presets().to_vec()
                                        key=|preset| preset.id
                                        let:preset
                                    >
                                        <div
                                            id=wallpaper_option_dom_id(preset.id)
                                            class=move || {
                                                if wallpaper_preset_by_id(&wallpaper_selection.get()).id == preset.id {
                                                    "wallpaper-picker-item selected"
                                                } else {
                                                    "wallpaper-picker-item"
                                                }
                                            }
                                            role="option"
                                            aria-selected=move || {
                                                wallpaper_preset_by_id(&wallpaper_selection.get()).id == preset.id
                                            }
                                            tabindex="-1"
                                            on:click:undelegated=move |_| {
                                                wallpaper_selection.set(preset.id.to_string());
                                            }
                                            on:dblclick:undelegated=move |_| {
                                                wallpaper_selection.set(preset.id.to_string());
                                                preview_selected_wallpaper.call(());
                                            }
                                        >
                                            <span
                                                class="wallpaper-preview-thumb"
                                                data-wallpaper=preset.id
                                                aria-hidden="true"
                                            />
                                            <span class="wallpaper-picker-item-copy">
                                                <span class="wallpaper-picker-item-label">
                                                    {preset.label}
                                                </span>
                                                <span class="wallpaper-picker-item-meta">
                                                    {wallpaper_preset_kind_label(preset.kind)}
                                                </span>
                                            </span>
                                        </div>
                                    </For>
                                </div>

                                <div class="display-properties-actions-row">
                                    <button
                                        id="display-properties-preview-button"
                                        class="display-action-button"
                                        on:click:undelegated=move |_| preview_selected_wallpaper.call(())
                                    >
                                        "Preview"
                                    </button>
                                    <button
                                        id="display-properties-apply-button"
                                        class="display-action-button"
                                        on:click:undelegated=move |_| apply_selected_wallpaper.call(())
                                    >
                                        "Apply"
                                    </button>
                                </div>

                                <div class="display-properties-current">
                                    {move || {
                                        let current = wallpaper_preset_by_id(&state.get().theme.wallpaper_id);
                                        format!("Current desktop: {}", current.label)
                                    }}
                                </div>
                            </div>
                        </div>

                        <div
                            id="display-properties-tabpanel-appearance"
                            class="display-properties-content"
                            role="tabpanel"
                            aria-labelledby="display-properties-tab-appearance"
                            hidden=move || active_tab.get() != DisplayPropertiesTab::Appearance
                        >
                            <div class="display-preview-column">
                                <div class="display-preview-frame" aria-hidden="true">
                                    <div class="display-preview-monitor">
                                        <div class="display-preview-screen">
                                            <div class="display-preview-desktop-icon">"Desktop"</div>
                                        </div>
                                        <div class="display-preview-taskbar">
                                            <span class="display-preview-start">"Skin"</span>
                                            <span class="display-preview-clock">
                                                {move || skin_selection.get().label()}
                                            </span>
                                        </div>
                                    </div>
                                </div>
                                <div class="display-preview-caption">{move || skin_selection.get().label()}</div>
                                <div class="display-preview-note">
                                    {move || {
                                        desktop_skin_presets()
                                            .iter()
                                            .find(|preset| preset.skin == skin_selection.get())
                                            .map(|preset| preset.note)
                                            .unwrap_or("Shell skin preset")
                                    }}
                                </div>
                            </div>

                            <div class="display-options-column">
                                <div id="skin-listbox-label" class="display-list-label">
                                    "Desktop Skin"
                                </div>
                                <div
                                    id="skin-listbox"
                                    class="wallpaper-picker-list"
                                    role="listbox"
                                    tabindex="0"
                                    aria-labelledby="skin-listbox-label"
                                    aria-activedescendant=move || skin_option_dom_id(skin_selection.get())
                                    aria-orientation="vertical"
                                    on:keydown=move |ev| on_skin_listbox_keydown.call(ev)
                                >
                                    <For
                                        each=move || desktop_skin_presets().to_vec()
                                        key=|preset| preset.skin.css_id()
                                        let:preset
                                    >
                                        <div
                                            id=skin_option_dom_id(preset.skin)
                                            class=move || {
                                                if skin_selection.get() == preset.skin {
                                                    "wallpaper-picker-item selected"
                                                } else {
                                                    "wallpaper-picker-item"
                                                }
                                            }
                                            role="option"
                                            aria-selected=move || skin_selection.get() == preset.skin
                                            tabindex="-1"
                                            on:click:undelegated=move |_| skin_selection.set(preset.skin)
                                            on:dblclick:undelegated=move |_| {
                                                skin_selection.set(preset.skin);
                                                preview_selected_skin.call(());
                                            }
                                        >
                                            <span class="desktop-context-wallpaper-check" aria-hidden="true">
                                                <FluentIcon icon=IconName::Checkmark size=IconSize::Xs />
                                            </span>
                                            <span class="wallpaper-picker-item-copy">
                                                <span class="wallpaper-picker-item-label">
                                                    {preset.skin.label()}
                                                </span>
                                                <span class="wallpaper-picker-item-meta">{preset.note}</span>
                                            </span>
                                        </div>
                                    </For>
                                </div>

                                <div class="display-properties-actions-row">
                                    <button
                                        id="display-properties-skin-preview-button"
                                        class="display-action-button"
                                        on:click:undelegated=move |_| preview_selected_skin.call(())
                                    >
                                        "Preview"
                                    </button>
                                    <button
                                        id="display-properties-skin-apply-button"
                                        class="display-action-button"
                                        on:click:undelegated=move |_| apply_selected_skin.call(())
                                    >
                                        "Apply"
                                    </button>
                                </div>

                                <div class="display-properties-current">
                                    {move || format!("Current skin: {}", state.get().theme.skin.label())}
                                </div>
                            </div>
                        </div>
                    </div>

                    <footer class="display-properties-footer">
                        <button
                            id="display-properties-ok-button"
                            class="display-footer-button"
                            on:click:undelegated=move |_| close_display_properties_ok.call(())
                        >
                            "OK"
                        </button>
                        <button
                            id="display-properties-cancel-button"
                            class="display-footer-button"
                            on:click:undelegated=move |_| close_display_properties_cancel.call(())
                        >
                            "Cancel"
                        </button>
                    </footer>
                </section>
            </div>
        </Show>
    }
}
