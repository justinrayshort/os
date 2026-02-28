//! Shared structural, control, typography, and layout primitives.

use leptos::ev::{FocusEvent, KeyboardEvent, MouseEvent};
use leptos::*;

use crate::{Icon, IconName, IconSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Semantic surface variants for structural primitives.
pub enum SurfaceVariant {
    /// Primary surface.
    Standard,
    /// Secondary or muted surface.
    Muted,
    /// Inset surface.
    Inset,
}

impl Default for SurfaceVariant {
    fn default() -> Self {
        Self::Standard
    }
}

impl SurfaceVariant {
    fn token(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Muted => "muted",
            Self::Inset => "inset",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Semantic elevation levels for shared primitives.
pub enum Elevation {
    /// Flat surface.
    Flat,
    /// Raised surface.
    Raised,
    /// Overlay surface.
    Overlay,
    /// Inset surface.
    Inset,
    /// Pressed control surface.
    Pressed,
}

impl Default for Elevation {
    fn default() -> Self {
        Self::Flat
    }
}

impl Elevation {
    fn token(self) -> &'static str {
        match self {
            Self::Flat => "flat",
            Self::Raised => "raised",
            Self::Overlay => "overlay",
            Self::Inset => "inset",
            Self::Pressed => "pressed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared button variants.
pub enum ButtonVariant {
    /// Standard action button.
    Standard,
    /// Primary emphasized action button.
    Primary,
    /// Quiet/toggle style button.
    Quiet,
    /// Accent/emphasized button.
    Accent,
    /// Danger/destructive button.
    Danger,
}

impl Default for ButtonVariant {
    fn default() -> Self {
        Self::Standard
    }
}

impl ButtonVariant {
    fn token(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Primary => "primary",
            Self::Quiet => "quiet",
            Self::Accent => "accent",
            Self::Danger => "danger",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared button sizing tokens.
pub enum ButtonSize {
    /// Dense button.
    Sm,
    /// Default button.
    Md,
    /// Large button.
    Lg,
}

impl Default for ButtonSize {
    fn default() -> Self {
        Self::Md
    }
}

impl ButtonSize {
    fn token(self) -> &'static str {
        match self {
            Self::Sm => "sm",
            Self::Md => "md",
            Self::Lg => "lg",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared input-field variants.
pub enum FieldVariant {
    /// Standard input.
    Standard,
    /// Inset/editor input.
    Inset,
}

impl Default for FieldVariant {
    fn default() -> Self {
        Self::Standard
    }
}

impl FieldVariant {
    fn token(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Inset => "inset",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared text roles.
pub enum TextRole {
    /// Body text.
    Body,
    /// Label text.
    Label,
    /// Caption text.
    Caption,
    /// Title text.
    Title,
    /// Monospace/code text.
    Code,
}

impl Default for TextRole {
    fn default() -> Self {
        Self::Body
    }
}

impl TextRole {
    fn token(self) -> &'static str {
        match self {
            Self::Body => "body",
            Self::Label => "label",
            Self::Caption => "caption",
            Self::Title => "title",
            Self::Code => "code",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared text tone.
pub enum TextTone {
    /// Primary text.
    Primary,
    /// Secondary text.
    Secondary,
    /// Accent text.
    Accent,
}

impl Default for TextTone {
    fn default() -> Self {
        Self::Primary
    }
}

impl TextTone {
    fn token(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Secondary => "secondary",
            Self::Accent => "accent",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared progress variants.
pub enum ProgressVariant {
    /// Standard progress indicator.
    Standard,
}

impl Default for ProgressVariant {
    fn default() -> Self {
        Self::Standard
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared layout gap tokens.
pub enum LayoutGap {
    /// No gap.
    None,
    /// Small gap.
    Sm,
    /// Default gap.
    Md,
    /// Large gap.
    Lg,
}

impl Default for LayoutGap {
    fn default() -> Self {
        Self::Md
    }
}

impl LayoutGap {
    fn token(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Sm => "sm",
            Self::Md => "md",
            Self::Lg => "lg",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared layout padding tokens.
pub enum LayoutPadding {
    /// No padding.
    None,
    /// Compact padding.
    Sm,
    /// Default padding.
    Md,
    /// Spacious padding.
    Lg,
}

impl Default for LayoutPadding {
    fn default() -> Self {
        Self::Md
    }
}

impl LayoutPadding {
    fn token(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Sm => "sm",
            Self::Md => "md",
            Self::Lg => "lg",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared layout alignment tokens.
pub enum LayoutAlign {
    /// Stretch/fill alignment.
    Stretch,
    /// Start alignment.
    Start,
    /// Center alignment.
    Center,
    /// End alignment.
    End,
}

impl Default for LayoutAlign {
    fn default() -> Self {
        Self::Stretch
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared layout justification tokens.
pub enum LayoutJustify {
    /// Start justification.
    Start,
    /// Center justification.
    Center,
    /// Space between items.
    Between,
    /// End justification.
    End,
}

impl Default for LayoutJustify {
    fn default() -> Self {
        Self::Start
    }
}

impl LayoutJustify {
    fn token(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Center => "center",
            Self::Between => "between",
            Self::End => "end",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared guided-step status tokens.
pub enum StepStatus {
    /// Current active step.
    Current,
    /// Completed prior step.
    Complete,
    /// Pending future step.
    Pending,
    /// Step has a validation error.
    Error,
}

impl StepStatus {
    fn token(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Complete => "complete",
            Self::Pending => "pending",
            Self::Error => "error",
        }
    }
}

impl LayoutAlign {
    fn token(self) -> &'static str {
        match self {
            Self::Stretch => "stretch",
            Self::Start => "start",
            Self::Center => "center",
            Self::End => "end",
        }
    }
}

fn merge_layout_class(base: &'static str, layout_class: Option<&'static str>) -> String {
    match layout_class {
        Some(layout_class) if !layout_class.is_empty() => format!("{base} {layout_class}"),
        _ => base.to_string(),
    }
}

fn bool_token(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

#[component]
/// Root application shell layout container.
pub fn AppShell(
    /// Layout-only class hook for app-specific grid placement.
    #[prop(optional)]
    layout_class: Option<&'static str>,
    /// Child content.
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-app-shell", layout_class)
            data-ui-primitive="true"
            data-ui-kind="app-shell"
            data-ui-variant="standard"
            data-ui-elevation="flat"
        >
            {children()}
        </div>
    }
}

#[component]
/// Shared menubar primitive.
pub fn MenuBar(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(default = LayoutGap::Sm)] gap: LayoutGap,
    #[prop(default = LayoutPadding::Sm)] padding: LayoutPadding,
    #[prop(optional)] role: Option<&'static str>,
    #[prop(optional)] aria_label: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-menubar", layout_class)
            data-ui-primitive="true"
            data-ui-kind="menubar"
            data-ui-variant="standard"
            data-ui-gap=gap.token()
            data-ui-padding=padding.token()
            role=role
            aria-label=aria_label
        >
            {children()}
        </div>
    }
}

#[component]
/// Shared toolbar primitive.
pub fn ToolBar(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(default = LayoutGap::Sm)] gap: LayoutGap,
    #[prop(default = LayoutPadding::Sm)] padding: LayoutPadding,
    #[prop(optional)] role: Option<&'static str>,
    #[prop(optional)] aria_label: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-toolbar", layout_class)
            data-ui-primitive="true"
            data-ui-kind="toolbar"
            data-ui-variant="standard"
            data-ui-gap=gap.token()
            data-ui-padding=padding.token()
            role=role
            aria-label=aria_label
        >
            {children()}
        </div>
    }
}

#[component]
/// Shared status bar primitive.
pub fn StatusBar(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(default = LayoutGap::Sm)] gap: LayoutGap,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-statusbar", layout_class)
            data-ui-primitive="true"
            data-ui-kind="statusbar"
            data-ui-variant="standard"
            data-ui-gap=gap.token()
        >
            {children()}
        </div>
    }
}

#[component]
/// Generic surface primitive.
pub fn Surface(
    #[prop(default = SurfaceVariant::Standard)] variant: SurfaceVariant,
    #[prop(default = Elevation::Flat)] elevation: Elevation,
    #[prop(default = LayoutPadding::Md)] padding: LayoutPadding,
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional)] role: Option<&'static str>,
    #[prop(optional)] aria_label: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-surface", layout_class)
            data-ui-primitive="true"
            data-ui-kind="surface"
            data-ui-variant=variant.token()
            data-ui-elevation=elevation.token()
            data-ui-padding=padding.token()
            role=role
            aria-label=aria_label
        >
            {children()}
        </div>
    }
}

#[component]
/// Generic panel primitive.
pub fn Panel(
    #[prop(default = SurfaceVariant::Standard)] variant: SurfaceVariant,
    #[prop(default = Elevation::Raised)] elevation: Elevation,
    #[prop(default = LayoutPadding::Md)] padding: LayoutPadding,
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional)] role: Option<&'static str>,
    #[prop(optional)] aria_label: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <section
            class=merge_layout_class("ui-panel", layout_class)
            data-ui-primitive="true"
            data-ui-kind="panel"
            data-ui-variant=variant.token()
            data-ui-elevation=elevation.token()
            data-ui-padding=padding.token()
            role=role
            aria-label=aria_label
        >
            {children()}
        </section>
    }
}

#[component]
/// Visual elevation layer wrapper.
pub fn ElevationLayer(
    #[prop(default = Elevation::Raised)] elevation: Elevation,
    #[prop(default = LayoutPadding::Sm)] padding: LayoutPadding,
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-layer", layout_class)
            data-ui-primitive="true"
            data-ui-kind="layer"
            data-ui-elevation=elevation.token()
            data-ui-padding=padding.token()
        >
            {children()}
        </div>
    }
}

#[component]
/// Shared button primitive with standardized states and icon slots.
pub fn Button(
    #[prop(default = ButtonVariant::Standard)] variant: ButtonVariant,
    #[prop(default = ButtonSize::Md)] size: ButtonSize,
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional)] id: Option<&'static str>,
    #[prop(optional)] role: Option<&'static str>,
    #[prop(optional)] aria_label: Option<&'static str>,
    #[prop(optional)] aria_controls: Option<&'static str>,
    #[prop(optional)] title: Option<&'static str>,
    #[prop(optional)] tabindex: Option<i32>,
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
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] title: Option<&'static str>,
    #[prop(optional)] placeholder: Option<&'static str>,
    #[prop(optional)] aria_label: Option<&'static str>,
    #[prop(optional)] node_ref: NodeRef<html::Input>,
    #[prop(optional)] autocomplete: Option<&'static str>,
    #[prop(optional)] spellcheck: Option<bool>,
    #[prop(optional)] input_type: Option<&'static str>,
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
    #[prop(optional)] id: Option<&'static str>,
    #[prop(optional)] aria_label: Option<&'static str>,
    #[prop(optional)] spellcheck: Option<&'static str>,
    #[prop(optional)] autocomplete: Option<&'static str>,
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
    #[prop(optional)] aria_label: Option<&'static str>,
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
/// Shared disclosure panel for secondary or advanced controls.
pub fn DisclosurePanel(
    /// Layout-only class hook for app-specific placement.
    #[prop(optional)]
    layout_class: Option<&'static str>,
    /// Summary text shown in the disclosure trigger.
    title: &'static str,
    /// Optional helper text under the summary label.
    #[prop(optional)]
    description: Option<&'static str>,
    /// Whether the panel is expanded.
    #[prop(optional, into)]
    expanded: MaybeSignal<bool>,
    /// Optional toggle callback when the summary is activated.
    #[prop(optional)]
    on_toggle: Option<Callback<MouseEvent>>,
    /// Child content shown when the disclosure is expanded.
    children: ChildrenFn,
) -> impl IntoView {
    view! {
        <section
            class=merge_layout_class("ui-disclosure", layout_class)
            data-ui-primitive="true"
            data-ui-kind="disclosure"
            data-ui-state=move || if expanded.get() { "open" } else { "closed" }
        >
            <button
                type="button"
                class="ui-disclosure-toggle"
                data-ui-primitive="true"
                data-ui-kind="button"
                data-ui-variant="quiet"
                data-ui-size="md"
                data-ui-state=move || if expanded.get() { "selected" } else { "idle" }
                aria-expanded=move || expanded.get()
                on:click=move |ev| {
                    if let Some(on_toggle) = on_toggle.as_ref() {
                        on_toggle.call(ev);
                    }
                }
            >
                <span class="ui-disclosure-copy">
                    <span class="ui-disclosure-title">{title}</span>
                    {description.map(|description| {
                        view! { <span class="ui-disclosure-description">{description}</span> }
                    })}
                </span>
                <span class="ui-disclosure-indicator" aria-hidden="true">
                    {move || if expanded.get() { "Hide" } else { "Show" }}
                </span>
            </button>
            <Show when=move || expanded.get() fallback=|| ()>
                <div class="ui-disclosure-body">{children()}</div>
            </Show>
        </section>
    }
}

#[component]
/// Shared range-field primitive.
pub fn RangeField(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional)] min: Option<&'static str>,
    #[prop(optional)] max: Option<&'static str>,
    #[prop(optional)] aria_label: Option<&'static str>,
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
    #[prop(optional)] aria_label: Option<&'static str>,
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
            data-ui-variant="standard"
        ></progress>
    }
}

#[component]
/// Shared text primitive.
pub fn Text(
    #[prop(default = TextRole::Body)] role: TextRole,
    #[prop(default = TextTone::Primary)] tone: TextTone,
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <span
            class=merge_layout_class("ui-text", layout_class)
            data-ui-primitive="true"
            data-ui-kind="text"
            data-ui-variant=role.token()
            data-ui-tone=tone.token()
        >
            {children()}
        </span>
    }
}

#[component]
/// Shared heading primitive.
pub fn Heading(
    #[prop(default = TextRole::Title)] role: TextRole,
    #[prop(default = TextTone::Primary)] tone: TextTone,
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-heading", layout_class)
            data-ui-primitive="true"
            data-ui-kind="heading"
            data-ui-variant=role.token()
            data-ui-tone=tone.token()
        >
            {children()}
        </div>
    }
}

#[component]
/// Root container for a guided multi-step flow.
pub fn StepFlow(
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <section
            class=merge_layout_class("ui-step-flow", layout_class)
            data-ui-primitive="true"
            data-ui-kind="step-flow"
        >
            {children()}
        </section>
    }
}

#[component]
/// Header section for a guided multi-step flow.
pub fn StepFlowHeader(
    #[prop(optional)] layout_class: Option<&'static str>,
    title: &'static str,
    #[prop(optional)] description: Option<&'static str>,
) -> impl IntoView {
    view! {
        <header
            class=merge_layout_class("ui-step-flow-header", layout_class)
            data-ui-primitive="true"
            data-ui-kind="step-flow-header"
        >
            <div class="ui-step-flow-title">{title}</div>
            {description.map(|description| {
                view! { <div class="ui-step-flow-description">{description}</div> }
            })}
        </header>
    }
}

#[component]
/// Individual step block within a guided flow.
pub fn StepFlowStep(
    #[prop(optional)] layout_class: Option<&'static str>,
    title: &'static str,
    #[prop(optional)] description: Option<&'static str>,
    #[prop(into)] status: MaybeSignal<StepStatus>,
    children: Children,
) -> impl IntoView {
    view! {
        <section
            class=merge_layout_class("ui-step-flow-step", layout_class)
            data-ui-primitive="true"
            data-ui-kind="step-flow-step"
            data-ui-state=move || status.get().token()
        >
            <div class="ui-step-flow-step-header">
                <span class="ui-step-flow-step-badge">{move || status.get().token()}</span>
                <div class="ui-step-flow-step-copy">
                    <div class="ui-step-flow-step-title">{title}</div>
                    {description.map(|description| {
                        view! { <div class="ui-step-flow-step-description">{description}</div> }
                    })}
                </div>
            </div>
            <div class="ui-step-flow-step-body">{children()}</div>
        </section>
    }
}

#[component]
/// Shared action row for guided flows.
pub fn StepFlowActions(
    #[prop(default = LayoutJustify::Between)] justify: LayoutJustify,
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-step-flow-actions", layout_class)
            data-ui-primitive="true"
            data-ui-kind="step-flow-actions"
            data-ui-justify=justify.token()
        >
            {children()}
        </div>
    }
}

#[component]
/// Vertical layout stack.
pub fn Stack(
    #[prop(default = LayoutGap::Md)] gap: LayoutGap,
    #[prop(default = LayoutAlign::Stretch)] align: LayoutAlign,
    #[prop(default = LayoutPadding::None)] padding: LayoutPadding,
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-stack", layout_class)
            data-ui-primitive="true"
            data-ui-kind="stack"
            data-ui-gap=gap.token()
            data-ui-align=align.token()
            data-ui-padding=padding.token()
        >
            {children()}
        </div>
    }
}

#[component]
/// Horizontal wrapping cluster.
pub fn Cluster(
    #[prop(default = LayoutGap::Md)] gap: LayoutGap,
    #[prop(default = LayoutAlign::Center)] align: LayoutAlign,
    #[prop(default = LayoutJustify::Start)] justify: LayoutJustify,
    #[prop(default = LayoutPadding::None)] padding: LayoutPadding,
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-cluster", layout_class)
            data-ui-primitive="true"
            data-ui-kind="cluster"
            data-ui-gap=gap.token()
            data-ui-align=align.token()
            data-ui-justify=justify.token()
            data-ui-padding=padding.token()
        >
            {children()}
        </div>
    }
}

#[component]
/// Grid layout primitive.
pub fn Grid(
    #[prop(default = LayoutGap::Md)] gap: LayoutGap,
    #[prop(default = LayoutPadding::None)] padding: LayoutPadding,
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-grid", layout_class)
            data-ui-primitive="true"
            data-ui-kind="grid"
            data-ui-gap=gap.token()
            data-ui-padding=padding.token()
        >
            {children()}
        </div>
    }
}
