use leptos::{view, IntoView, View};
use serde_json::Value;

use crate::model::{AppId, OpenWindowRequest, WindowRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppDescriptor {
    pub app_id: AppId,
    pub launcher_label: &'static str,
    pub desktop_icon_label: &'static str,
    pub show_in_launcher: bool,
    pub show_on_desktop: bool,
    pub single_instance: bool,
}

const APP_REGISTRY: [AppDescriptor; 5] = [
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
    OpenWindowRequest::new(app_id)
}

pub fn render_window_contents(window: &WindowRecord) -> View {
    match window.app_id {
        AppId::Explorer => render_explorer_placeholder(&window.launch_params),
        AppId::Notepad => render_notepad_placeholder(&window.launch_params),
        AppId::Paint => render_paint_placeholder(),
        AppId::Terminal => render_terminal_placeholder(),
        AppId::Dialup => render_dialup_placeholder(),
    }
}

fn render_explorer_placeholder(params: &Value) -> View {
    let project_slug = params
        .get("project_slug")
        .and_then(Value::as_str)
        .unwrap_or("root");
    view! {
        <div class="app app-explorer">
            <p>"Explorer placeholder"</p>
            <p>{format!("Folder/target: {project_slug}")}</p>
            <ul>
                <li>"Projects"</li>
                <li>"Notes"</li>
                <li>"About"</li>
            </ul>
        </div>
    }
    .into_view()
}

fn render_notepad_placeholder(params: &Value) -> View {
    let slug = params
        .get("slug")
        .and_then(Value::as_str)
        .unwrap_or("welcome");
    view! {
        <div class="app app-notepad">
            <p>"Notepad placeholder"</p>
            <p>{format!("Document slug: {slug}")}</p>
            <pre>"This panel will render build-time HTML for notes/posts."</pre>
        </div>
    }
    .into_view()
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

fn render_terminal_placeholder() -> View {
    view! {
        <div class="app app-terminal">
            <p>"Terminal placeholder"</p>
            <pre>
                "help\nopen notes welcome\nsearch wasm\ntheme classic"
            </pre>
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
