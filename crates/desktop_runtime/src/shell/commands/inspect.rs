use std::rc::Rc;

use desktop_app_contract::AppCommandRegistration;
use leptos::SignalGetUntracked;
use system_shell_contract::{
    CommandDataShape, CommandOutputShape, StructuredScalar, StructuredValue,
};

use crate::components::DesktopRuntimeContext;

pub(super) fn registrations(runtime: DesktopRuntimeContext) -> Vec<AppCommandRegistration> {
    vec![
        inspect_runtime_registration(runtime.clone()),
        inspect_windows_registration(runtime.clone()),
        inspect_storage_registration(runtime),
    ]
}

fn inspect_runtime_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: super::super::namespaced_descriptor(
            "inspect runtime",
            &[],
            "Inspect desktop runtime state.",
            "inspect runtime",
            Vec::new(),
            Vec::new(),
            system_shell_contract::CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Record),
        ),
        completion: None,
        handler: Rc::new(move |_| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let desktop = runtime.state.get_untracked();
                Ok(system_shell_contract::CommandResult {
                    output: super::super::record_data(vec![
                        super::super::int_field("windows", desktop.windows.len() as i64),
                        super::super::bool_field("start_menu_open", desktop.start_menu_open),
                        super::super::string_field("skin", desktop.theme.skin.css_id()),
                        super::super::bool_field("high_contrast", desktop.theme.high_contrast),
                        super::super::bool_field("reduced_motion", desktop.theme.reduced_motion),
                        super::super::int_field(
                            "terminal_history_len",
                            desktop.terminal_history.len() as i64,
                        ),
                    ]),
                    display: system_shell_contract::DisplayPreference::Record,
                    notices: Vec::new(),
                    cwd: None,
                    exit: system_shell_contract::ShellExit::success(),
                })
            })
        }),
    }
}

fn inspect_windows_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: super::super::namespaced_descriptor(
            "inspect windows",
            &[],
            "Inspect open window state.",
            "inspect windows",
            Vec::new(),
            Vec::new(),
            system_shell_contract::CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Table),
        ),
        completion: None,
        handler: Rc::new(move |_| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let windows = runtime.state.get_untracked().windows;
                Ok(system_shell_contract::CommandResult {
                    output: super::super::table_data(
                        vec![
                            "id".to_string(),
                            "app_id".to_string(),
                            "title".to_string(),
                            "focused".to_string(),
                            "minimized".to_string(),
                            "maximized".to_string(),
                        ],
                        windows.iter().map(super::super::window_row).collect(),
                        Some(system_shell_contract::CommandPath::new("inspect windows")),
                    ),
                    display: system_shell_contract::DisplayPreference::Table,
                    notices: Vec::new(),
                    cwd: None,
                    exit: system_shell_contract::ShellExit::success(),
                })
            })
        }),
    }
}

fn inspect_storage_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: super::super::namespaced_descriptor(
            "inspect storage",
            &[],
            "Inspect storage namespaces and host strategy.",
            "inspect storage",
            Vec::new(),
            Vec::new(),
            system_shell_contract::CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Record),
        ),
        completion: None,
        handler: Rc::new(move |_| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let namespaces = runtime
                    .host
                    .get_value()
                    .app_state_store()
                    .list_app_state_namespaces()
                    .await
                    .map_err(super::super::unavailable)?;
                Ok(system_shell_contract::CommandResult {
                    output: super::super::record_data(vec![
                        super::super::string_field(
                            "host_strategy",
                            runtime.host.get_value().host_strategy_name(),
                        ),
                        super::super::value_field(
                            "namespaces",
                            StructuredValue::List(
                                namespaces
                                    .into_iter()
                                    .map(|namespace| {
                                        StructuredValue::Scalar(StructuredScalar::String(namespace))
                                    })
                                    .collect(),
                            ),
                        ),
                    ]),
                    display: system_shell_contract::DisplayPreference::Record,
                    notices: Vec::new(),
                    cwd: None,
                    exit: system_shell_contract::ShellExit::success(),
                })
            })
        }),
    }
}
