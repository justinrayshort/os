//! Shell runtime integration for the browser-native system terminal.

use std::{cmp::Ordering, rc::Rc};

use desktop_app_contract::{
    AppCapability, AppCommandContext, AppCommandProvider, AppCommandRegistration,
    CommandRegistrationHandle as AppCommandRegistrationHandle, CommandService, ShellSessionHandle,
};
use futures::future::LocalBoxFuture;
use leptos::SignalGetUntracked;
use nu_ansi_term::{Color, Style};
use nu_protocol::{Config as NuConfig, Record as NuRecord, Span as NuSpan, Value as NuValue};
use nu_table::{NuTable, TableTheme, TextStyle};
use platform_storage::ExplorerEntryKind;
use serde_json::Value;
use system_shell::{CommandExecutionContext, CommandRegistryHandle};
use system_shell_contract::{
    CommandArgSpec, CommandDataShape, CommandDescriptor, CommandExample, CommandId,
    CommandInputShape, CommandInteractionKind, CommandNotice, CommandNoticeLevel,
    CommandOutputShape, CommandPath, CommandResult, CommandScope, CommandVisibility,
    CompletionItem, CompletionRequest, DisplayPreference, HelpDoc, ParsedLiteral, ParsedValue,
    ShellError, ShellErrorCode, ShellRequest, ShellStreamEvent, StructuredData, StructuredField,
    StructuredRecord, StructuredScalar, StructuredSchema, StructuredSchemaField, StructuredTable,
    StructuredValue,
};
use tabled::grid::records::vec_records::Text;

use crate::{
    apps,
    components::DesktopRuntimeContext,
    model::{AppId, DesktopSkin, WindowId},
    reducer::DesktopAction,
};

const TASKBAR_HEIGHT_PX: i32 = 38;
const TABLE_RENDER_WIDTH: usize = 120;

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
                let session = leptos::with_owner(runtime.owner, || {
                    runtime.shell_engine.get_value().new_session(cwd)
                });
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
        context.invocation.clone(),
        context.argv.clone(),
        context.args.clone(),
        context.cwd.clone(),
        context.input.clone(),
        context.source_window_id,
        Rc::new(move |event| emit_shell_event(&emit_context, event)),
        Rc::new(move |cwd| set_cwd_context.set_cwd(cwd)),
        Rc::new(move || cancel_context.is_cancelled()),
    )
}

fn emit_shell_event(context: &CommandExecutionContext, event: ShellStreamEvent) {
    match event {
        ShellStreamEvent::Notice { notice, .. } => match notice.level {
            CommandNoticeLevel::Info => context.info(notice.message),
            CommandNoticeLevel::Warning => context.warn(notice.message),
            CommandNoticeLevel::Error => context.error(notice.message),
        },
        ShellStreamEvent::Progress { value, label, .. } => context.progress(value, label),
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
        help_list_registration(runtime.clone()),
        help_show_registration(runtime.clone()),
        clear_registration(),
        history_list_registration(runtime.clone()),
        open_registration(runtime.clone()),
        apps_list_registration(),
        apps_open_registration(runtime.clone()),
        windows_list_registration(runtime.clone(), "windows list", "List open windows."),
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
        windows_list_registration(
            runtime.clone(),
            "inspect windows",
            "Inspect open window state.",
        ),
        inspect_storage_registration(),
        pwd_registration(),
        cd_registration(),
        ls_registration(),
        data_select_registration(),
        data_where_registration(),
        data_sort_registration(),
        data_first_registration(),
        data_get_registration(),
    ]
}

fn usage_error(message: impl Into<String>) -> ShellError {
    ShellError::new(ShellErrorCode::Usage, message)
}

fn unavailable(message: impl Into<String>) -> ShellError {
    ShellError::new(ShellErrorCode::Unavailable, message)
}

fn descriptor(
    path: &str,
    aliases: &[&str],
    summary: &str,
    usage: &str,
    args: Vec<CommandArgSpec>,
    examples: Vec<CommandExample>,
    interaction_kind: CommandInteractionKind,
    input_shape: CommandInputShape,
    output_shape: CommandOutputShape,
) -> CommandDescriptor {
    let path = CommandPath::new(path);
    CommandDescriptor {
        id: CommandId::new(path.display()),
        parent_path: path.parent(),
        path,
        aliases: aliases.iter().map(|alias| alias.to_string()).collect(),
        scope: CommandScope::Global,
        visibility: CommandVisibility::Public,
        interaction_kind,
        discoverable_children: true,
        input_shape,
        output_shape,
        args,
        options: Vec::new(),
        help: HelpDoc {
            summary: summary.to_string(),
            description: None,
            usage: usage.to_string(),
            examples,
        },
    }
}

fn root_descriptor(
    path: &str,
    aliases: &[&str],
    summary: &str,
    usage: &str,
    args: Vec<CommandArgSpec>,
    examples: Vec<CommandExample>,
    input_shape: CommandInputShape,
    output_shape: CommandOutputShape,
) -> CommandDescriptor {
    descriptor(
        path,
        aliases,
        summary,
        usage,
        args,
        examples,
        CommandInteractionKind::RootVerb,
        input_shape,
        output_shape,
    )
}

fn namespaced_descriptor(
    path: &str,
    aliases: &[&str],
    summary: &str,
    usage: &str,
    args: Vec<CommandArgSpec>,
    examples: Vec<CommandExample>,
    input_shape: CommandInputShape,
    output_shape: CommandOutputShape,
) -> CommandDescriptor {
    descriptor(
        path,
        aliases,
        summary,
        usage,
        args,
        examples,
        CommandInteractionKind::Hierarchical,
        input_shape,
        output_shape,
    )
}

fn empty_result() -> CommandResult {
    CommandResult::success(StructuredData::Empty)
}

fn info_result(message: impl Into<String>) -> CommandResult {
    CommandResult {
        output: StructuredData::Empty,
        display: DisplayPreference::Auto,
        notices: vec![CommandNotice {
            level: CommandNoticeLevel::Info,
            message: message.into(),
        }],
        cwd: None,
        exit: system_shell_contract::ShellExit::success(),
    }
}

fn string_data(value: impl Into<String>) -> StructuredData {
    StructuredData::Value(StructuredValue::Scalar(StructuredScalar::String(
        value.into(),
    )))
}

fn value_field(name: &str, value: StructuredValue) -> StructuredField {
    StructuredField {
        name: name.to_string(),
        value,
    }
}

fn string_field(name: &str, value: impl Into<String>) -> StructuredField {
    value_field(
        name,
        StructuredValue::Scalar(StructuredScalar::String(value.into())),
    )
}

fn bool_field(name: &str, value: bool) -> StructuredField {
    value_field(name, StructuredValue::Scalar(StructuredScalar::Bool(value)))
}

fn int_field(name: &str, value: i64) -> StructuredField {
    value_field(name, StructuredValue::Scalar(StructuredScalar::Int(value)))
}

fn optional_u64_field(name: &str, value: Option<u64>) -> StructuredField {
    match value {
        Some(value) => value_field(
            name,
            StructuredValue::Scalar(StructuredScalar::Int(value as i64)),
        ),
        None => value_field(name, StructuredValue::Scalar(StructuredScalar::Null)),
    }
}

fn record_data(fields: Vec<StructuredField>) -> StructuredData {
    StructuredData::Record(StructuredRecord { fields })
}

fn table_data(
    columns: Vec<String>,
    rows: Vec<StructuredRecord>,
    source: Option<CommandPath>,
) -> StructuredData {
    let schema = Some(StructuredSchema {
        fields: columns
            .iter()
            .map(|name| StructuredSchemaField {
                name: name.clone(),
                shape: CommandDataShape::Any,
            })
            .collect(),
    });
    let fallback_text = render_table_fallback(&columns, &rows);
    StructuredData::Table(StructuredTable {
        columns,
        rows,
        schema,
        source_command: source,
        fallback_text,
    })
}

fn display_structured_value(value: &StructuredValue) -> String {
    structured_value_to_nu(value).to_abbreviated_string(&NuConfig::default())
}

fn render_table_fallback(columns: &[String], rows: &[StructuredRecord]) -> Option<String> {
    if columns.is_empty() {
        return None;
    }

    let mut table = NuTable::new(rows.len() + 1, columns.len());
    let header_style = Style::new().fg(Color::Blue).bold();
    table.set_row(
        0,
        columns
            .iter()
            .map(|column| Text::new(header_style.paint(column.as_str()).to_string()))
            .collect(),
    );

    for (row_index, row) in rows.iter().enumerate() {
        table.set_row(
            row_index + 1,
            columns
                .iter()
                .map(|column| {
                    let value = row
                        .fields
                        .iter()
                        .find(|field| field.name == *column)
                        .map(|field| display_structured_value(&field.value))
                        .unwrap_or_default();
                    Text::new(value)
                })
                .collect(),
        );
    }

    table.set_data_style(TextStyle::basic_left());
    table.set_header_style(TextStyle::basic_center());
    table.set_theme(TableTheme::rounded());
    table.set_structure(false, true, false);
    table.draw(TABLE_RENDER_WIDTH)
}

fn structured_value_to_nu(value: &StructuredValue) -> NuValue {
    match value {
        StructuredValue::Scalar(StructuredScalar::Null) => NuValue::nothing(NuSpan::unknown()),
        StructuredValue::Scalar(StructuredScalar::Bool(value)) => {
            NuValue::bool(*value, NuSpan::unknown())
        }
        StructuredValue::Scalar(StructuredScalar::Int(value)) => {
            NuValue::int(*value, NuSpan::unknown())
        }
        StructuredValue::Scalar(StructuredScalar::Float(value)) => {
            NuValue::float(*value, NuSpan::unknown())
        }
        StructuredValue::Scalar(StructuredScalar::String(value)) => {
            NuValue::string(value.clone(), NuSpan::unknown())
        }
        StructuredValue::Record(record) => {
            NuValue::record(structured_record_to_nu(record), NuSpan::unknown())
        }
        StructuredValue::List(values) => NuValue::list(
            values.iter().map(structured_value_to_nu).collect(),
            NuSpan::unknown(),
        ),
    }
}

fn structured_record_to_nu(record: &StructuredRecord) -> NuRecord {
    let mut out = NuRecord::new();
    for field in &record.fields {
        out.push(field.name.clone(), structured_value_to_nu(&field.value));
    }
    out
}

fn json_to_structured_value(value: Value) -> StructuredValue {
    match value {
        Value::Null => StructuredValue::Scalar(StructuredScalar::Null),
        Value::Bool(value) => StructuredValue::Scalar(StructuredScalar::Bool(value)),
        Value::Number(value) => {
            if let Some(int) = value.as_i64() {
                StructuredValue::Scalar(StructuredScalar::Int(int))
            } else {
                StructuredValue::Scalar(StructuredScalar::Float(value.as_f64().unwrap_or_default()))
            }
        }
        Value::String(value) => StructuredValue::Scalar(StructuredScalar::String(value)),
        Value::Array(values) => {
            StructuredValue::List(values.into_iter().map(json_to_structured_value).collect())
        }
        Value::Object(values) => StructuredValue::Record(StructuredRecord {
            fields: values
                .into_iter()
                .map(|(name, value)| StructuredField {
                    name,
                    value: json_to_structured_value(value),
                })
                .collect(),
        }),
    }
}

fn json_to_structured_data(value: Value) -> StructuredData {
    match json_to_structured_value(value) {
        StructuredValue::Record(record) => StructuredData::Record(record),
        StructuredValue::List(values) => StructuredData::List(values),
        other => StructuredData::Value(other),
    }
}

fn structured_value_to_json(value: &StructuredValue) -> Value {
    match value {
        StructuredValue::Scalar(StructuredScalar::Null) => Value::Null,
        StructuredValue::Scalar(StructuredScalar::Bool(value)) => Value::Bool(*value),
        StructuredValue::Scalar(StructuredScalar::Int(value)) => Value::Number((*value).into()),
        StructuredValue::Scalar(StructuredScalar::Float(value)) => {
            serde_json::Number::from_f64(*value)
                .map(Value::Number)
                .unwrap_or(Value::Null)
        }
        StructuredValue::Scalar(StructuredScalar::String(value)) => Value::String(value.clone()),
        StructuredValue::Record(record) => Value::Object(
            record
                .fields
                .iter()
                .map(|field| (field.name.clone(), structured_value_to_json(&field.value)))
                .collect(),
        ),
        StructuredValue::List(values) => {
            Value::Array(values.iter().map(structured_value_to_json).collect())
        }
    }
}

fn parsed_value_to_structured(value: &ParsedValue) -> StructuredValue {
    match &value.literal {
        ParsedLiteral::Null => StructuredValue::Scalar(StructuredScalar::Null),
        ParsedLiteral::Bool(value) => StructuredValue::Scalar(StructuredScalar::Bool(*value)),
        ParsedLiteral::Int(value) => StructuredValue::Scalar(StructuredScalar::Int(*value)),
        ParsedLiteral::Float(value) => StructuredValue::Scalar(StructuredScalar::Float(*value)),
        ParsedLiteral::String(value) => {
            StructuredValue::Scalar(StructuredScalar::String(value.clone()))
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

fn app_row(entry: apps::AppDescriptor) -> StructuredRecord {
    StructuredRecord {
        fields: vec![
            string_field("app_id", entry.app_id.canonical_id()),
            string_field("label", entry.launcher_label),
            bool_field("single_instance", entry.single_instance),
        ],
    }
}

fn window_row(window: &crate::model::WindowRecord) -> StructuredRecord {
    StructuredRecord {
        fields: vec![
            int_field("id", window.id.0 as i64),
            string_field("app_id", window.app_id.canonical_id()),
            string_field("title", window.title.clone()),
            bool_field("focused", window.is_focused),
            bool_field("minimized", window.minimized),
            bool_field("maximized", window.maximized),
        ],
    }
}

fn explorer_row(entry: &platform_storage::ExplorerEntry) -> StructuredRecord {
    StructuredRecord {
        fields: vec![
            string_field("name", entry.name.clone()),
            string_field(
                "kind",
                match entry.kind {
                    ExplorerEntryKind::File => "file",
                    ExplorerEntryKind::Directory => "dir",
                },
            ),
            string_field("path", entry.path.clone()),
            optional_u64_field("size", entry.size),
            optional_u64_field("modified_at_unix_ms", entry.modified_at_unix_ms),
        ],
    }
}

fn table_rows_from_descriptors(
    descriptors: &[CommandDescriptor],
    prefix: &[String],
) -> Vec<StructuredRecord> {
    let mut seen = std::collections::BTreeSet::new();
    let mut rows = Vec::new();
    for descriptor in descriptors {
        let tokens = descriptor_tokens(descriptor);
        if tokens.len() <= prefix.len() || !tokens.starts_with(prefix) {
            continue;
        }
        let command = tokens[..prefix.len() + 1].join(" ");
        if !seen.insert(command.clone()) {
            continue;
        }
        rows.push(StructuredRecord {
            fields: vec![
                string_field("command", command),
                string_field("summary", descriptor.help.summary.clone()),
            ],
        });
    }
    rows
}

fn descriptor_tokens(descriptor: &CommandDescriptor) -> Vec<String> {
    descriptor
        .path
        .segments()
        .iter()
        .map(|segment| segment.as_str().to_string())
        .collect()
}

fn help_target<'a>(descriptors: &'a [CommandDescriptor], target: &[String]) -> HelpTarget<'a> {
    for descriptor in descriptors {
        let tokens = descriptor_tokens(descriptor);
        if tokens == target {
            return HelpTarget::Leaf(descriptor);
        }
        if descriptor.aliases.iter().any(|alias| {
            alias
                .split_whitespace()
                .eq(target.iter().map(String::as_str))
        }) {
            return HelpTarget::Leaf(descriptor);
        }
    }

    if descriptors.iter().any(|descriptor| {
        let tokens = descriptor_tokens(descriptor);
        tokens.len() > target.len() && tokens.starts_with(target)
    }) {
        return HelpTarget::Namespace;
    }

    HelpTarget::Missing
}

enum HelpTarget<'a> {
    Leaf(&'a CommandDescriptor),
    Namespace,
    Missing,
}

fn help_list_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            "help list",
            &[],
            "List top-level commands and namespaces.",
            "help list",
            Vec::new(),
            vec![CommandExample {
                command: "help list".to_string(),
                summary: "Show top-level command categories.".to_string(),
            }],
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Table),
        ),
        completion: None,
        handler: Rc::new(move |_| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let descriptors = runtime.shell_engine.get_value().descriptors();
                let rows = table_rows_from_descriptors(&descriptors, &[]);
                Ok(CommandResult {
                    output: table_data(
                        vec!["command".to_string(), "summary".to_string()],
                        rows,
                        Some(CommandPath::new("help list")),
                    ),
                    display: DisplayPreference::Help,
                    notices: Vec::new(),
                    cwd: None,
                    exit: system_shell_contract::ShellExit::success(),
                })
            })
        }),
    }
}

fn help_show_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            "help show",
            &[],
            "Show help for a command or namespace.",
            "help show <command...>",
            vec![CommandArgSpec {
                name: "command".to_string(),
                summary: "Command path to inspect.".to_string(),
                required: true,
                repeatable: true,
            }],
            vec![CommandExample {
                command: "help show ls".to_string(),
                summary: "Show help for a root verb.".to_string(),
            }],
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Any),
        ),
        completion: None,
        handler: Rc::new(move |context| {
            let runtime = runtime.clone();
            Box::pin(async move {
                if context.args.is_empty() {
                    return Err(usage_error("usage: help show <command...>"));
                }
                let descriptors = runtime.shell_engine.get_value().descriptors();
                match help_target(&descriptors, &context.args) {
                    HelpTarget::Leaf(descriptor) => Ok(CommandResult {
                        output: record_data(vec![
                            string_field("path", descriptor.path.display()),
                            string_field("summary", descriptor.help.summary.clone()),
                            string_field("usage", descriptor.help.usage.clone()),
                        ]),
                        display: DisplayPreference::Help,
                        notices: Vec::new(),
                        cwd: None,
                        exit: system_shell_contract::ShellExit::success(),
                    }),
                    HelpTarget::Namespace => Ok(CommandResult {
                        output: table_data(
                            vec!["command".to_string(), "summary".to_string()],
                            table_rows_from_descriptors(&descriptors, &context.args),
                            Some(CommandPath::from_segments(
                                context
                                    .args
                                    .iter()
                                    .cloned()
                                    .map(system_shell_contract::CommandSegment::new),
                            )),
                        ),
                        display: DisplayPreference::Help,
                        notices: Vec::new(),
                        cwd: None,
                        exit: system_shell_contract::ShellExit::success(),
                    }),
                    HelpTarget::Missing => Err(ShellError::new(
                        ShellErrorCode::NotFound,
                        format!("command not found: {}", context.args.join(" ")),
                    )),
                }
            })
        }),
    }
}

fn clear_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            "terminal clear",
            &["clear"],
            "Clear the terminal transcript UI.",
            "terminal clear",
            Vec::new(),
            vec![CommandExample {
                command: "clear".to_string(),
                summary: "Clear the current terminal transcript.".to_string(),
            }],
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Empty),
        ),
        completion: None,
        handler: Rc::new(|_| Box::pin(async move { Ok(empty_result()) })),
    }
}

fn history_list_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            "history list",
            &[],
            "Show recent terminal command history.",
            "history list",
            Vec::new(),
            Vec::new(),
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Table),
        ),
        completion: None,
        handler: Rc::new(move |_| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let rows = runtime
                    .state
                    .get_untracked()
                    .terminal_history
                    .iter()
                    .enumerate()
                    .map(|(index, command)| StructuredRecord {
                        fields: vec![
                            int_field("index", index as i64),
                            string_field("command", command.clone()),
                        ],
                    })
                    .collect::<Vec<_>>();
                Ok(CommandResult {
                    output: table_data(
                        vec!["index".to_string(), "command".to_string()],
                        rows,
                        Some(CommandPath::new("history list")),
                    ),
                    display: DisplayPreference::Table,
                    notices: Vec::new(),
                    cwd: None,
                    exit: system_shell_contract::ShellExit::success(),
                })
            })
        }),
    }
}

fn open_completion(request: CompletionRequest) -> Vec<CompletionItem> {
    let prefix = request.argv.get(1).cloned().unwrap_or_default();
    apps::app_registry()
        .iter()
        .filter(|entry| entry.app_id.canonical_id().starts_with(&prefix))
        .map(|entry| CompletionItem {
            value: entry.app_id.canonical_id().to_string(),
            label: entry.app_id.canonical_id().to_string(),
            detail: Some(entry.launcher_label.to_string()),
        })
        .collect()
}

fn open_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: root_descriptor(
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
            vec![CommandExample {
                command: "open system.terminal".to_string(),
                summary: "Open the terminal app.".to_string(),
            }],
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Empty),
        ),
        completion: Some(Rc::new(|request| {
            Box::pin(async move { Ok(open_completion(request)) })
        })),
        handler: Rc::new(move |context| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let target = context
                    .args
                    .first()
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
                Ok(info_result(format!("opened `{target}`")))
            })
        }),
    }
}

fn apps_list_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            "apps list",
            &[],
            "List registered apps.",
            "apps list",
            Vec::new(),
            Vec::new(),
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Table),
        ),
        completion: None,
        handler: Rc::new(|_| {
            Box::pin(async move {
                Ok(CommandResult {
                    output: table_data(
                        vec![
                            "app_id".to_string(),
                            "label".to_string(),
                            "single_instance".to_string(),
                        ],
                        apps::app_registry().iter().copied().map(app_row).collect(),
                        Some(CommandPath::new("apps list")),
                    ),
                    display: DisplayPreference::Table,
                    notices: Vec::new(),
                    cwd: None,
                    exit: system_shell_contract::ShellExit::success(),
                })
            })
        }),
    }
}

fn apps_open_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            "apps open",
            &[],
            "Open an app by canonical id.",
            "apps open <app-id>",
            vec![CommandArgSpec {
                name: "app-id".to_string(),
                summary: "Canonical app id.".to_string(),
                required: true,
                repeatable: false,
            }],
            Vec::new(),
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Empty),
        ),
        completion: Some(Rc::new(|request| {
            Box::pin(async move { Ok(open_completion(request)) })
        })),
        handler: Rc::new(move |context| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let target = context
                    .args
                    .first()
                    .ok_or_else(|| usage_error("usage: apps open <app-id>"))?;
                let Some(mut action) = resolve_open_target(target) else {
                    return Err(ShellError::new(
                        ShellErrorCode::NotFound,
                        format!("unknown app `{target}`"),
                    ));
                };
                if let DesktopAction::ActivateApp {
                    ref mut viewport, ..
                } = action
                {
                    *viewport = Some(runtime.host.desktop_viewport_rect(TASKBAR_HEIGHT_PX));
                }
                runtime.dispatch_action(action);
                Ok(info_result(format!("opened `{target}`")))
            })
        }),
    }
}

fn windows_list_registration(
    runtime: DesktopRuntimeContext,
    path: &'static str,
    summary: &'static str,
) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            path,
            &[],
            summary,
            path,
            Vec::new(),
            Vec::new(),
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Table),
        ),
        completion: None,
        handler: Rc::new(move |_| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let windows = runtime.state.get_untracked().windows;
                Ok(CommandResult {
                    output: table_data(
                        vec![
                            "id".to_string(),
                            "app_id".to_string(),
                            "title".to_string(),
                            "focused".to_string(),
                            "minimized".to_string(),
                            "maximized".to_string(),
                        ],
                        windows.iter().map(window_row).collect(),
                        Some(CommandPath::new(path)),
                    ),
                    display: DisplayPreference::Table,
                    notices: Vec::new(),
                    cwd: None,
                    exit: system_shell_contract::ShellExit::success(),
                })
            })
        }),
    }
}

fn simple_window_registration(
    runtime: DesktopRuntimeContext,
    path: &'static str,
    summary: &'static str,
    builder: fn(WindowId) -> DesktopAction,
) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
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
            Vec::new(),
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Empty),
        ),
        completion: None,
        handler: Rc::new(move |context| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let raw = context
                    .args
                    .first()
                    .ok_or_else(|| usage_error(format!("usage: {path} <window-id>")))?;
                let window_id = parse_window_id(raw)?;
                runtime.dispatch_action(builder(window_id));
                Ok(info_result(format!("{path} {}", window_id.0)))
            })
        }),
    }
}

fn windows_focus_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    simple_window_registration(runtime, "windows focus", "Focus a window.", |window_id| {
        DesktopAction::FocusWindow { window_id }
    })
}

fn windows_close_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    simple_window_registration(runtime, "windows close", "Close a window.", |window_id| {
        DesktopAction::CloseWindow { window_id }
    })
}

fn windows_minimize_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    simple_window_registration(
        runtime,
        "windows minimize",
        "Minimize a window.",
        |window_id| DesktopAction::MinimizeWindow { window_id },
    )
}

fn windows_restore_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    simple_window_registration(
        runtime,
        "windows restore",
        "Restore a window.",
        |window_id| DesktopAction::RestoreWindow { window_id },
    )
}

fn theme_show_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            "theme show",
            &[],
            "Show current theme state.",
            "theme show",
            Vec::new(),
            Vec::new(),
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Record),
        ),
        completion: None,
        handler: Rc::new(move |_| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let theme = runtime.state.get_untracked().theme;
                Ok(CommandResult {
                    output: record_data(vec![
                        string_field("skin", theme.skin.css_id()),
                        bool_field("high_contrast", theme.high_contrast),
                        bool_field("reduced_motion", theme.reduced_motion),
                        bool_field("audio_enabled", theme.audio_enabled),
                    ]),
                    display: DisplayPreference::Record,
                    notices: Vec::new(),
                    cwd: None,
                    exit: system_shell_contract::ShellExit::success(),
                })
            })
        }),
    }
}

fn theme_set_skin_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            "theme set skin",
            &[],
            "Set the desktop skin.",
            "theme set skin <modern-adaptive|classic-xp|classic-95>",
            vec![CommandArgSpec {
                name: "skin".to_string(),
                summary: "Desktop skin id.".to_string(),
                required: true,
                repeatable: false,
            }],
            Vec::new(),
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Empty),
        ),
        completion: None,
        handler: Rc::new(move |context| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let skin = match context.args.first().map(String::as_str) {
                    Some("modern-adaptive") => DesktopSkin::ModernAdaptive,
                    Some("classic-xp") => DesktopSkin::ClassicXp,
                    Some("classic-95") => DesktopSkin::Classic95,
                    Some(other) => return Err(usage_error(format!("unknown skin `{other}`"))),
                    None => return Err(usage_error("usage: theme set skin <skin>")),
                };
                runtime.dispatch_action(DesktopAction::SetSkin { skin });
                Ok(info_result(format!("skin set to {}", skin.css_id())))
            })
        }),
    }
}

fn theme_flag_registration(
    runtime: DesktopRuntimeContext,
    path: &'static str,
    summary: &'static str,
    builder: fn(bool) -> DesktopAction,
) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
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
            Vec::new(),
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Empty),
        ),
        completion: None,
        handler: Rc::new(move |context| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let raw = context
                    .args
                    .first()
                    .ok_or_else(|| usage_error(format!("usage: {path} <on|off>")))?;
                let value = parse_bool_flag(raw)?;
                runtime.dispatch_action(builder(value));
                Ok(info_result(format!("{path} {raw}")))
            })
        }),
    }
}

fn theme_set_high_contrast_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    theme_flag_registration(
        runtime,
        "theme set high-contrast",
        "Set high-contrast mode.",
        |enabled| DesktopAction::SetHighContrast { enabled },
    )
}

fn theme_set_reduced_motion_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    theme_flag_registration(
        runtime,
        "theme set reduced-motion",
        "Set reduced-motion mode.",
        |enabled| DesktopAction::SetReducedMotion { enabled },
    )
}

fn config_get_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            "config get",
            &[],
            "Load one config value from prefs storage.",
            "config get <namespace> <key>",
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
            Vec::new(),
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Any),
        ),
        completion: None,
        handler: Rc::new(|context| {
            Box::pin(async move {
                let namespace = context
                    .args
                    .first()
                    .ok_or_else(|| usage_error("usage: config get <namespace> <key>"))?;
                let key = context
                    .args
                    .get(1)
                    .ok_or_else(|| usage_error("usage: config get <namespace> <key>"))?;
                let pref_key = format!("{namespace}.{key}");
                let value = platform_storage::load_pref_typed::<Value>(&pref_key)
                    .await
                    .map_err(unavailable)?;
                match value {
                    Some(value) => Ok(CommandResult {
                        output: json_to_structured_data(value),
                        display: DisplayPreference::Auto,
                        notices: Vec::new(),
                        cwd: None,
                        exit: system_shell_contract::ShellExit::success(),
                    }),
                    None => Ok(info_result(format!("no value stored for `{pref_key}`"))),
                }
            })
        }),
    }
}

fn config_set_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            "config set",
            &[],
            "Store one config value in prefs storage.",
            "config set <namespace> <key> <value>",
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
                    name: "value".to_string(),
                    summary: "Typed literal or string payload.".to_string(),
                    required: true,
                    repeatable: false,
                },
            ],
            Vec::new(),
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Empty),
        ),
        completion: None,
        handler: Rc::new(|context| {
            Box::pin(async move {
                if context.args.len() < 3 {
                    return Err(usage_error("usage: config set <namespace> <key> <value>"));
                }
                let namespace = &context.args[0];
                let key = &context.args[1];
                let value = context
                    .invocation
                    .values
                    .get(2)
                    .map(parsed_value_to_structured)
                    .unwrap_or_else(|| {
                        StructuredValue::Scalar(StructuredScalar::String(context.args[2].clone()))
                    });
                let pref_key = format!("{namespace}.{key}");
                platform_storage::save_pref_typed(&pref_key, &structured_value_to_json(&value))
                    .await
                    .map_err(unavailable)?;
                Ok(info_result(format!("saved `{pref_key}`")))
            })
        }),
    }
}

fn inspect_runtime_registration(runtime: DesktopRuntimeContext) -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            "inspect runtime",
            &[],
            "Inspect desktop runtime state.",
            "inspect runtime",
            Vec::new(),
            Vec::new(),
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Record),
        ),
        completion: None,
        handler: Rc::new(move |_| {
            let runtime = runtime.clone();
            Box::pin(async move {
                let desktop = runtime.state.get_untracked();
                Ok(CommandResult {
                    output: record_data(vec![
                        int_field("windows", desktop.windows.len() as i64),
                        bool_field("start_menu_open", desktop.start_menu_open),
                        string_field("skin", desktop.theme.skin.css_id()),
                        bool_field("high_contrast", desktop.theme.high_contrast),
                        bool_field("reduced_motion", desktop.theme.reduced_motion),
                        int_field(
                            "terminal_history_len",
                            desktop.terminal_history.len() as i64,
                        ),
                    ]),
                    display: DisplayPreference::Record,
                    notices: Vec::new(),
                    cwd: None,
                    exit: system_shell_contract::ShellExit::success(),
                })
            })
        }),
    }
}

fn inspect_storage_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            "inspect storage",
            &[],
            "Inspect storage namespaces and host strategy.",
            "inspect storage",
            Vec::new(),
            Vec::new(),
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Record),
        ),
        completion: None,
        handler: Rc::new(|_| {
            Box::pin(async move {
                let namespaces = platform_storage::list_app_state_namespaces()
                    .await
                    .map_err(unavailable)?;
                Ok(CommandResult {
                    output: record_data(vec![
                        string_field("host_strategy", platform_storage::host_strategy_name()),
                        value_field(
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
                    display: DisplayPreference::Record,
                    notices: Vec::new(),
                    cwd: None,
                    exit: system_shell_contract::ShellExit::success(),
                })
            })
        }),
    }
}

fn pwd_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: root_descriptor(
            "pwd",
            &[],
            "Print the logical filesystem cwd.",
            "pwd",
            Vec::new(),
            Vec::new(),
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Scalar),
        ),
        completion: None,
        handler: Rc::new(|context| {
            Box::pin(async move {
                Ok(CommandResult {
                    output: string_data(context.cwd),
                    display: DisplayPreference::Value,
                    notices: Vec::new(),
                    cwd: None,
                    exit: system_shell_contract::ShellExit::success(),
                })
            })
        }),
    }
}

fn path_completion_items(
    cwd: &str,
    raw_prefix: &str,
    directories_only: bool,
) -> LocalBoxFuture<'static, Result<Vec<CompletionItem>, ShellError>> {
    let cwd = cwd.to_string();
    let raw_prefix = raw_prefix.to_string();
    Box::pin(async move {
        let (dir_input, leaf_prefix) = if let Some((dir, leaf)) = raw_prefix.rsplit_once('/') {
            (dir.to_string(), leaf.to_string())
        } else {
            ("".to_string(), raw_prefix.clone())
        };
        let dir = if dir_input.is_empty() {
            cwd.clone()
        } else {
            normalize_session_path(&cwd, &dir_input)
        };
        let listing = platform_storage::explorer_list_dir(&dir)
            .await
            .map_err(unavailable)?;
        Ok(listing
            .entries
            .into_iter()
            .filter(|entry| {
                (!directories_only || entry.kind == ExplorerEntryKind::Directory)
                    && entry.name.starts_with(&leaf_prefix)
            })
            .map(|entry| CompletionItem {
                value: entry.path,
                label: entry.name,
                detail: Some(match entry.kind {
                    ExplorerEntryKind::File => "file".to_string(),
                    ExplorerEntryKind::Directory => "dir".to_string(),
                }),
            })
            .collect())
    })
}

fn cd_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: root_descriptor(
            "cd",
            &[],
            "Change the logical filesystem cwd.",
            "cd <path>",
            vec![CommandArgSpec {
                name: "path".to_string(),
                summary: "Target directory path.".to_string(),
                required: true,
                repeatable: false,
            }],
            Vec::new(),
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Empty),
        ),
        completion: Some(Rc::new(|request| {
            let raw = request.argv.get(1).cloned().unwrap_or_default();
            path_completion_items(&request.cwd, &raw, true)
        })),
        handler: Rc::new(|context| {
            Box::pin(async move {
                let target = context
                    .args
                    .first()
                    .ok_or_else(|| usage_error("usage: cd <path>"))?;
                let resolved = normalize_session_path(&context.cwd, target);
                let metadata = platform_storage::explorer_stat(&resolved)
                    .await
                    .map_err(unavailable)?;
                if metadata.kind != ExplorerEntryKind::Directory {
                    return Err(usage_error(format!("not a directory: `{resolved}`")));
                }
                let mut result = info_result(format!("cwd = {resolved}"));
                result.cwd = Some(resolved);
                Ok(result)
            })
        }),
    }
}

fn ls_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: root_descriptor(
            "ls",
            &[],
            "List a directory using the active explorer backend.",
            "ls [path]",
            vec![CommandArgSpec {
                name: "path".to_string(),
                summary: "Optional target directory.".to_string(),
                required: false,
                repeatable: false,
            }],
            Vec::new(),
            CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Table),
        ),
        completion: Some(Rc::new(|request| {
            let raw = request.argv.get(1).cloned().unwrap_or_default();
            path_completion_items(&request.cwd, &raw, false)
        })),
        handler: Rc::new(|context| {
            Box::pin(async move {
                let target = context
                    .args
                    .first()
                    .map(|path| normalize_session_path(&context.cwd, path))
                    .unwrap_or_else(|| context.cwd.clone());
                let listing = platform_storage::explorer_list_dir(&target)
                    .await
                    .map_err(unavailable)?;
                Ok(CommandResult {
                    output: table_data(
                        vec![
                            "name".to_string(),
                            "kind".to_string(),
                            "path".to_string(),
                            "size".to_string(),
                            "modified_at_unix_ms".to_string(),
                        ],
                        listing.entries.iter().map(explorer_row).collect(),
                        Some(CommandPath::new("ls")),
                    ),
                    display: DisplayPreference::Table,
                    notices: vec![CommandNotice {
                        level: CommandNoticeLevel::Info,
                        message: format!("listed {}", listing.cwd),
                    }],
                    cwd: None,
                    exit: system_shell_contract::ShellExit::success(),
                })
            })
        }),
    }
}

fn data_table_input(context: &AppCommandContext) -> Result<StructuredTable, ShellError> {
    match &context.input {
        StructuredData::Table(table) => Ok(table.clone()),
        StructuredData::Empty => Err(usage_error("command requires piped table input")),
        _ => Err(usage_error("command requires table-shaped piped input")),
    }
}

fn data_select_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            "data select",
            &[],
            "Select a subset of fields from records or tables.",
            "data select <field...>",
            vec![CommandArgSpec {
                name: "field".to_string(),
                summary: "Field names to keep.".to_string(),
                required: true,
                repeatable: true,
            }],
            Vec::new(),
            CommandInputShape::accepts(CommandDataShape::Any),
            CommandOutputShape::new(CommandDataShape::Any),
        ),
        completion: None,
        handler: Rc::new(|context| {
            Box::pin(async move {
                if context.args.is_empty() {
                    return Err(usage_error("usage: data select <field...>"));
                }
                match &context.input {
                    StructuredData::Table(table) => {
                        let rows = table
                            .rows
                            .iter()
                            .map(|row| StructuredRecord {
                                fields: context
                                    .args
                                    .iter()
                                    .filter_map(|name| {
                                        row.fields.iter().find(|field| &field.name == name).cloned()
                                    })
                                    .collect(),
                            })
                            .collect();
                        Ok(CommandResult {
                            output: table_data(
                                context.args.clone(),
                                rows,
                                Some(CommandPath::new("data select")),
                            ),
                            display: DisplayPreference::Table,
                            notices: Vec::new(),
                            cwd: None,
                            exit: system_shell_contract::ShellExit::success(),
                        })
                    }
                    StructuredData::Record(record) => Ok(CommandResult {
                        output: StructuredData::Record(StructuredRecord {
                            fields: context
                                .args
                                .iter()
                                .filter_map(|name| {
                                    record
                                        .fields
                                        .iter()
                                        .find(|field| &field.name == name)
                                        .cloned()
                                })
                                .collect(),
                        }),
                        display: DisplayPreference::Record,
                        notices: Vec::new(),
                        cwd: None,
                        exit: system_shell_contract::ShellExit::success(),
                    }),
                    StructuredData::Value(StructuredValue::Record(record)) => Ok(CommandResult {
                        output: StructuredData::Record(StructuredRecord {
                            fields: context
                                .args
                                .iter()
                                .filter_map(|name| {
                                    record
                                        .fields
                                        .iter()
                                        .find(|field| &field.name == name)
                                        .cloned()
                                })
                                .collect(),
                        }),
                        display: DisplayPreference::Record,
                        notices: Vec::new(),
                        cwd: None,
                        exit: system_shell_contract::ShellExit::success(),
                    }),
                    _ => Err(usage_error("data select expects record or table input")),
                }
            })
        }),
    }
}

fn field_value<'a>(record: &'a StructuredRecord, name: &str) -> Option<&'a StructuredValue> {
    record
        .fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| &field.value)
}

fn compare_scalar(left: &StructuredValue, right: &StructuredValue) -> Ordering {
    match (left, right) {
        (
            StructuredValue::Scalar(StructuredScalar::Int(left)),
            StructuredValue::Scalar(StructuredScalar::Int(right)),
        ) => left.cmp(right),
        (
            StructuredValue::Scalar(StructuredScalar::Float(left)),
            StructuredValue::Scalar(StructuredScalar::Float(right)),
        ) => left.partial_cmp(right).unwrap_or(Ordering::Equal),
        _ => display_structured_value(left).cmp(&display_structured_value(right)),
    }
}

fn data_sort_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            "data sort",
            &[],
            "Sort table rows by a field.",
            "data sort <field> [--desc]",
            vec![CommandArgSpec {
                name: "field".to_string(),
                summary: "Field to sort by.".to_string(),
                required: true,
                repeatable: false,
            }],
            Vec::new(),
            CommandInputShape::accepts(CommandDataShape::Table),
            CommandOutputShape::new(CommandDataShape::Table),
        ),
        completion: None,
        handler: Rc::new(|context| {
            Box::pin(async move {
                let field = context
                    .args
                    .first()
                    .ok_or_else(|| usage_error("usage: data sort <field> [--desc]"))?
                    .clone();
                let mut table = data_table_input(&context)?;
                let descending = context
                    .invocation
                    .options
                    .iter()
                    .any(|option| option.name == "desc");
                table.rows.sort_by(|left, right| {
                    let left_value = field_value(left, &field);
                    let right_value = field_value(right, &field);
                    let ord = match (left_value, right_value) {
                        (Some(left), Some(right)) => compare_scalar(left, right),
                        (Some(_), None) => Ordering::Greater,
                        (None, Some(_)) => Ordering::Less,
                        (None, None) => Ordering::Equal,
                    };
                    if descending {
                        ord.reverse()
                    } else {
                        ord
                    }
                });
                Ok(CommandResult {
                    output: StructuredData::Table(table),
                    display: DisplayPreference::Table,
                    notices: Vec::new(),
                    cwd: None,
                    exit: system_shell_contract::ShellExit::success(),
                })
            })
        }),
    }
}

fn predicate_matches(left: &StructuredValue, op: &str, right: &StructuredValue) -> bool {
    match op {
        "==" => left == right,
        "!=" => left != right,
        ">" => compare_scalar(left, right) == Ordering::Greater,
        ">=" => matches!(
            compare_scalar(left, right),
            Ordering::Greater | Ordering::Equal
        ),
        "<" => compare_scalar(left, right) == Ordering::Less,
        "<=" => matches!(
            compare_scalar(left, right),
            Ordering::Less | Ordering::Equal
        ),
        "contains" => display_structured_value(left).contains(&display_structured_value(right)),
        _ => false,
    }
}

fn data_where_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            "data where",
            &[],
            "Filter table rows by a field predicate.",
            "data where <field> <op> <value>",
            vec![
                CommandArgSpec {
                    name: "field".to_string(),
                    summary: "Field to inspect.".to_string(),
                    required: true,
                    repeatable: false,
                },
                CommandArgSpec {
                    name: "op".to_string(),
                    summary: "Predicate operator.".to_string(),
                    required: true,
                    repeatable: false,
                },
                CommandArgSpec {
                    name: "value".to_string(),
                    summary: "Expected value.".to_string(),
                    required: true,
                    repeatable: false,
                },
            ],
            Vec::new(),
            CommandInputShape::accepts(CommandDataShape::Table),
            CommandOutputShape::new(CommandDataShape::Table),
        ),
        completion: None,
        handler: Rc::new(|context| {
            Box::pin(async move {
                if context.args.len() < 3 {
                    return Err(usage_error("usage: data where <field> <op> <value>"));
                }
                let field = &context.args[0];
                let op = &context.args[1];
                let expected = context
                    .invocation
                    .values
                    .get(2)
                    .map(parsed_value_to_structured)
                    .unwrap_or_else(|| {
                        StructuredValue::Scalar(StructuredScalar::String(context.args[2].clone()))
                    });
                let table = data_table_input(&context)?;
                let rows = table
                    .rows
                    .into_iter()
                    .filter(|row| {
                        field_value(row, field)
                            .map(|value| predicate_matches(value, op, &expected))
                            .unwrap_or(false)
                    })
                    .collect();
                Ok(CommandResult {
                    output: table_data(table.columns, rows, Some(CommandPath::new("data where"))),
                    display: DisplayPreference::Table,
                    notices: Vec::new(),
                    cwd: None,
                    exit: system_shell_contract::ShellExit::success(),
                })
            })
        }),
    }
}

fn data_first_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            "data first",
            &[],
            "Take the first row or rows from table/list input.",
            "data first [count]",
            vec![CommandArgSpec {
                name: "count".to_string(),
                summary: "Number of items to keep.".to_string(),
                required: false,
                repeatable: false,
            }],
            Vec::new(),
            CommandInputShape::accepts(CommandDataShape::Any),
            CommandOutputShape::new(CommandDataShape::Any),
        ),
        completion: None,
        handler: Rc::new(|context| {
            Box::pin(async move {
                let count = context
                    .args
                    .first()
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(1);
                match &context.input {
                    StructuredData::Table(table) => Ok(CommandResult {
                        output: table_data(
                            table.columns.clone(),
                            table.rows.iter().take(count).cloned().collect(),
                            Some(CommandPath::new("data first")),
                        ),
                        display: DisplayPreference::Table,
                        notices: Vec::new(),
                        cwd: None,
                        exit: system_shell_contract::ShellExit::success(),
                    }),
                    StructuredData::List(values) => Ok(CommandResult {
                        output: StructuredData::List(values.iter().take(count).cloned().collect()),
                        display: DisplayPreference::Value,
                        notices: Vec::new(),
                        cwd: None,
                        exit: system_shell_contract::ShellExit::success(),
                    }),
                    _ => Err(usage_error("data first expects table or list input")),
                }
            })
        }),
    }
}

fn data_get_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: namespaced_descriptor(
            "data get",
            &[],
            "Extract one field from record or table input.",
            "data get <field>",
            vec![CommandArgSpec {
                name: "field".to_string(),
                summary: "Field name.".to_string(),
                required: true,
                repeatable: false,
            }],
            Vec::new(),
            CommandInputShape::accepts(CommandDataShape::Any),
            CommandOutputShape::new(CommandDataShape::Any),
        ),
        completion: None,
        handler: Rc::new(|context| {
            Box::pin(async move {
                let field = context
                    .args
                    .first()
                    .ok_or_else(|| usage_error("usage: data get <field>"))?;
                match &context.input {
                    StructuredData::Table(table) => Ok(CommandResult {
                        output: StructuredData::List(
                            table
                                .rows
                                .iter()
                                .filter_map(|row| field_value(row, field).cloned())
                                .collect(),
                        ),
                        display: DisplayPreference::Value,
                        notices: Vec::new(),
                        cwd: None,
                        exit: system_shell_contract::ShellExit::success(),
                    }),
                    StructuredData::Record(record) => {
                        let value = field_value(record, field)
                            .cloned()
                            .ok_or_else(|| usage_error(format!("missing field `{field}`")))?;
                        Ok(CommandResult {
                            output: StructuredData::Value(value),
                            display: DisplayPreference::Value,
                            notices: Vec::new(),
                            cwd: None,
                            exit: system_shell_contract::ShellExit::success(),
                        })
                    }
                    StructuredData::Value(StructuredValue::Record(record)) => {
                        let value = field_value(record, field)
                            .cloned()
                            .ok_or_else(|| usage_error(format!("missing field `{field}`")))?;
                        Ok(CommandResult {
                            output: StructuredData::Value(value),
                            display: DisplayPreference::Value,
                            notices: Vec::new(),
                            cwd: None,
                            exit: system_shell_contract::ShellExit::success(),
                        })
                    }
                    _ => Err(usage_error("data get expects record or table input")),
                }
            })
        }),
    }
}
