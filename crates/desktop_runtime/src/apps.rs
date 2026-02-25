use crate::model::{AppId, OpenWindowRequest, WindowRecord};
use desktop_app_calculator::CalculatorApp;
use desktop_app_explorer::ExplorerApp;
use desktop_app_notepad::NotepadApp;
use desktop_app_terminal::TerminalApp;
use leptos::{view, IntoView, View};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppDescriptor {
    pub app_id: AppId,
    pub launcher_label: &'static str,
    pub desktop_icon_label: &'static str,
    pub show_in_launcher: bool,
    pub show_on_desktop: bool,
    pub single_instance: bool,
}

const APP_REGISTRY: [AppDescriptor; 6] = [
    AppDescriptor {
        app_id: AppId::Calculator,
        launcher_label: "Calculator",
        desktop_icon_label: "Calculator",
        show_in_launcher: true,
        show_on_desktop: true,
        single_instance: true,
    },
    AppDescriptor {
        app_id: AppId::Explorer,
        launcher_label: "Explorer",
        desktop_icon_label: "Explorer",
        show_in_launcher: true,
        show_on_desktop: true,
        single_instance: false,
    },
    AppDescriptor {
        app_id: AppId::Notepad,
        launcher_label: "Notepad",
        desktop_icon_label: "Notes",
        show_in_launcher: true,
        show_on_desktop: false,
        single_instance: false,
    },
    AppDescriptor {
        app_id: AppId::Paint,
        launcher_label: "Paint",
        desktop_icon_label: "Paint",
        show_in_launcher: true,
        show_on_desktop: false,
        single_instance: false,
    },
    AppDescriptor {
        app_id: AppId::Terminal,
        launcher_label: "Terminal",
        desktop_icon_label: "Terminal",
        show_in_launcher: true,
        show_on_desktop: true,
        single_instance: true,
    },
    AppDescriptor {
        app_id: AppId::Dialup,
        launcher_label: "Dial-up",
        desktop_icon_label: "Connect",
        show_in_launcher: true,
        show_on_desktop: false,
        single_instance: false,
    },
];

pub fn app_registry() -> &'static [AppDescriptor] {
    &APP_REGISTRY
}

pub fn launcher_apps() -> Vec<AppDescriptor> {
    app_registry()
        .iter()
        .copied()
        .filter(|entry| entry.show_in_launcher)
        .collect()
}

pub fn desktop_icon_apps() -> Vec<AppDescriptor> {
    app_registry()
        .iter()
        .copied()
        .filter(|entry| entry.show_on_desktop)
        .collect()
}

pub fn app_descriptor(app_id: AppId) -> &'static AppDescriptor {
    app_registry()
        .iter()
        .find(|entry| entry.app_id == app_id)
        .expect("app descriptor exists")
}

pub fn default_open_request(app_id: AppId) -> OpenWindowRequest {
    let mut req = OpenWindowRequest::new(app_id);
    if matches!(app_id, AppId::Calculator) {
        req.rect = Some(crate::model::WindowRect {
            x: 72,
            y: 64,
            w: 560,
            h: 420,
        });
    }
    req
}

pub fn render_window_contents(window: &WindowRecord) -> View {
    match window.app_id {
        AppId::Calculator => {
            view! { <CalculatorApp launch_params=window.launch_params.clone() /> }.into_view()
        }
        AppId::Explorer => {
            view! { <ExplorerApp launch_params=window.launch_params.clone() /> }.into_view()
        }
        AppId::Notepad => {
            view! { <NotepadApp launch_params=window.launch_params.clone() /> }.into_view()
        }
        AppId::Paint => render_paint_placeholder(),
        AppId::Terminal => {
            view! { <TerminalApp launch_params=window.launch_params.clone() /> }.into_view()
        }
        AppId::Dialup => render_dialup_placeholder(),
    }
}

fn render_paint_placeholder() -> View {
    view! {
        <div class="app app-paint">
            <p>"Paint placeholder"</p>
            <p>"Canvas + IndexedDB save slots land here in Phase 3."</p>
        </div>
    }
    .into_view()
}

fn render_dialup_placeholder() -> View {
    view! {
        <div class="app app-dialup">
            <p>"Dial-up placeholder"</p>
            <p>"Negotiating connection..."</p>
            <progress max="100" value="45"></progress>
        </div>
    }
    .into_view()
}
