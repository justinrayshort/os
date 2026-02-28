use super::*;

#[component]
/// Shared overlay surface for menus and popups.
pub fn MenuSurface(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional, into)] id: Option<String>,
    #[prop(optional, into)] role: Option<String>,
    #[prop(optional, into)] aria_label: Option<String>,
    #[prop(optional, into)] style: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-menu-surface", layout_class)
            id=id
            role=role
            aria-label=aria_label
            style=style
            data-ui-primitive="true"
            data-ui-kind="menu-surface"
        >
            {children()}
        </div>
    }
}

#[component]
/// Shared overlay menu item primitive.
pub fn MenuItem(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional, into)] id: Option<String>,
    #[prop(optional, into)] role: Option<String>,
    #[prop(optional, into)] aria_label: Option<String>,
    #[prop(optional, into)] aria_checked: Option<String>,
    #[prop(optional, into)] disabled: MaybeSignal<bool>,
    #[prop(optional, into)] selected: MaybeSignal<bool>,
    #[prop(optional)] on_click: Option<Callback<MouseEvent>>,
    children: Children,
) -> impl IntoView {
    view! {
        <Button
            layout_class=layout_class.unwrap_or("")
            id=id.unwrap_or_default()
            role=role.unwrap_or_default()
            aria_label=aria_label.unwrap_or_default()
            disabled=disabled
            selected=selected
            ui_slot="menu-item"
            variant=ButtonVariant::Quiet
            on_click=Callback::new(move |ev| {
                if let Some(on_click) = on_click.as_ref() {
                    on_click.call(ev);
                }
            })
        >
            <span aria-checked=aria_checked>{children()}</span>
        </Button>
    }
}

#[component]
/// Shared overlay menu separator.
pub fn MenuSeparator(#[prop(optional)] layout_class: Option<&'static str>) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-menu-separator", layout_class)
            role="separator"
            aria-hidden="true"
            data-ui-primitive="true"
            data-ui-kind="menu-separator"
        ></div>
    }
}
