use super::*;

#[component]
pub(super) fn DisplayPropertiesDialog(
    state: RwSignal<DesktopState>,
    display_properties_open: RwSignal<bool>,
    wallpaper_selection: RwSignal<String>,
    on_wallpaper_listbox_keydown: Callback<web_sys::KeyboardEvent>,
    preview_selected_wallpaper: Callback<()>,
    apply_selected_wallpaper: Callback<()>,
    close_display_properties_ok: Callback<()>,
    close_display_properties_cancel: Callback<()>,
) -> impl IntoView {
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
                                "wallpaper-listbox",
                                "display-properties-preview-button",
                                "display-properties-apply-button",
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
                                class="display-properties-tab active"
                                role="tab"
                                aria-selected="true"
                                aria-controls="display-properties-tabpanel-background"
                                tabindex="0"
                            >
                                "Background"
                            </button>
                            <button
                                id="display-properties-tab-appearance"
                                class="display-properties-tab"
                                role="tab"
                                aria-selected="false"
                                aria-controls="display-properties-tabpanel-background"
                                aria-disabled="true"
                                tabindex="-1"
                                disabled=true
                            >
                                "Appearance"
                            </button>
                            <button
                                id="display-properties-tab-effects"
                                class="display-properties-tab"
                                role="tab"
                                aria-selected="false"
                                aria-controls="display-properties-tabpanel-background"
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
