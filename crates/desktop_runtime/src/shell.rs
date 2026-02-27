//! Shell runtime integration for the browser-native system terminal.

use std::rc::Rc;

use desktop_app_contract::{
    AppCapability, AppCommandContext, AppCommandProvider, AppCommandRegistration,
    CommandRegistrationHandle as AppCommandRegistrationHandle, CommandService, ShellSessionHandle,
};
use leptos::SignalGetUntracked;
use serde::Serialize;
use serde_json::{json, Value};
use system_shell::{CommandExecutionContext, CommandRegistryHandle};
use system_shell_contract::{
    CommandArgSpec, CommandDescriptor, CommandExample, CommandId, CommandOptionSpec, CommandScope,
    CommandVisibility, CompletionItem, CompletionRequest, HelpDoc, ShellError, ShellErrorCode,
    ShellExit, ShellRequest, ShellStreamEvent,
};

use crate::{
    apps,
    components::DesktopRuntimeContext,
    model::{AppId, DesktopSkin, WindowId},
    reducer::DesktopAction,
};

const TASKBAR_HEIGHT_PX: i32 = 38;

/// Builds a command service for one mounted window/app.
pub fn build_command_service(
    runtime: DesktopRuntimeContext,
    app_id: AppId,
    window_id: WindowId,
    history: leptos::ReadSignal<Vec<String>>,
) -> CommandService {
    CommandService::new(
        history,
        Rc::new({
            let runtime = runtime.clone();
            move |cwd| {
                let session = runtime.shell_engine.get_value().new_session(cwd);
                let submit_session = session.clone();
                let cancel_session = session.clone();
                let complete_session = session.clone();
                Ok(ShellSessionHandle::new(
                    session.events(),
                    session.active_execution(),
                    session.cwd(),
                    Rc::new({
                        let runtime = runtime;
                        move |request: ShellRequest| {
                            runtime.dispatch_action(DesktopAction::PushTerminalHistory {
                                command: request.line.clone(),
                            });
                            submit_session.submit(request);
                        }
                    }),
                    Rc::new(move || cancel_session.cancel()),
                    Rc::new(move |request: CompletionRequest| {
                        let complete_session = complete_session.clone();
                        Box::pin(async move { complete_session.complete(request).await })
                    }),
                ))
            }
        }),
        Rc::new({
            let runtime = runtime.clone();
            move |registration| {
                register_app_command(runtime.clone(), app_id, window_id, registration)
            }
        }),
        Rc::new({
            let runtime = runtime.clone();
            move |provider: Rc<dyn AppCommandProvider>| {
                let mut handles = Vec::new();
                for registration in provider.commands() {
                    handles.push(register_app_command(
                        runtime.clone(),
                        app_id,
                        window_id,
                        registration,
                    )?);
                }
                Ok(AppCommandRegistrationHandle::new(Rc::new(move || {
                    for handle in &handles {
                        handle.unregister();
                    }
                })))
            }
        }),
    )
}

/// Registers runtime-owned built-in commands and returns the owning handles.
pub fn register_builtin_commands(runtime: DesktopRuntimeContext) -> Vec<CommandRegistryHandle> {
    let mut handles = Vec::new();
    let engine = runtime.shell_engine.get_value();
    for registration in builtin_registrations(runtime) {
        let descriptor = registration.descriptor.clone();
        let handler = registration.handler.clone();
        handles.push(engine.register_command(
            registration.descriptor,
            registration.completion,
            Rc::new(move |context: CommandExecutionContext| {
                let app_context = adapt_context(context, descriptor.clone());
                handler(app_context)
            }),
        ));
    }
    handles
}

fn register_app_command(
    runtime: DesktopRuntimeContext,
    app_id: AppId,
    window_id: WindowId,
    registration: AppCommandRegistration,
) -> Result<AppCommandRegistrationHandle, String> {
    if !app_can_register_commands(app_id) {
        return Err(format!(
            "{} is not allowed to register system commands",
            app_id.canonical_id()
        ));
    }
    validate_scope(&registration.descriptor.scope, app_id, window_id)?;
    let completion = registration.completion.clone();
    let handler = registration.handler.clone();
    let descriptor = registration.descriptor.clone();
    let system_handle = runtime.shell_engine.get_value().register_command(
        registration.descriptor,
        completion.map(|completion| {
            Rc::new(move |request| completion(request)) as system_shell::CompletionHandler
        }),
        Rc::new(move |context: CommandExecutionContext| {
            let app_context = adapt_context(context, descriptor.clone());
            handler(app_context)
        }),
    );
    Ok(AppCommandRegistrationHandle::new(Rc::new(move || {
        system_handle.unregister();
    })))
}

fn adapt_context(
    context: CommandExecutionContext,
    _descriptor: CommandDescriptor,
) -> AppCommandContext {
    let emit_context = context.clone();
    let set_cwd_context = context.clone();
    let cancel_context = context.clone();
    AppCommandContext::new(
        context.execution_id,
        context.argv.clone(),
        context.cwd.clone(),
        context.source_window_id,
        Rc::new(move |event| emit_shell_event(&emit_context, event)),
        Rc::new(move |cwd| set_cwd_context.set_cwd(cwd)),
        Rc::new(move || cancel_context.is_cancelled()),
    )
}

fn emit_shell_event(context: &CommandExecutionContext, event: ShellStreamEvent) {
    match event {
        ShellStreamEvent::StdoutChunk { text, .. } => context.stdout(text),
        ShellStreamEvent::StderrChunk { text, .. } => context.stderr(text),
        ShellStreamEvent::Status { text, .. } => context.status(text),
        ShellStreamEvent::Json { value, .. } => context.json(value),
        _ => {}
    }
}

fn app_can_register_commands(app_id: AppId) -> bool {
    apps::app_is_privileged(app_id)
        || apps::app_requested_capabilities(app_id)
            .iter()
            .any(|cap| *cap == AppCapability::Commands)
}

fn validate_scope(scope: &CommandScope, app_id: AppId, window_id: WindowId) -> Result<(), String> {
    match scope {
        CommandScope::Global if apps::app_is_privileged(app_id) => Ok(()),
        CommandScope::Global => {
            Err("only privileged apps may register global commands".to_string())
        }
        CommandScope::App { app_id: owner } if owner == app_id.canonical_id() => Ok(()),
        CommandScope::App { .. } => Err("app-scoped command owner mismatch".to_string()),
        CommandScope::Window { window_id: owner } if *owner == window_id.0 => Ok(()),
        CommandScope::Window { .. } => Err("window-scoped command owner mismatch".to_string()),
    }
}

fn builtin_registrations(runtime: DesktopRuntimeContext) -> Vec<AppCommandRegistration> {
    vec![
        help_registration(runtime.clone()),
        clear_registration(),
        history_registration(runtime.clone()),
        open_registration(runtime.clone()),
        apps_list_registration(runtime.clone()),
        apps_open_registration(runtime.clone()),
        windows_list_registration(runtime.clone()),
        windows_focus_registration(runtime.clone()),
        windows_close_registration(runtime.clone()),
        windows_minimize_registration(runtime.clone()),
        windows_restore_registration(runtime.clone()),
        theme_show_registration(runtime.clone()),
        theme_set_skin_registration(runtime.clone()),
        theme_set_high_contrast_registration(runtime.clone()),
        theme_set_reduced_motion_registration(runtime.clone()),
        config_get_registration(),
        config_set_registration(),
        inspect_runtime_registration(runtime.clone()),
        inspect_windows_registration(runtime.clone()),
        inspect_storage_registration(),
        fs_pwd_registration(),
        fs_cd_registration(),
        fs_ls_registration(),
    ]
}

fn descriptor(
    path: &str,
    aliases: &[&str],
    summary: &str,
    usage: &str,
    args: Vec<CommandArgSpec>,
    options: Vec<CommandOptionSpec>,
    examples: Vec<CommandExample>,
) -> CommandDescriptor {
    CommandDescriptor {
        id: CommandId::new(path),
        path: system_shell_contract::CommandPath::new(path),
        aliases: aliases.iter().map(|alias| alias.to_string()).collect(),
        scope: CommandScope::Global,
        visibility: CommandVisibility::Public,
        args,
        options,
        help: HelpDoc {
            summary: summary.to_string(),
            description: None,
            usage: usage.to_string(),
            examples,
        },
    }
}

fn usage_error(message: impl Into<String>) -> ShellError {
    ShellError::new(ShellErrorCode::Usage, message)
}

fn unavailable(message: impl Into<String>) -> ShellError {
    ShellError::new(ShellErrorCode::Unavailable, message)
}

fn stdout_json<T: Serialize>(context: &AppCommandContext, value: &T) {
    if let Ok(json_value) = serde_json::to_value(value) {
        context.json(json_value.clone());
        if let Ok(rendered) = serde_json::to_string_pretty(&json_value) {
            context.stdout(rendered);
        }
    }
}

fn normalize_session_path(cwd: &str, input: &str) -> String {
    if input.trim().starts_with('/') {
        return platform_storage::normalize_virtual_path(input);
    }
    platform_storage::normalize_virtual_path(&format!("{}/{}", cwd.trim_end_matches('/'), input))
}

fn parse_bool_flag(raw: &str) -> Result<bool, ShellError> {
    match raw {
        "on" | "true" | "1" => Ok(true),
        "off" | "false" | "0" => Ok(false),
        _ => Err(usage_error(format!("expected on/off, got `{raw}`"))),
    }
}

fn parse_window_id(raw: &str) -> Result<WindowId, ShellError> {
    raw.parse::<u64>()
        .map(WindowId)
        .map_err(|_| usage_error(format!("invalid window id `{raw}`")))
}

fn resolve_open_target(target: &str) -> Option<DesktopAction> {
    if let Some(app_id) = AppId::from_canonical_id(target) {
        return Some(DesktopAction::ActivateApp {
            app_id,
            viewport: None,
        });
    }
    if let Some(slug) = target.strip_prefix("notes:") {
        return Some(DesktopAction::OpenWindow(
            crate::reducer::build_open_request_from_deeplink(
                crate::model::DeepLinkOpenTarget::NotesSlug(slug.to_string()),
            ),
        ));
    }
    if let Some(slug) = target.strip_prefix("projects:") {
        return Some(DesktopAction::OpenWindow(
            crate::reducer::build_open_request_from_deeplink(
                crate::model::DeepLinkOpenTarget::ProjectSlug(slug.to_string()),
            ),
        ));
    }
    None
}

fn help_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: descriptor(
            "help",
            &[],
            "List commands or show help for one command.",
            "help [command]",
            vec![CommandArgSpec {
                name: "command".to_string(),
                summary: "Optional command path or alias.".to_string(),
                required: false,
                repeatable: false,
            }],
            vec![],
            vec![CommandExample {
                command: "help windows.list".to_string(),
                summary: "Show help for a specific command.".to_string(),
            }],
        ),
        completion: None,
        handler: Rc::new(move |context| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let descriptors = runtime.shell_engine.get_value().descriptors();
                if context.argv.len() > 2 {
                    return Err(usage_error("usage: help [command]"));
                }
                if let Some(target) = context.argv.get(1) {
                    let matches = descriptors
                        .into_iter()
                        .filter(|descriptor| {
                            descriptor.path.as_str() == target
                                || descriptor.aliases.iter().any(|alias| alias == target)
                        })
                        .collect::<Vec<_>>();
                    if matches.is_empty() {
                        return Err(ShellError::new(
                            ShellErrorCode::NotFound,
                            format!("command not found: {target}"),
                        ));
                    }
                    for matched in matches {
                        context.stdout(format!(
                            "{}\n{}\nUsage: {}\n",
                            matched.path.as_str(),
                            matched.help.summary,
                            matched.help.usage
                        ));
                    }
                    return Ok(ShellExit::success());
                }

                let mut lines = vec!["Available commands:".to_string()];
                let mut descriptors = descriptors;
                descriptors.sort_by(|left, right| left.path.as_str().cmp(right.path.as_str()));
                for descriptor in descriptors {
                    lines.push(format!(
                        "  {:<26} {}",
                        descriptor.path.as_str(),
                        descriptor.help.summary
                    ));
                }
                context.stdout(lines.join("\n"));
                Ok(ShellExit::success())
            })
        }),
    }
}

fn clear_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: descriptor(
            "clear",
            &[],
            "Clear the terminal transcript UI.",
            "clear",
            vec![],
            vec![],
            vec![],
        ),
        completion: None,
        handler: Rc::new(|_| Box::pin(async { Ok(ShellExit::success()) })),
    }
}

fn history_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: descriptor(
            "history",
            &[],
            "Show recent terminal command history.",
            "history",
            vec![],
            vec![],
            vec![],
        ),
        completion: None,
        handler: Rc::new(move |context| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let history = runtime.state.get_untracked().terminal_history;
                if history.is_empty() {
                    context.status("no terminal history");
                } else {
                    context.stdout(history.join("\n"));
                }
                Ok(ShellExit::success())
            })
        }),
    }
}

fn open_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: descriptor(
            "open",
            &[],
            "Open a system app or deep-link target.",
            "open <target>",
            vec![CommandArgSpec {
                name: "target".to_string(),
                summary: "Canonical app id or deep-link target such as notes:slug.".to_string(),
                required: true,
                repeatable: false,
            }],
            vec![],
            vec![],
        ),
        completion: Some(Rc::new(|request| {
            Box::pin(async move {
                let prefix = request.argv.get(1).cloned().unwrap_or_default();
                Ok(apps::app_registry()
                    .iter()
                    .filter(|entry| entry.app_id.canonical_id().starts_with(&prefix))
                    .map(|entry| CompletionItem {
                        value: entry.app_id.canonical_id().to_string(),
                        label: entry.app_id.canonical_id().to_string(),
                        detail: Some(entry.launcher_label.to_string()),
                    })
                    .collect())
            })
        })),
        handler: Rc::new(move |context| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let target = context
                    .argv
                    .get(1)
                    .ok_or_else(|| usage_error("usage: open <target>"))?;
                let Some(mut action) = resolve_open_target(target) else {
                    return Err(ShellError::new(
                        ShellErrorCode::NotFound,
                        format!("unknown open target `{target}`"),
                    ));
                };
                if let DesktopAction::ActivateApp {
                    ref mut viewport, ..
                } = action
                {
                    *viewport = Some(runtime.host.desktop_viewport_rect(TASKBAR_HEIGHT_PX));
                }
                runtime.dispatch_action(action);
                context.status(format!("opened `{target}`"));
                Ok(ShellExit::success())
            })
        }),
    }
}

fn apps_list_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: descriptor(
            "apps.list",
            &[],
            "List registered apps.",
            "apps.list",
            vec![],
            vec![],
            vec![],
        ),
        completion: None,
        handler: Rc::new(move |context| {
            let _runtime = runtime.clone();
            Box::pin(async move {
                let apps = apps::app_registry()
                    .iter()
                    .map(|entry| {
                        json!({
                            "app_id": entry.app_id.canonical_id(),
                            "label": entry.launcher_label,
                            "single_instance": entry.single_instance,
                        })
                    })
                    .collect::<Vec<_>>();
                context.stdout(
                    apps::app_registry()
                        .iter()
                        .map(|entry| {
                            format!("{}  {}", entry.app_id.canonical_id(), entry.launcher_label)
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
                context.json(Value::Array(apps));
                Ok(ShellExit::success())
            })
        }),
    }
}

fn apps_open_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    let mut registration = open_registration(runtime);
    registration.descriptor.path = system_shell_contract::CommandPath::new("apps.open");
    registration.descriptor.id = CommandId::new("apps.open");
    registration.descriptor.help.usage = "apps.open <app-id>".to_string();
    registration
}

fn windows_list_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: descriptor(
            "windows.list",
            &[],
            "List open windows.",
            "windows.list",
            vec![],
            vec![],
            vec![],
        ),
        completion: None,
        handler: Rc::new(move |context| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let windows = runtime.state.get_untracked().windows;
                if windows.is_empty() {
                    context.status("no open windows");
                    return Ok(ShellExit::success());
                }
                context.stdout(
                    windows
                        .iter()
                        .map(|window| {
                            format!(
                                "{}  {}  {}",
                                window.id.0,
                                window.app_id.canonical_id(),
                                window.title
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
                stdout_json(&context, &windows);
                Ok(ShellExit::success())
            })
        }),
    }
}

fn windows_focus_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    simple_window_registration(runtime, "windows.focus", "Focus a window.", |window_id| {
        DesktopAction::FocusWindow { window_id }
    })
}

fn windows_close_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    simple_window_registration(runtime, "windows.close", "Close a window.", |window_id| {
        DesktopAction::CloseWindow { window_id }
    })
}

fn windows_minimize_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    simple_window_registration(
        runtime,
        "windows.minimize",
        "Minimize a window.",
        |window_id| DesktopAction::MinimizeWindow { window_id },
    )
}

fn windows_restore_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    simple_window_registration(
        runtime,
        "windows.restore",
        "Restore a window.",
        |window_id| DesktopAction::RestoreWindow { window_id },
    )
}

fn simple_window_registration(
    runtime: DesktopRuntimeContext,
    path: &'static str,
    summary: &'static str,
    builder: fn(WindowId) -> DesktopAction,
) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: descriptor(
            path,
            &[],
            summary,
            &format!("{path} <window-id>"),
            vec![CommandArgSpec {
                name: "window-id".to_string(),
                summary: "Runtime window identifier.".to_string(),
                required: true,
                repeatable: false,
            }],
            vec![],
            vec![],
        ),
        completion: None,
        handler: Rc::new(move |context| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let raw = context
                    .argv
                    .get(1)
                    .ok_or_else(|| usage_error(format!("usage: {path} <window-id>")))?;
                let window_id = parse_window_id(raw)?;
                runtime.dispatch_action(builder(window_id));
                context.status(format!("{path} {}", window_id.0));
                Ok(ShellExit::success())
            })
        }),
    }
}

fn theme_show_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: descriptor(
            "theme.show",
            &[],
            "Show current theme state.",
            "theme.show",
            vec![],
            vec![],
            vec![],
        ),
        completion: None,
        handler: Rc::new(move |context| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let theme = runtime.state.get_untracked().theme;
                stdout_json(&context, &theme);
                Ok(ShellExit::success())
            })
        }),
    }
}

fn theme_set_skin_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: descriptor(
            "theme.set.skin",
            &[],
            "Set the desktop skin.",
            "theme.set.skin <modern-adaptive|classic-xp|classic-95>",
            vec![CommandArgSpec {
                name: "skin".to_string(),
                summary: "Desktop skin id.".to_string(),
                required: true,
                repeatable: false,
            }],
            vec![],
            vec![],
        ),
        completion: None,
        handler: Rc::new(move |context| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let skin = match context.argv.get(1).map(String::as_str) {
                    Some("modern-adaptive") => DesktopSkin::ModernAdaptive,
                    Some("classic-xp") => DesktopSkin::ClassicXp,
                    Some("classic-95") => DesktopSkin::Classic95,
                    Some(other) => return Err(usage_error(format!("unknown skin `{other}`"))),
                    None => return Err(usage_error("usage: theme.set.skin <skin>")),
                };
                runtime.dispatch_action(DesktopAction::SetSkin { skin });
                context.status(format!("skin set to {}", skin.css_id()));
                Ok(ShellExit::success())
            })
        }),
    }
}

fn theme_set_high_contrast_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    theme_flag_registration(
        runtime,
        "theme.set.high-contrast",
        "Set high-contrast mode.",
        |enabled| DesktopAction::SetHighContrast { enabled },
    )
}

fn theme_set_reduced_motion_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    theme_flag_registration(
        runtime,
        "theme.set.reduced-motion",
        "Set reduced-motion mode.",
        |enabled| DesktopAction::SetReducedMotion { enabled },
    )
}

fn theme_flag_registration(
    runtime: DesktopRuntimeContext,
    path: &'static str,
    summary: &'static str,
    builder: fn(bool) -> DesktopAction,
) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: descriptor(
            path,
            &[],
            summary,
            &format!("{path} <on|off>"),
            vec![CommandArgSpec {
                name: "value".to_string(),
                summary: "Use on or off.".to_string(),
                required: true,
                repeatable: false,
            }],
            vec![],
            vec![],
        ),
        completion: None,
        handler: Rc::new(move |context| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let raw = context
                    .argv
                    .get(1)
                    .ok_or_else(|| usage_error(format!("usage: {path} <on|off>")))?;
                let value = parse_bool_flag(raw)?;
                runtime.dispatch_action(builder(value));
                context.status(format!("{path} {raw}"));
                Ok(ShellExit::success())
            })
        }),
    }
}

fn config_get_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: descriptor(
            "config.get",
            &[],
            "Load one config value from prefs storage.",
            "config.get <namespace> <key>",
            vec![
                CommandArgSpec {
                    name: "namespace".to_string(),
                    summary: "Config namespace.".to_string(),
                    required: true,
                    repeatable: false,
                },
                CommandArgSpec {
                    name: "key".to_string(),
                    summary: "Config key.".to_string(),
                    required: true,
                    repeatable: false,
                },
            ],
            vec![],
            vec![],
        ),
        completion: None,
        handler: Rc::new(|context| {
            Box::pin(async move {
                let namespace = context
                    .argv
                    .get(1)
                    .ok_or_else(|| usage_error("usage: config.get <namespace> <key>"))?;
                let key = context
                    .argv
                    .get(2)
                    .ok_or_else(|| usage_error("usage: config.get <namespace> <key>"))?;
                let pref_key = format!("{namespace}.{key}");
                let value = platform_storage::load_pref_typed::<Value>(&pref_key)
                    .await
                    .map_err(|err| unavailable(err))?;
                match value {
                    Some(value) => {
                        context.json(value.clone());
                        context.stdout(
                            serde_json::to_string_pretty(&value)
                                .unwrap_or_else(|_| value.to_string()),
                        );
                    }
                    None => context.status(format!("no value stored for `{pref_key}`")),
                }
                Ok(ShellExit::success())
            })
        }),
    }
}

fn config_set_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: descriptor(
            "config.set",
            &[],
            "Store one config value in prefs storage.",
            "config.set <namespace> <key> <json>",
            vec![
                CommandArgSpec {
                    name: "namespace".to_string(),
                    summary: "Config namespace.".to_string(),
                    required: true,
                    repeatable: false,
                },
                CommandArgSpec {
                    name: "key".to_string(),
                    summary: "Config key.".to_string(),
                    required: true,
                    repeatable: false,
                },
                CommandArgSpec {
                    name: "json".to_string(),
                    summary: "JSON value payload.".to_string(),
                    required: true,
                    repeatable: true,
                },
            ],
            vec![],
            vec![],
        ),
        completion: None,
        handler: Rc::new(|context| {
            Box::pin(async move {
                if context.argv.len() < 4 {
                    return Err(usage_error("usage: config.set <namespace> <key> <json>"));
                }
                let namespace = &context.argv[1];
                let key = &context.argv[2];
                let raw_json = context.argv[3..].join(" ");
                let value: Value = serde_json::from_str(&raw_json)
                    .map_err(|err| usage_error(format!("invalid json: {err}")))?;
                let pref_key = format!("{namespace}.{key}");
                platform_storage::save_pref_typed(&pref_key, &value)
                    .await
                    .map_err(|err| unavailable(err))?;
                context.status(format!("saved `{pref_key}`"));
                Ok(ShellExit::success())
            })
        }),
    }
}

fn inspect_runtime_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: descriptor(
            "inspect.runtime",
            &[],
            "Inspect desktop runtime state.",
            "inspect.runtime",
            vec![],
            vec![],
            vec![],
        ),
        completion: None,
        handler: Rc::new(move |context| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let desktop = runtime.state.get_untracked();
                let payload = json!({
                    "windows": desktop.windows.len(),
                    "start_menu_open": desktop.start_menu_open,
                    "skin": desktop.theme.skin.css_id(),
                    "high_contrast": desktop.theme.high_contrast,
                    "reduced_motion": desktop.theme.reduced_motion,
                    "terminal_history_len": desktop.terminal_history.len(),
                });
                context.json(payload.clone());
                context.stdout(serde_json::to_string_pretty(&payload).unwrap_or_default());
                Ok(ShellExit::success())
            })
        }),
    }
}

fn inspect_windows_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    let mut registration = windows_list_registration(runtime);
    registration.descriptor.path = system_shell_contract::CommandPath::new("inspect.windows");
    registration.descriptor.id = CommandId::new("inspect.windows");
    registration.descriptor.help.summary = "Inspect open window state.".to_string();
    registration
}

fn inspect_storage_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: descriptor(
            "inspect.storage",
            &[],
            "Inspect storage namespaces and host strategy.",
            "inspect.storage",
            vec![],
            vec![],
            vec![],
        ),
        completion: None,
        handler: Rc::new(|context| {
            Box::pin(async move {
                let namespaces = platform_storage::list_app_state_namespaces()
                    .await
                    .map_err(|err| unavailable(err))?;
                let payload = json!({
                    "host_strategy": platform_storage::host_strategy_name(),
                    "namespaces": namespaces,
                });
                context.json(payload.clone());
                context.stdout(serde_json::to_string_pretty(&payload).unwrap_or_default());
                Ok(ShellExit::success())
            })
        }),
    }
}

fn fs_pwd_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: descriptor(
            "fs.pwd",
            &["pwd"],
            "Print the logical filesystem cwd.",
            "fs.pwd",
            vec![],
            vec![],
            vec![],
        ),
        completion: None,
        handler: Rc::new(|context| {
            Box::pin(async move {
                context.stdout(context.cwd.clone());
                Ok(ShellExit::success())
            })
        }),
    }
}

fn fs_cd_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: descriptor(
            "fs.cd",
            &["cd"],
            "Change the logical filesystem cwd.",
            "fs.cd <path>",
            vec![CommandArgSpec {
                name: "path".to_string(),
                summary: "Target directory path.".to_string(),
                required: true,
                repeatable: false,
            }],
            vec![],
            vec![],
        ),
        completion: None,
        handler: Rc::new(|context| {
            Box::pin(async move {
                let target = context
                    .argv
                    .get(1)
                    .ok_or_else(|| usage_error("usage: fs.cd <path>"))?;
                let resolved = normalize_session_path(&context.cwd, target);
                platform_storage::explorer_stat(&resolved)
                    .await
                    .map_err(|err| unavailable(err))?;
                context.set_cwd(resolved.clone());
                context.status(format!("cwd = {resolved}"));
                Ok(ShellExit::success())
            })
        }),
    }
}

fn fs_ls_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: descriptor(
            "fs.ls",
            &["ls"],
            "List a directory using the active explorer backend.",
            "fs.ls [path]",
            vec![CommandArgSpec {
                name: "path".to_string(),
                summary: "Optional target directory.".to_string(),
                required: false,
                repeatable: false,
            }],
            vec![],
            vec![],
        ),
        completion: None,
        handler: Rc::new(|context| {
            Box::pin(async move {
                let target = context
                    .argv
                    .get(1)
                    .map(|path| normalize_session_path(&context.cwd, path))
                    .unwrap_or_else(|| context.cwd.clone());
                let listing = platform_storage::explorer_list_dir(&target)
                    .await
                    .map_err(|err| unavailable(err))?;
                context.stdout(
                    listing
                        .entries
                        .iter()
                        .map(|entry| format!("{:?}\t{}", entry.kind, entry.path))
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
                stdout_json(&context, &listing);
                Ok(ShellExit::success())
            })
        }),
    }
}
