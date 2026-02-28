use super::*;

#[component]
/// Root application shell layout container.
pub fn AppShell(
    #[prop(optional)] layout_class: Option<&'static str>,
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
/// Root desktop shell primitive.
pub fn DesktopRoot(
    #[prop(optional, into)] id: Option<String>,
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional)] tabindex: Option<i32>,
    #[prop(optional)] ui_slot: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            id=id
            class=merge_layout_class("desktop-shell", layout_class)
            tabindex=tabindex
            data-ui-primitive="true"
            data-ui-kind="desktop-root"
            data-ui-slot=ui_slot
        >
            {children()}
        </div>
    }
}

#[component]
/// Desktop wallpaper and backdrop host.
pub fn DesktopBackdrop(
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("desktop-backdrop", layout_class)
            data-ui-primitive="true"
            data-ui-kind="desktop-backdrop"
        >
            {children()}
        </div>
    }
}

#[component]
/// Desktop icon grid.
pub fn DesktopIconGrid(
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-desktop-icon-grid", layout_class)
            data-ui-primitive="true"
            data-ui-kind="desktop-icon-grid"
        >
            {children()}
        </div>
    }
}

#[component]
/// Desktop icon launcher button.
pub fn DesktopIconButton(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional, into)] title: Option<String>,
    #[prop(optional, into)] aria_label: Option<String>,
    #[prop(optional)] on_click: Option<Callback<MouseEvent>>,
    #[prop(optional)] on_dblclick: Option<Callback<MouseEvent>>,
    children: Children,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class=merge_layout_class("ui-desktop-icon-button", layout_class)
            title=title
            aria-label=aria_label
            data-ui-primitive="true"
            data-ui-kind="desktop-icon-button"
            on:click=move |ev| {
                if let Some(on_click) = on_click.as_ref() {
                    on_click.call(ev);
                }
            }
            on:dblclick=move |ev| {
                if let Some(on_dblclick) = on_dblclick.as_ref() {
                    on_dblclick.call(ev);
                }
            }
        >
            {children()}
        </button>
    }
}

#[component]
/// Window stack host.
pub fn DesktopWindowLayer(
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-window-layer", layout_class)
            data-ui-primitive="true"
            data-ui-kind="desktop-window-layer"
        >
            {children()}
        </div>
    }
}

#[component]
/// Shared window frame primitive.
pub fn WindowFrame(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional, into)] style: Option<String>,
    #[prop(optional, into)] aria_label: Option<String>,
    #[prop(optional, into)] focused: MaybeSignal<bool>,
    #[prop(optional, into)] minimized: MaybeSignal<bool>,
    #[prop(optional, into)] maximized: MaybeSignal<bool>,
    children: Children,
) -> impl IntoView {
    view! {
        <section
            class=merge_layout_class("ui-window-frame", layout_class)
            style=style
            role="dialog"
            aria-label=aria_label
            data-ui-primitive="true"
            data-ui-kind="window-frame"
            data-ui-focused=move || bool_token(focused.get())
            data-ui-minimized=move || bool_token(minimized.get())
            data-ui-maximized=move || bool_token(maximized.get())
        >
            {children()}
        </section>
    }
}

#[component]
/// Shared window titlebar primitive.
pub fn WindowTitleBar(
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <header
            class=merge_layout_class("ui-window-titlebar", layout_class)
            data-ui-primitive="true"
            data-ui-kind="window-titlebar"
        >
            {children()}
        </header>
    }
}

#[component]
/// Shared window title group.
pub fn WindowTitle(
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-window-title", layout_class)
            data-ui-primitive="true"
            data-ui-kind="window-title"
        >
            {children()}
        </div>
    }
}

#[component]
/// Shared titlebar controls row.
pub fn WindowControls(
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-window-controls", layout_class)
            data-ui-primitive="true"
            data-ui-kind="window-controls"
        >
            {children()}
        </div>
    }
}

#[component]
/// Shared titlebar control button.
pub fn WindowControlButton(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional, into)] aria_label: Option<String>,
    #[prop(optional, into)] disabled: MaybeSignal<bool>,
    #[prop(optional)] on_click: Option<Callback<MouseEvent>>,
    children: Children,
) -> impl IntoView {
    view! {
        <Button
            layout_class=layout_class.unwrap_or("")
            aria_label=aria_label.unwrap_or_default()
            disabled=disabled
            ui_slot="window-control"
            variant=ButtonVariant::Quiet
            size=ButtonSize::Sm
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
/// Shared window body primitive.
pub fn WindowBody(
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-window-body", layout_class)
            data-ui-primitive="true"
            data-ui-kind="window-body"
        >
            {children()}
        </div>
    }
}

#[component]
/// Shared resize handle primitive.
pub fn ResizeHandle(
    edge: &'static str,
    #[prop(optional)] layout_class: Option<&'static str>,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-resize-handle", layout_class)
            data-ui-primitive="true"
            data-ui-kind="resize-handle"
            data-ui-slot=edge
        ></div>
    }
}

#[component]
/// Shared taskbar root.
pub fn Taskbar(
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <footer
            class=merge_layout_class("ui-taskbar", layout_class)
            data-ui-primitive="true"
            data-ui-kind="taskbar"
        >
            {children()}
        </footer>
    }
}

#[component]
/// Shared taskbar section.
pub fn TaskbarSection(
    ui_slot: &'static str,
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-taskbar-section", layout_class)
            data-ui-primitive="true"
            data-ui-kind="taskbar-section"
            data-ui-slot=ui_slot
        >
            {children()}
        </div>
    }
}

#[component]
/// Shared taskbar button.
pub fn TaskbarButton(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional, into)] aria_label: Option<String>,
    #[prop(optional, into)] title: Option<String>,
    #[prop(optional, into)] selected: MaybeSignal<bool>,
    #[prop(optional, into)] pressed: MaybeSignal<bool>,
    #[prop(optional)] on_click: Option<Callback<MouseEvent>>,
    children: Children,
) -> impl IntoView {
    view! {
        <Button
            layout_class=layout_class.unwrap_or("")
            aria_label=aria_label.unwrap_or_default()
            title=title.unwrap_or_default()
            selected=selected
            pressed=pressed
            ui_slot="taskbar-button"
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
/// Shared taskbar overflow button.
pub fn TaskbarOverflowButton(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional, into)] aria_label: Option<String>,
    #[prop(optional)] on_click: Option<Callback<MouseEvent>>,
    children: Children,
) -> impl IntoView {
    view! {
        <TaskbarButton
            layout_class=layout_class.unwrap_or("")
            aria_label=aria_label.unwrap_or_default()
            on_click=Callback::new(move |ev| {
                if let Some(on_click) = on_click.as_ref() {
                    on_click.call(ev);
                }
            })
        >
            {children()}
        </TaskbarButton>
    }
}

#[component]
/// Shared tray list container.
pub fn TrayList(
    #[prop(optional)] layout_class: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class=merge_layout_class("ui-tray-list", layout_class)
            data-ui-primitive="true"
            data-ui-kind="tray-list"
        >
            {children()}
        </div>
    }
}

#[component]
/// Shared tray button.
pub fn TrayButton(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional, into)] aria_label: Option<String>,
    #[prop(optional, into)] pressed: MaybeSignal<bool>,
    #[prop(optional)] on_click: Option<Callback<MouseEvent>>,
    children: Children,
) -> impl IntoView {
    view! {
        <TaskbarButton
            layout_class=layout_class.unwrap_or("")
            aria_label=aria_label.unwrap_or_default()
            pressed=pressed
            on_click=Callback::new(move |ev| {
                if let Some(on_click) = on_click.as_ref() {
                    on_click.call(ev);
                }
            })
        >
            {children()}
        </TaskbarButton>
    }
}

#[component]
/// Shared taskbar clock button.
pub fn ClockButton(
    #[prop(optional)] layout_class: Option<&'static str>,
    #[prop(optional, into)] aria_label: Option<String>,
    #[prop(optional)] on_click: Option<Callback<MouseEvent>>,
    children: Children,
) -> impl IntoView {
    view! {
        <TaskbarButton
            layout_class=layout_class.unwrap_or("")
            aria_label=aria_label.unwrap_or_default()
            on_click=Callback::new(move |ev| {
                if let Some(on_click) = on_click.as_ref() {
                    on_click.call(ev);
                }
            })
        >
            {children()}
        </TaskbarButton>
    }
}
