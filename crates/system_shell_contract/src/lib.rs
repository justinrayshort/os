//! Shared shell command contracts used by the headless shell engine, runtime integration, and
//! terminal UI.
//!
//! This crate is intentionally runtime-agnostic. It defines serializable command metadata,
//! execution requests, completion payloads, and stream events without depending on Leptos,
//! browser APIs, or desktop runtime internals.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Stable command registration identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandId(String);

impl CommandId {
    /// Creates a command identifier from trusted caller input.
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    /// Returns the identifier text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable namespaced command path such as `apps.list`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandPath(String);

impl CommandPath {
    /// Creates a command path from trusted caller input.
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    /// Returns the path text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Execution identifier for a terminal command run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionId(pub u64);

/// Visibility policy for registered commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommandVisibility {
    /// Command is listed in help and completion.
    Public,
    /// Command is callable but omitted from normal listings.
    Hidden,
}

/// Registry scope for a command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CommandScope {
    /// Runtime-owned or globally visible command.
    Global,
    /// Commands exposed by an application package.
    App {
        /// Canonical application identifier.
        app_id: String,
    },
    /// Commands exposed only for a specific window instance.
    Window {
        /// Stable runtime window identifier.
        window_id: u64,
    },
}

/// Positional argument specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandArgSpec {
    /// Human-readable argument label.
    pub name: String,
    /// Short description.
    pub summary: String,
    /// Whether this argument is required.
    pub required: bool,
    /// Whether this argument consumes remaining values.
    pub repeatable: bool,
}

/// Named option or flag specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandOptionSpec {
    /// Long option name without leading `--`.
    pub name: String,
    /// Optional short option name without leading `-`.
    pub short: Option<char>,
    /// Short description.
    pub summary: String,
    /// Whether the option consumes a value.
    pub takes_value: bool,
}

/// Example invocation rendered in help output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandExample {
    /// Example command line.
    pub command: String,
    /// Example explanation.
    pub summary: String,
}

/// Complete help metadata for a registered command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelpDoc {
    /// Summary sentence.
    pub summary: String,
    /// Optional longer description.
    pub description: Option<String>,
    /// Usage string displayed in help output.
    pub usage: String,
    /// Example invocations.
    pub examples: Vec<CommandExample>,
}

/// Full command registration metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDescriptor {
    /// Stable command identifier.
    pub id: CommandId,
    /// Canonical command path.
    pub path: CommandPath,
    /// Alternate flat aliases, such as `ls`.
    pub aliases: Vec<String>,
    /// Registration scope.
    pub scope: CommandScope,
    /// Visibility policy.
    pub visibility: CommandVisibility,
    /// Positional argument metadata.
    pub args: Vec<CommandArgSpec>,
    /// Option metadata.
    pub options: Vec<CommandOptionSpec>,
    /// Help metadata.
    pub help: HelpDoc,
}

/// Completion request payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// Current cwd for the shell session.
    pub cwd: String,
    /// Full input line.
    pub line: String,
    /// Parsed argv tokens before the active cursor position.
    pub argv: Vec<String>,
    /// Cursor offset within `line`.
    pub cursor: usize,
    /// Optional source window identifier.
    pub source_window_id: Option<u64>,
}

/// One completion candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionItem {
    /// Text inserted into the input line.
    pub value: String,
    /// Human-readable label.
    pub label: String,
    /// Optional short description.
    pub detail: Option<String>,
}

/// Shell execution request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellRequest {
    /// Input line to parse and execute.
    pub line: String,
    /// Current logical cwd.
    pub cwd: String,
    /// Optional source window identifier.
    pub source_window_id: Option<u64>,
}

/// Final execution result metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellExecutionSummary {
    /// Execution identifier.
    pub execution_id: ExecutionId,
    /// Canonical command path if one was matched.
    pub command_path: Option<CommandPath>,
    /// Process-style exit metadata.
    pub exit: ShellExit,
}

/// Shell exit status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellExit {
    /// Numeric exit code.
    pub code: i32,
    /// Optional explanatory message.
    pub message: Option<String>,
}

impl ShellExit {
    /// Successful command completion.
    pub fn success() -> Self {
        Self {
            code: 0,
            message: None,
        }
    }

    /// Cancellation completion.
    pub fn cancelled() -> Self {
        Self {
            code: 130,
            message: Some("command cancelled".to_string()),
        }
    }
}

/// Structured shell error classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShellErrorCode {
    /// User input violated command usage.
    Usage,
    /// The command was not found.
    NotFound,
    /// The command is unavailable in this host context.
    Unavailable,
    /// The caller lacks permission to perform the action.
    PermissionDenied,
    /// Internal command or runtime failure.
    Internal,
}

/// Error emitted by shell parsing, lookup, or handlers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellError {
    /// Error category.
    pub code: ShellErrorCode,
    /// Human-readable message.
    pub message: String,
}

impl ShellError {
    /// Creates a new shell error.
    pub fn new(code: ShellErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Converts the error into a conventional exit code.
    pub fn exit_code(&self) -> i32 {
        match self.code {
            ShellErrorCode::Usage => 2,
            ShellErrorCode::NotFound => 3,
            ShellErrorCode::Unavailable | ShellErrorCode::PermissionDenied => 4,
            ShellErrorCode::Internal => 5,
        }
    }
}

/// Incremental stream event emitted by the shell runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ShellStreamEvent {
    /// Execution started.
    Started {
        /// Execution identifier.
        execution_id: ExecutionId,
    },
    /// Stdout chunk.
    StdoutChunk {
        /// Execution identifier.
        execution_id: ExecutionId,
        /// Text payload.
        text: String,
    },
    /// Stderr chunk.
    StderrChunk {
        /// Execution identifier.
        execution_id: ExecutionId,
        /// Text payload.
        text: String,
    },
    /// Human-readable status update.
    Status {
        /// Execution identifier.
        execution_id: ExecutionId,
        /// Status text.
        text: String,
    },
    /// Progress update in the `0.0..=1.0` range when known.
    Progress {
        /// Execution identifier.
        execution_id: ExecutionId,
        /// Optional progress ratio.
        value: Option<f32>,
        /// Optional short label.
        label: Option<String>,
    },
    /// Structured JSON payload.
    Json {
        /// Execution identifier.
        execution_id: ExecutionId,
        /// Structured payload.
        value: Value,
    },
    /// Execution completed successfully or with a command error.
    Completed {
        /// Summary payload.
        summary: ShellExecutionSummary,
    },
    /// Execution was cancelled.
    Cancelled {
        /// Execution identifier.
        execution_id: ExecutionId,
    },
}

/// Opaque registration token used to unregister commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CommandRegistrationToken(pub u64);
