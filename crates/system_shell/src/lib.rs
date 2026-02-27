//! Runtime-agnostic browser-native shell engine with dynamic command registration.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

use futures::future::LocalBoxFuture;
use leptos::{
    create_rw_signal, ReadSignal, RwSignal, SignalGetUntracked, SignalSet, SignalUpdate,
};
use serde_json::Value;
use shrs_core_headless::{eval_line, HeadlessEvalInput, HeadlessShellState};
use system_shell_contract::{
    CommandDescriptor, CommandRegistrationToken, CommandScope, CompletionItem, CompletionRequest,
    ExecutionId, ShellError, ShellErrorCode, ShellExecutionSummary, ShellExit, ShellRequest,
    ShellStreamEvent,
};

/// Async completion provider.
pub type CompletionHandler =
    Rc<dyn Fn(CompletionRequest) -> LocalBoxFuture<'static, Result<Vec<CompletionItem>, ShellError>>>;

/// Async command handler.
pub type CommandHandler =
    Rc<dyn Fn(CommandExecutionContext) -> LocalBoxFuture<'static, Result<ShellExit, ShellError>>>;

/// Shared command execution context for handlers.
#[derive(Clone)]
pub struct CommandExecutionContext {
    /// Parsed execution identifier.
    pub execution_id: ExecutionId,
    /// Canonical descriptor for the resolved command.
    pub descriptor: CommandDescriptor,
    /// Full argv for the command line.
    pub argv: Vec<String>,
    /// Current logical cwd.
    pub cwd: String,
    /// Optional source window identifier.
    pub source_window_id: Option<u64>,
    emitter: EventEmitter,
    session_cwd: RwSignal<String>,
    cancelled: Rc<Cell<bool>>,
}

impl CommandExecutionContext {
    /// Emits a stdout chunk.
    pub fn stdout(&self, text: impl Into<String>) {
        self.emitter.stdout(self.execution_id, text.into());
    }

    /// Emits a stderr chunk.
    pub fn stderr(&self, text: impl Into<String>) {
        self.emitter.stderr(self.execution_id, text.into());
    }

    /// Emits a status update.
    pub fn status(&self, text: impl Into<String>) {
        self.emitter.status(self.execution_id, text.into());
    }

    /// Emits a structured JSON payload.
    pub fn json(&self, value: Value) {
        self.emitter.json(self.execution_id, value);
    }

    /// Updates the logical cwd for the active session.
    pub fn set_cwd(&self, cwd: impl Into<String>) {
        self.session_cwd.set(cwd.into());
    }

    /// Returns whether the foreground execution has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.get()
    }
}

#[derive(Clone)]
struct EventEmitter {
    events: RwSignal<Vec<ShellStreamEvent>>,
}

impl EventEmitter {
    fn push(&self, event: ShellStreamEvent) {
        self.events.update(|events| events.push(event));
    }

    fn stdout(&self, execution_id: ExecutionId, text: String) {
        self.push(ShellStreamEvent::StdoutChunk { execution_id, text });
    }

    fn stderr(&self, execution_id: ExecutionId, text: String) {
        self.push(ShellStreamEvent::StderrChunk { execution_id, text });
    }

    fn status(&self, execution_id: ExecutionId, text: String) {
        self.push(ShellStreamEvent::Status { execution_id, text });
    }

    fn json(&self, execution_id: ExecutionId, value: Value) {
        self.push(ShellStreamEvent::Json { execution_id, value });
    }
}

#[derive(Clone)]
struct RegisteredCommand {
    descriptor: CommandDescriptor,
    completion: Option<CompletionHandler>,
    handler: CommandHandler,
}

#[derive(Default)]
struct RegistryState {
    next_token: u64,
    by_token: BTreeMap<CommandRegistrationToken, RegisteredCommand>,
}

/// Shared command registry.
#[derive(Clone, Default)]
pub struct CommandRegistry {
    state: Rc<RefCell<RegistryState>>,
}

impl CommandRegistry {
    /// Registers one command and returns its registration token.
    pub fn register(
        &self,
        descriptor: CommandDescriptor,
        completion: Option<CompletionHandler>,
        handler: CommandHandler,
    ) -> CommandRegistrationToken {
        let mut state = self.state.borrow_mut();
        state.next_token = state.next_token.saturating_add(1);
        let token = CommandRegistrationToken(state.next_token);
        state.by_token.insert(
            token,
            RegisteredCommand {
                descriptor,
                completion,
                handler,
            },
        );
        token
    }

    /// Removes a previously registered command token.
    pub fn unregister(&self, token: CommandRegistrationToken) {
        self.state.borrow_mut().by_token.remove(&token);
    }

    fn visible_commands(&self) -> Vec<RegisteredCommand> {
        self.state.borrow().by_token.values().cloned().collect()
    }

    /// Returns the currently registered command descriptors.
    pub fn descriptors(&self) -> Vec<CommandDescriptor> {
        self.visible_commands()
            .into_iter()
            .map(|registered| registered.descriptor)
            .collect()
    }
}

/// Drop-based registration handle.
#[derive(Clone)]
pub struct CommandRegistryHandle {
    registry: CommandRegistry,
    token: CommandRegistrationToken,
    active: Rc<Cell<bool>>,
}

impl CommandRegistryHandle {
    /// Unregisters the command if it is still active.
    pub fn unregister(&self) {
        if self.active.replace(false) {
            self.registry.unregister(self.token);
        }
    }
}

impl Drop for CommandRegistryHandle {
    fn drop(&mut self) {
        self.unregister();
    }
}

#[derive(Clone)]
struct SessionState {
    parser_state: Rc<RefCell<HeadlessShellState>>,
    cwd: RwSignal<String>,
    events: RwSignal<Vec<ShellStreamEvent>>,
    active_execution: RwSignal<Option<ExecutionId>>,
    next_execution_id: Rc<Cell<u64>>,
    cancel_flag: Rc<Cell<bool>>,
}

/// A shell session with one foreground execution slot.
#[derive(Clone)]
pub struct ShellSessionHandle {
    state: SessionState,
    registry: CommandRegistry,
}

impl ShellSessionHandle {
    /// Reactive stream event log for this session.
    pub fn events(&self) -> ReadSignal<Vec<ShellStreamEvent>> {
        self.state.events.read_only()
    }

    /// Reactive active execution id for this session.
    pub fn active_execution(&self) -> ReadSignal<Option<ExecutionId>> {
        self.state.active_execution.read_only()
    }

    /// Reactive current cwd for this session.
    pub fn cwd(&self) -> ReadSignal<String> {
        self.state.cwd.read_only()
    }

    /// Cancels the active foreground execution.
    pub fn cancel(&self) {
        if self.state.active_execution.get_untracked().is_some() {
            self.state.cancel_flag.set(true);
        }
    }

    /// Resolves completion candidates for the current input.
    pub async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<Vec<CompletionItem>, ShellError> {
        let commands = self.registry.visible_commands();
        if request.argv.len() <= 1 {
            let prefix = request.argv.first().cloned().unwrap_or_default();
            let mut items = Vec::new();
            for registered in commands {
                let descriptor = registered.descriptor;
                let summary = descriptor.help.summary.clone();
                if descriptor.path.as_str().starts_with(&prefix) {
                    items.push(CompletionItem {
                        value: descriptor.path.as_str().to_string(),
                        label: descriptor.path.as_str().to_string(),
                        detail: Some(summary.clone()),
                    });
                }
                for alias in descriptor.aliases {
                    if alias.starts_with(&prefix) {
                        items.push(CompletionItem {
                            value: alias.clone(),
                            label: alias,
                            detail: Some(summary.clone()),
                        });
                    }
                }
            }
            return Ok(items);
        }

        let matched = self.resolve_command(&request.argv[0])?;
        if let Some(completion) = matched.completion {
            return completion(request).await;
        }
        Ok(Vec::new())
    }

    /// Parses and executes one command request.
    pub fn submit(&self, request: ShellRequest) {
        if self.state.active_execution.get_untracked().is_some() {
            self.state.events.update(|events| {
                events.push(ShellStreamEvent::Status {
                    execution_id: ExecutionId(0),
                    text: "another command is already running".to_string(),
                });
            });
            return;
        }

        let parsed = {
            let mut parser_state = self.state.parser_state.borrow_mut();
            eval_line(
                &mut parser_state,
                HeadlessEvalInput {
                    line: request.line.clone(),
                },
            )
        };

        let output = match parsed {
            Ok(output) => output,
            Err(err) => {
                let execution_id = self.next_execution_id();
                self.state.events.update(|events| {
                    events.push(ShellStreamEvent::Started { execution_id });
                    events.push(ShellStreamEvent::StderrChunk {
                        execution_id,
                        text: err.message.clone(),
                    });
                    events.push(ShellStreamEvent::Completed {
                        summary: ShellExecutionSummary {
                            execution_id,
                            command_path: None,
                            exit: ShellExit {
                                code: 2,
                                message: Some(err.message),
                            },
                        },
                    });
                });
                return;
            }
        };

        if output.is_empty {
            return;
        }

        let execution_id = self.next_execution_id();
        self.state.cancel_flag.set(false);
        self.state.active_execution.set(Some(execution_id));
        let descriptor_result = self.resolve_command(&output.argv[0]);
        let state = self.state.clone();
        let registry = self.registry.clone();
        let request_cwd = request.cwd.clone();
        leptos::spawn_local(async move {
            let emitter = EventEmitter {
                events: state.events,
            };
            emitter.push(ShellStreamEvent::Started { execution_id });

            match descriptor_result {
                Ok(registered) => {
                    let command_path = registered.descriptor.path.clone();
                    if output.wants_help && output.argv[0] != "help" {
                        emitter.stdout(
                            execution_id,
                            render_help(&registered.descriptor),
                        );
                        emitter.push(ShellStreamEvent::Completed {
                            summary: ShellExecutionSummary {
                                execution_id,
                                command_path: Some(command_path.clone()),
                                exit: ShellExit::success(),
                            },
                        });
                    } else {
                        let context = CommandExecutionContext {
                            execution_id,
                            descriptor: registered.descriptor.clone(),
                            argv: output.argv.clone(),
                            cwd: request_cwd,
                            source_window_id: request.source_window_id,
                            emitter: emitter.clone(),
                            session_cwd: state.cwd,
                            cancelled: state.cancel_flag.clone(),
                        };
                        let result = (registered.handler)(context).await;
                        if state.cancel_flag.get() {
                            emitter.push(ShellStreamEvent::Cancelled { execution_id });
                            emitter.push(ShellStreamEvent::Completed {
                                summary: ShellExecutionSummary {
                                    execution_id,
                                    command_path: Some(command_path.clone()),
                                    exit: ShellExit::cancelled(),
                                },
                            });
                        } else {
                            match result {
                                Ok(exit) => emitter.push(ShellStreamEvent::Completed {
                                    summary: ShellExecutionSummary {
                                        execution_id,
                                        command_path: Some(command_path.clone()),
                                        exit,
                                    },
                                }),
                                Err(err) => {
                                    emitter.stderr(execution_id, err.message.clone());
                                    emitter.push(ShellStreamEvent::Completed {
                                        summary: ShellExecutionSummary {
                                            execution_id,
                                            command_path: Some(command_path.clone()),
                                            exit: ShellExit {
                                                code: err.exit_code(),
                                                message: Some(err.message),
                                            },
                                        },
                                    });
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    emitter.stderr(execution_id, err.message.clone());
                    emitter.push(ShellStreamEvent::Completed {
                        summary: ShellExecutionSummary {
                            execution_id,
                            command_path: None,
                            exit: ShellExit {
                                code: err.exit_code(),
                                message: Some(err.message),
                            },
                        },
                    });
                }
            }

            state.active_execution.set(None);
            let _ = registry;
        });
    }

    fn next_execution_id(&self) -> ExecutionId {
        let next = self.state.next_execution_id.get().saturating_add(1);
        self.state.next_execution_id.set(next);
        ExecutionId(next)
    }

    fn resolve_command(&self, token: &str) -> Result<RegisteredCommand, ShellError> {
        let commands = self.registry.visible_commands();
        let mut best_scope_rank = None::<u8>;
        let mut matches = Vec::new();

        for registered in commands {
            let matches_token = registered.descriptor.path.as_str() == token
                || registered.descriptor.aliases.iter().any(|alias| alias == token);
            if !matches_token {
                continue;
            }

            let rank = match registered.descriptor.scope {
                CommandScope::Window { .. } => 3,
                CommandScope::App { .. } => 2,
                CommandScope::Global => 1,
            };
            match best_scope_rank {
                Some(best) if rank < best => continue,
                Some(best) if rank > best => {
                    best_scope_rank = Some(rank);
                    matches.clear();
                }
                None => best_scope_rank = Some(rank),
                _ => {}
            }
            matches.push(registered);
        }

        match matches.len() {
            0 => Err(ShellError::new(
                ShellErrorCode::NotFound,
                format!("command not found: {token}"),
            )),
            1 => Ok(matches.remove(0)),
            _ => Err(ShellError::new(
                ShellErrorCode::Usage,
                format!("ambiguous command `{token}`"),
            )),
        }
    }
}

fn render_help(descriptor: &CommandDescriptor) -> String {
    let mut lines = vec![
        descriptor.path.as_str().to_string(),
        descriptor.help.summary.clone(),
        format!("Usage: {}", descriptor.help.usage),
    ];
    if !descriptor.aliases.is_empty() {
        lines.push(format!("Aliases: {}", descriptor.aliases.join(", ")));
    }
    if !descriptor.args.is_empty() {
        lines.push("Arguments:".to_string());
        for arg in &descriptor.args {
            lines.push(format!("  {} - {}", arg.name, arg.summary));
        }
    }
    if !descriptor.options.is_empty() {
        lines.push("Options:".to_string());
        for option in &descriptor.options {
            let short = option
                .short
                .map(|short| format!("-{}, ", short))
                .unwrap_or_default();
            lines.push(format!("  {}--{} - {}", short, option.name, option.summary));
        }
    }
    if !descriptor.help.examples.is_empty() {
        lines.push("Examples:".to_string());
        for example in &descriptor.help.examples {
            lines.push(format!("  {}  # {}", example.command, example.summary));
        }
    }
    lines.join("\n")
}

/// Root shell engine used by the runtime.
#[derive(Clone, Default)]
pub struct ShellEngine {
    registry: CommandRegistry,
}

impl ShellEngine {
    /// Creates a new shared shell engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the shared registry.
    pub fn registry(&self) -> CommandRegistry {
        self.registry.clone()
    }

    /// Returns all currently visible command descriptors.
    pub fn descriptors(&self) -> Vec<CommandDescriptor> {
        self.registry.descriptors()
    }

    /// Registers a command and returns a drop-based handle.
    pub fn register_command(
        &self,
        descriptor: CommandDescriptor,
        completion: Option<CompletionHandler>,
        handler: CommandHandler,
    ) -> CommandRegistryHandle {
        let token = self.registry.register(descriptor, completion, handler);
        CommandRegistryHandle {
            registry: self.registry.clone(),
            token,
            active: Rc::new(Cell::new(true)),
        }
    }

    /// Creates one shell session with its own cwd and event stream.
    pub fn new_session(&self, cwd: impl Into<String>) -> ShellSessionHandle {
        let cwd = cwd.into();
        let state = SessionState {
            parser_state: Rc::new(RefCell::new(HeadlessShellState::default())),
            cwd: create_rw_signal(cwd),
            events: create_rw_signal(Vec::new()),
            active_execution: create_rw_signal(None),
            next_execution_id: Rc::new(Cell::new(0)),
            cancel_flag: Rc::new(Cell::new(false)),
        };
        ShellSessionHandle {
            state,
            registry: self.registry.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use system_shell_contract::{
        CommandArgSpec, CommandExample, CommandId, CommandOptionSpec, CommandPath,
        CommandVisibility, HelpDoc,
    };

    fn descriptor(path: &str, aliases: &[&str], scope: CommandScope) -> CommandDescriptor {
        CommandDescriptor {
            id: CommandId::new(path),
            path: CommandPath::new(path),
            aliases: aliases.iter().map(|alias| alias.to_string()).collect(),
            scope,
            visibility: CommandVisibility::Public,
            args: vec![CommandArgSpec {
                name: "value".to_string(),
                summary: "value".to_string(),
                required: false,
                repeatable: false,
            }],
            options: vec![CommandOptionSpec {
                name: "help".to_string(),
                short: Some('h'),
                summary: "show help".to_string(),
                takes_value: false,
            }],
            help: HelpDoc {
                summary: "summary".to_string(),
                description: None,
                usage: path.to_string(),
                examples: vec![CommandExample {
                    command: path.to_string(),
                    summary: "example".to_string(),
                }],
            },
        }
    }

    #[test]
    fn registration_handle_unregisters() {
        leptos::create_runtime();
        let engine = ShellEngine::new();
        let handle = engine.register_command(
            descriptor("apps.list", &[], CommandScope::Global),
            None,
            Rc::new(|_| Box::pin(async { Ok(ShellExit::success()) })),
        );
        assert_eq!(engine.registry.visible_commands().len(), 1);
        handle.unregister();
        assert_eq!(engine.registry.visible_commands().len(), 0);
    }
}
