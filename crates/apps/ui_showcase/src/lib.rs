//! Built-in UI showcase app for the shared neumorphic design system.
//!
//! The app renders every reusable control family through `system_ui`
//! primitives so visual refinements can be reviewed in a production surface
//! without introducing app-local design contracts.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

use desktop_app_contract::AppServices;
use leptos::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use system_ui::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum BinaryChoice {
    Yes,
    No,
}

impl BinaryChoice {
    fn label(self) -> &'static str {
        match self {
            Self::Yes => "Yes",
            Self::No => "No",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UiShowcaseState {
    binary_choice: BinaryChoice,
    switch_enabled: bool,
    slider_value: u16,
    progress_value: u16,
    modal_open: bool,
    dial_value: i32,
    dropdown_value: String,
}

impl Default for UiShowcaseState {
    fn default() -> Self {
        Self {
            binary_choice: BinaryChoice::Yes,
            switch_enabled: true,
            slider_value: 64,
            progress_value: 72,
            modal_open: false,
            dial_value: 58,
            dropdown_value: "Soft Surface".to_string(),
        }
    }
}

#[component]
/// UI showcase app window contents.
pub fn UiShowcaseApp(
    /// Legacy launch parameters. Reserved for future deep-link demos.
    launch_params: Value,
    /// Manager-restored app state payload.
    restored_state: Option<Value>,
    /// Optional app services for persisted window state.
    services: Option<AppServices>,
) -> impl IntoView {
    let _ = launch_params;

    let state = create_rw_signal(UiShowcaseState::default());
    let last_saved = create_rw_signal::<Option<String>>(None);
    let services_for_persist = services.clone();
    let dropdown_menu_open = create_rw_signal(false);
    let text_value = create_rw_signal("Soft controls hold focus clearly.".to_string());
    let area_value = create_rw_signal(
        "Every control shares one light source, one accent family, and one depth grammar."
            .to_string(),
    );

    if let Some(restored_state) = restored_state {
        if let Ok(restored) = serde_json::from_value::<UiShowcaseState>(restored_state) {
            last_saved.set(serde_json::to_string(&restored).ok());
            state.set(restored);
        }
    }

    create_effect(move |_| {
        let snapshot = state.get();
        let serialized = match serde_json::to_string(&snapshot) {
            Ok(serialized) => serialized,
            Err(err) => {
                logging::warn!("ui showcase serialize failed: {err}");
                return;
            }
        };

        if last_saved.get().as_deref() == Some(serialized.as_str()) {
            return;
        }
        last_saved.set(Some(serialized));

        if let Some(services) = services_for_persist.clone() {
            if let Ok(value) = serde_json::to_value(&snapshot) {
                services.state.persist_window_state(value);
            }
        }
    });

    view! {
        <AppShell>
            <Surface variant=SurfaceVariant::Muted elevation=Elevation::Inset>
                <Stack gap=LayoutGap::Lg>
                    <PaneHeader title="Neumorphic UI Showcase" meta="Shared primitives, shared tokens, one continuous surface">
                        <Badge>"soft-neumorphic"</Badge>
                    </PaneHeader>

                    <Panel>
                        <Stack gap=LayoutGap::Md>
                            <Heading role=TextRole::Title>"Buttons"</Heading>
                            <Cluster gap=LayoutGap::Sm>
                                <Button variant=ButtonVariant::Primary>"Primary CTA"</Button>
                                <Button variant=ButtonVariant::Secondary>"Secondary"</Button>
                                <Button variant=ButtonVariant::Quiet>"Quiet"</Button>
                                <Button variant=ButtonVariant::Primary pressed=true>"Pressed"</Button>
                                <Button variant=ButtonVariant::Secondary disabled=true>"Disabled"</Button>
                            </Cluster>
                            <Cluster gap=LayoutGap::Sm>
                                <SegmentedControl aria_label="Binary choice">
                                    <SegmentedControlOption
                                        selected=Signal::derive(move || state.get().binary_choice == BinaryChoice::Yes)
                                        on_click=Callback::new(move |_| {
                                            state.update(|value| value.binary_choice = BinaryChoice::Yes);
                                        })
                                    >
                                        "Yes"
                                    </SegmentedControlOption>
                                    <SegmentedControlOption
                                        selected=Signal::derive(move || state.get().binary_choice == BinaryChoice::No)
                                        on_click=Callback::new(move |_| {
                                            state.update(|value| value.binary_choice = BinaryChoice::No);
                                        })
                                    >
                                        "No"
                                    </SegmentedControlOption>
                                </SegmentedControl>
                                <Button shape=ButtonShape::Pill variant=ButtonVariant::Secondary>
                                    {move || format!("Selected: {}", state.get().binary_choice.label())}
                                </Button>
                            </Cluster>
                            <Cluster gap=LayoutGap::Sm>
                                <IconButton icon=IconName::Play aria_label="Play" />
                                <IconButton icon=IconName::Pause aria_label="Pause" />
                                <IconButton icon=IconName::Stop aria_label="Stop" />
                                <IconButton icon=IconName::Next aria_label="Next" />
                                <IconButton icon=IconName::Home aria_label="Home" />
                            </Cluster>
                        </Stack>
                    </Panel>

                    <Panel>
                        <Stack gap=LayoutGap::Md>
                            <Heading role=TextRole::Title>"Sliders"</Heading>
                            <RangeField
                                min="0"
                                max="100"
                                aria_label="Ambient depth"
                                value=Signal::derive(move || state.get().slider_value.to_string())
                                on_input=Callback::new(move |ev| {
                                    if let Ok(parsed) = event_target_value(&ev).parse::<u16>() {
                                        state.update(|value| {
                                            value.slider_value = parsed.min(100);
                                            value.progress_value = parsed.min(100);
                                        });
                                    }
                                })
                            />
                            <Text tone=TextTone::Secondary>
                                {move || format!("Slider value: {}", state.get().slider_value)}
                            </Text>
                        </Stack>
                    </Panel>

                    <Panel>
                        <Stack gap=LayoutGap::Md>
                            <Heading role=TextRole::Title>"Dropdowns"</Heading>
                            <SelectField
                                aria_label="Surface preset"
                                value=Signal::derive(move || state.get().dropdown_value.clone())
                                on_change=Callback::new(move |ev| {
                                    let next = event_target_value(&ev);
                                    state.update(|value| value.dropdown_value = next);
                                })
                            >
                                <option value="Soft Surface">"Soft Surface"</option>
                                <option value="Accent Pulse">"Accent Pulse"</option>
                                <option value="Inset Focus">"Inset Focus"</option>
                            </SelectField>
                            <Cluster gap=LayoutGap::Sm>
                                <Button
                                    variant=ButtonVariant::Secondary
                                    on_click=Callback::new(move |_| {
                                        dropdown_menu_open.update(|open| *open = !*open);
                                    })
                                >
                                    "Show Dropdown Panel"
                                </Button>
                                <Text tone=TextTone::Secondary>
                                    {move || format!("Native select: {}", state.get().dropdown_value)}
                                </Text>
                            </Cluster>
                            <Show when=move || dropdown_menu_open.get() fallback=|| ()>
                                <MenuSurface role="listbox" aria_label="Dropdown panel">
                                    <MenuItem
                                        selected=Signal::derive(move || state.get().dropdown_value == "Soft Surface")
                                        on_click=Callback::new(move |_| {
                                            state.update(|value| value.dropdown_value = "Soft Surface".to_string());
                                            dropdown_menu_open.set(false);
                                        })
                                    >
                                        "Soft Surface"
                                    </MenuItem>
                                    <MenuItem
                                        selected=Signal::derive(move || state.get().dropdown_value == "Accent Pulse")
                                        on_click=Callback::new(move |_| {
                                            state.update(|value| value.dropdown_value = "Accent Pulse".to_string());
                                            dropdown_menu_open.set(false);
                                        })
                                    >
                                        "Accent Pulse"
                                    </MenuItem>
                                    <MenuItem
                                        selected=Signal::derive(move || state.get().dropdown_value == "Inset Focus")
                                        on_click=Callback::new(move |_| {
                                            state.update(|value| value.dropdown_value = "Inset Focus".to_string());
                                            dropdown_menu_open.set(false);
                                        })
                                    >
                                        "Inset Focus"
                                    </MenuItem>
                                </MenuSurface>
                            </Show>
                        </Stack>
                    </Panel>

                    <Panel>
                        <Stack gap=LayoutGap::Md>
                            <Heading role=TextRole::Title>"Switches"</Heading>
                            <ToggleRow
                                title="Reduced glare"
                                description="Shows the dedicated shared switch primitive with keyboard support."
                                checked=Signal::derive(move || state.get().switch_enabled)
                            >
                                <Switch
                                    aria_label="Reduced glare"
                                    checked=Signal::derive(move || state.get().switch_enabled)
                                    on_toggle=Callback::new(move |next| {
                                        state.update(|value| value.switch_enabled = next);
                                    })
                                />
                            </ToggleRow>
                        </Stack>
                    </Panel>

                    <Panel>
                        <Stack gap=LayoutGap::Md>
                            <Heading role=TextRole::Title>"Cards and Modal"</Heading>
                            <Card>
                                <Heading role=TextRole::Title>"Elevated card"</Heading>
                                <Text tone=TextTone::Secondary>
                                    "Cards stay on the same surface plane while using a slightly stronger raised shadow."
                                </Text>
                                <Cluster gap=LayoutGap::Sm>
                                    <Button
                                        variant=ButtonVariant::Primary
                                        on_click=Callback::new(move |_| {
                                            state.update(|value| value.modal_open = true);
                                        })
                                    >
                                        "Open Modal"
                                    </Button>
                                    <Button variant=ButtonVariant::Secondary>"Secondary Action"</Button>
                                </Cluster>
                            </Card>
                            <Show when=move || state.get().modal_open fallback=|| ()>
                                <Modal aria_label="Neumorphic modal example">
                                    <Cluster justify=LayoutJustify::Between>
                                        <Heading role=TextRole::Title>"Pop-up modal"</Heading>
                                        <IconButton
                                            icon=IconName::Dismiss
                                            aria_label="Close modal"
                                            on_click=Callback::new(move |_| {
                                                state.update(|value| value.modal_open = false);
                                            })
                                        />
                                    </Cluster>
                                    <Text tone=TextTone::Secondary>
                                        "The modal lifts above the base plane without switching to a different visual language."
                                    </Text>
                                    <Cluster justify=LayoutJustify::Between>
                                        <Button
                                            variant=ButtonVariant::Secondary
                                            on_click=Callback::new(move |_| {
                                                state.update(|value| value.modal_open = false);
                                            })
                                        >
                                            "Dismiss"
                                        </Button>
                                        <Button
                                            variant=ButtonVariant::Primary
                                            on_click=Callback::new(move |_| {
                                                state.update(|value| value.modal_open = false);
                                            })
                                        >
                                            "Confirm"
                                        </Button>
                                    </Cluster>
                                </Modal>
                            </Show>
                        </Stack>
                    </Panel>

                    <Panel>
                        <Stack gap=LayoutGap::Md>
                            <Heading role=TextRole::Title>"Knobs and Dials"</Heading>
                            <Cluster gap=LayoutGap::Md>
                                <KnobDial
                                    value=Signal::derive(move || state.get().dial_value).get()
                                    min=0
                                    max=100
                                    aria_label="Depth dial"
                                    on_change=Callback::new(move |next| {
                                        state.update(|value| value.dial_value = next);
                                    })
                                />
                                <Stack gap=LayoutGap::Sm>
                                    <Text>"Blue glow stays inside the shared token system."</Text>
                                    <Text tone=TextTone::Secondary>
                                        {move || format!("Dial value: {}", state.get().dial_value)}
                                    </Text>
                                </Stack>
                            </Cluster>
                        </Stack>
                    </Panel>

                    <Panel>
                        <Stack gap=LayoutGap::Md>
                            <Heading role=TextRole::Title>"Inputs"</Heading>
                            <TextField
                                aria_label="Text sample"
                                placeholder="Placeholder contrast stays readable"
                                value=Signal::derive(move || text_value.get())
                                on_input=Callback::new(move |ev| {
                                    text_value.set(event_target_value(&ev));
                                })
                            />
                            <TextArea
                                aria_label="Longer text sample"
                                value=Signal::derive(move || area_value.get())
                                on_input=Callback::new(move |ev| {
                                    area_value.set(event_target_value(&ev));
                                })
                            />
                        </Stack>
                    </Panel>

                    <Panel>
                        <Stack gap=LayoutGap::Md>
                            <Heading role=TextRole::Title>"Progress"</Heading>
                            <ProgressBar max=100 value=Signal::derive(move || state.get().progress_value).get() />
                            <Cluster gap=LayoutGap::Md>
                                <CircularProgress
                                    max=100
                                    value=Signal::derive(move || state.get().progress_value).get()
                                    label=format!("{}%", state.get().progress_value)
                                />
                                <Stack gap=LayoutGap::Sm>
                                    <Text>"Linear and circular progress share the same accent grammar."</Text>
                                    <Text tone=TextTone::Secondary>
                                        {move || format!("Progress: {}%", state.get().progress_value)}
                                    </Text>
                                </Stack>
                            </Cluster>
                        </Stack>
                    </Panel>

                    <Panel>
                        <Stack gap=LayoutGap::Md>
                            <Heading role=TextRole::Title>"States and Accessibility"</Heading>
                            <Cluster gap=LayoutGap::Sm>
                                <Button variant=ButtonVariant::Primary>"Focusable"</Button>
                                <Button variant=ButtonVariant::Secondary selected=true>"Selected"</Button>
                                <Button variant=ButtonVariant::Secondary pressed=true>"Pressed"</Button>
                                <Button variant=ButtonVariant::Secondary disabled=true>"Disabled"</Button>
                            </Cluster>
                            <Text tone=TextTone::Secondary>
                                "Focus rings, keyboard navigation, and disabled states are explicit and do not rely on shadow changes alone."
                            </Text>
                        </Stack>
                    </Panel>
                </Stack>
            </Surface>

            <StatusBar>
                <StatusBarItem>{move || format!("Choice: {}", state.get().binary_choice.label())}</StatusBarItem>
                <StatusBarItem>{move || format!("Switch: {}", if state.get().switch_enabled { "On" } else { "Off" })}</StatusBarItem>
                <StatusBarItem>{move || format!("Preset: {}", state.get().dropdown_value)}</StatusBarItem>
            </StatusBar>
        </AppShell>
    }
}
