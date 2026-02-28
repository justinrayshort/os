use super::*;

#[component]
/// Shared button primitive with standardized states and icon slots.
pub fn Button(
    #[prop(default = ButtonVariant::Standard)] variant: ButtonVariant,
    #[prop(default = ButtonSize::Md)] size: ButtonSize,
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional, into)] id: Option<String>,
    #[prop(optional, into)] role: Option<String>,
    #[prop(optional, into)] aria_label: Option<String>,
    #[prop(optional, into)] aria_controls: Option<String>,
    #[prop(optional, into)] title: Option<String>,
    #[prop(optional)] tabindex: Option<i32>,
    #[prop(optional)] ui_slot: Option<&'static str>,
    #[prop(optional, into)] disabled: MaybeSignal<bool>,
    #[prop(optional, into)] selected: MaybeSignal<bool>,
    #[prop(optional, into)] pressed: MaybeSignal<bool>,
    #[prop(optional)] leading_icon: Option<IconName>,
    #[prop(optional)] trailing_icon: Option<IconName>,
    #[prop(optional)] on_click: Option<Callback<MouseEvent>>,
    #[prop(optional)] on_keydown: Option<Callback<KeyboardEvent>>,
    children: Children,
) -> impl IntoView {
    let class = merge_layout_class("ui-button", layout_class);
    view! {
        <button
            type="button"
            class=class
            id=id
            role=role
            aria-label=aria_label
            aria-controls=aria_controls
            title=title
            tabindex=tabindex
            disabled=move || disabled.get()
            data-ui-primitive="true"
            data-ui-kind="button"
            data-ui-slot=ui_slot
            data-ui-variant=variant.token()
            data-ui-size=size.token()
            data-ui-state=move || {
                if pressed.get() {
                    "pressed"
                } else if selected.get() {
                    "selected"
                } else {
                    "idle"
                }
            }
            data-ui-selected=move || bool_token(selected.get())
            data-ui-pressed=move || bool_token(pressed.get())
            data-ui-disabled=move || bool_token(disabled.get())
            on:click=move |ev| {
                if let Some(on_click) = on_click.as_ref() {
                    on_click.call(ev);
                }
            }
            on:keydown=move |ev| {
                if let Some(on_keydown) = on_keydown.as_ref() {
                    on_keydown.call(ev);
                }
            }
        >
            {leading_icon.map(|icon| view! { <Icon icon size=IconSize::Sm /> })}
            {children()}
            {trailing_icon.map(|icon| view! { <Icon icon size=IconSize::Sm /> })}
        </button>
    }
}

#[component]
/// Shared text input primitive.
pub fn TextField(
    #[prop(default = FieldVariant::Standard)] variant: FieldVariant,
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional, into)] id: Option<String>,
    #[prop(optional, into)] title: Option<String>,
    #[prop(optional, into)] placeholder: Option<String>,
    #[prop(optional, into)] aria_label: Option<String>,
    #[prop(optional)] node_ref: NodeRef<html::Input>,
    #[prop(optional)] autocomplete: Option<&'static str>,
    #[prop(optional)] spellcheck: Option<bool>,
    #[prop(optional)] input_type: Option<&'static str>,
    #[prop(optional)] ui_slot: Option<&'static str>,
    #[prop(optional, into)] value: MaybeSignal<String>,
    #[prop(optional, into)] disabled: MaybeSignal<bool>,
    #[prop(optional)] on_input: Option<Callback<web_sys::Event>>,
    #[prop(optional)] on_keydown: Option<Callback<KeyboardEvent>>,
    #[prop(optional)] on_focus: Option<Callback<FocusEvent>>,
    #[prop(optional)] on_blur: Option<Callback<FocusEvent>>,
) -> impl IntoView {
    view! {
        <input
            class=merge_layout_class("ui-field", layout_class)
            id=id
            title=title
            placeholder=placeholder
            aria-label=aria_label
            node_ref=node_ref
            autocomplete=autocomplete
            spellcheck=spellcheck
            type=input_type.unwrap_or("text")
            prop:value=move || value.get()
            disabled=move || disabled.get()
            data-ui-primitive="true"
            data-ui-kind="text-field"
            data-ui-slot=ui_slot
            data-ui-variant=variant.token()
            data-ui-disabled=move || bool_token(disabled.get())
            on:input=move |ev| {
                if let Some(on_input) = on_input.as_ref() {
                    on_input.call(ev);
                }
            }
            on:keydown=move |ev| {
                if let Some(on_keydown) = on_keydown.as_ref() {
                    on_keydown.call(ev);
                }
            }
            on:focus=move |ev| {
                if let Some(on_focus) = on_focus.as_ref() {
                    on_focus.call(ev);
                }
            }
            on:blur=move |ev| {
                if let Some(on_blur) = on_blur.as_ref() {
                    on_blur.call(ev);
                }
            }
        />
    }
}

#[component]
/// Shared multiline text area primitive.
pub fn TextArea(
    #[prop(default = FieldVariant::Inset)] variant: FieldVariant,
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional, into)] id: Option<String>,
    #[prop(optional, into)] aria_label: Option<String>,
    #[prop(optional)] spellcheck: Option<&'static str>,
    #[prop(optional)] autocomplete: Option<&'static str>,
    #[prop(optional)] ui_slot: Option<&'static str>,
    #[prop(optional, into)] value: MaybeSignal<String>,
    #[prop(optional)] on_input: Option<Callback<web_sys::Event>>,
    #[prop(optional)] on_keydown: Option<Callback<KeyboardEvent>>,
) -> impl IntoView {
    view! {
        <textarea
            class=merge_layout_class("ui-textarea", layout_class)
            id=id
            aria-label=aria_label
            spellcheck=spellcheck.unwrap_or("false")
            autocomplete=autocomplete.unwrap_or("off")
            prop:value=move || value.get()
            data-ui-primitive="true"
            data-ui-kind="text-area"
            data-ui-slot=ui_slot
            data-ui-variant=variant.token()
            on:input=move |ev| {
                if let Some(on_input) = on_input.as_ref() {
                    on_input.call(ev);
                }
            }
            on:keydown=move |ev| {
                if let Some(on_keydown) = on_keydown.as_ref() {
                    on_keydown.call(ev);
                }
            }
        ></textarea>
    }
}

#[component]
/// Shared select-field primitive.
pub fn SelectField(
    #[prop(default = FieldVariant::Standard)] variant: FieldVariant,
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional, into)] aria_label: Option<String>,
    #[prop(optional)] ui_slot: Option<&'static str>,
    #[prop(optional, into)] value: MaybeSignal<String>,
    #[prop(optional)] on_change: Option<Callback<web_sys::Event>>,
    children: Children,
) -> impl IntoView {
    view! {
        <select
            class=merge_layout_class("ui-field", layout_class)
            aria-label=aria_label
            prop:value=move || value.get()
            data-ui-primitive="true"
            data-ui-kind="select"
            data-ui-slot=ui_slot
            data-ui-variant=variant.token()
            on:change=move |ev| {
                if let Some(on_change) = on_change.as_ref() {
                    on_change.call(ev);
                }
            }
        >
            {children()}
        </select>
    }
}

#[component]
/// Shared range-field primitive.
pub fn RangeField(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional)] min: Option<&'static str>,
    #[prop(optional)] max: Option<&'static str>,
    #[prop(optional, into)] aria_label: Option<String>,
    #[prop(optional)] ui_slot: Option<&'static str>,
    #[prop(optional, into)] value: MaybeSignal<String>,
    #[prop(optional)] on_input: Option<Callback<web_sys::Event>>,
) -> impl IntoView {
    view! {
        <input
            class=merge_layout_class("ui-field", layout_class)
            type="range"
            min=min
            max=max
            aria-label=aria_label
            prop:value=move || value.get()
            data-ui-primitive="true"
            data-ui-kind="range"
            data-ui-slot=ui_slot
            data-ui-variant="standard"
            on:input=move |ev| {
                if let Some(on_input) = on_input.as_ref() {
                    on_input.call(ev);
                }
            }
        />
    }
}

#[component]
/// Shared color-field primitive.
pub fn ColorField(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional, into)] aria_label: Option<String>,
    #[prop(optional)] ui_slot: Option<&'static str>,
    #[prop(optional, into)] value: MaybeSignal<String>,
    #[prop(optional)] on_input: Option<Callback<web_sys::Event>>,
) -> impl IntoView {
    view! {
        <input
            class=merge_layout_class("ui-field", layout_class)
            type="color"
            aria-label=aria_label
            prop:value=move || value.get()
            data-ui-primitive="true"
            data-ui-kind="color-field"
            data-ui-slot=ui_slot
            data-ui-variant="standard"
            on:input=move |ev| {
                if let Some(on_input) = on_input.as_ref() {
                    on_input.call(ev);
                }
            }
        />
    }
}

#[component]
/// Shared progress indicator primitive.
pub fn ProgressBar(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(default = ProgressVariant::Standard)] _variant: ProgressVariant,
    #[prop(optional)] ui_slot: Option<&'static str>,
    max: u16,
    value: u16,
) -> impl IntoView {
    view! {
        <progress
            class=merge_layout_class("ui-progress", layout_class)
            max=max
            value=value
            data-ui-primitive="true"
            data-ui-kind="progress"
            data-ui-slot=ui_slot
            data-ui-variant="standard"
        ></progress>
    }
}

#[component]
/// Shared completion list item.
pub fn CompletionItem(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional)] on_click: Option<Callback<MouseEvent>>,
    children: Children,
) -> impl IntoView {
    view! {
        <Button
            layout_class=layout_class.unwrap_or("")
            ui_slot="completion-item"
            variant=ButtonVariant::Quiet
            on_click=Callback::new(move |ev| {
                if let Some(on_click) = on_click.as_ref() {
                    on_click.call(ev);
                }
            })
        >
            {children()}
        </Button>
    }
}

#[component]
/// Shared completion list surface.
pub fn CompletionList(
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-completion-list", layout_class)
            data-ui-primitive="true"
            data-ui-kind="completion-list"
        >
            {children()}
        </div>
    }
}

#[component]
/// Shared labeled toggle row.
pub fn ToggleRow(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional, into)] title: Option<String>,
    #[prop(optional, into)] description: Option<String>,
    #[prop(optional, into)] checked: MaybeSignal<bool>,
    children: Children,
) -> impl IntoView {
    view! {
        <label
            class=merge_layout_class("ui-toggle-row", layout_class)
            data-ui-primitive="true"
            data-ui-kind="toggle-row"
            data-ui-selected=move || bool_token(checked.get())
        >
            <span data-ui-slot="copy">
                {title.map(|title| view! { <span data-ui-slot="title">{title}</span> })}
                {description.map(|description| view! { <span data-ui-slot="description">{description}</span> })}
            </span>
            <span data-ui-slot="control">{children()}</span>
        </label>
    }
}
