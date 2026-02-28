//! Structured xtask error types.

use std::fmt::{self, Display, Formatter};
use std::path::Path;

/// Stable error categories for xtask workflows.
///
/// These categories are intentionally coarse. They are meant to keep user-facing failures
/// understandable and to support future machine-readable workflow summaries without exposing
/// command-specific internals in the type itself.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XtaskErrorCategory {
    /// Invalid or unreadable configuration.
    Config,
    /// Missing or mismatched local environment prerequisites.
    Environment,
    /// Failure to spawn a child process.
    ProcessLaunch,
    /// Child process exited unsuccessfully.
    ProcessExit,
    /// Invalid user input or semantically invalid workflow request.
    Validation,
    /// Filesystem or general I/O failure.
    Io,
    /// Workflow requested on an unsupported operating system.
    UnsupportedPlatform,
}

/// Structured xtask error with contextual metadata.
///
/// The formatted display output is intentionally CLI-friendly. Optional `operation`, `target`,
/// and `hint` fields can be attached as the error propagates so failures remain actionable at the
/// point they are shown to the user.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XtaskError {
    /// High-level error category.
    pub category: XtaskErrorCategory,
    /// Human-readable message.
    pub message: String,
    /// Optional operation name.
    pub operation: Option<String>,
    /// Optional path target.
    pub target: Option<String>,
    /// Optional remediation hint.
    pub hint: Option<String>,
}

/// Convenience result type for xtask internals.
pub type XtaskResult<T> = Result<T, XtaskError>;

impl XtaskError {
    /// Create an error with the given category and message.
    pub fn new(category: XtaskErrorCategory, message: impl Into<String>) -> Self {
        Self {
            category,
            message: message.into(),
            operation: None,
            target: None,
            hint: None,
        }
    }

    /// Create a configuration error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::new(XtaskErrorCategory::Config, message)
    }

    /// Create an environment error.
    pub fn environment(message: impl Into<String>) -> Self {
        Self::new(XtaskErrorCategory::Environment, message)
    }

    /// Create a process launch error.
    pub fn process_launch(message: impl Into<String>) -> Self {
        Self::new(XtaskErrorCategory::ProcessLaunch, message)
    }

    /// Create a process exit error.
    pub fn process_exit(message: impl Into<String>) -> Self {
        Self::new(XtaskErrorCategory::ProcessExit, message)
    }

    /// Create a validation error.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(XtaskErrorCategory::Validation, message)
    }

    /// Create an IO error.
    pub fn io(message: impl Into<String>) -> Self {
        Self::new(XtaskErrorCategory::Io, message)
    }

    /// Create an unsupported-platform error.
    pub fn unsupported_platform(message: impl Into<String>) -> Self {
        Self::new(XtaskErrorCategory::UnsupportedPlatform, message)
    }

    /// Attach an operation label.
    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }

    /// Attach a target path.
    pub fn with_path(mut self, path: &Path) -> Self {
        self.target = Some(path.display().to_string());
        self
    }

    /// Attach a remediation hint.
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

impl Display for XtaskError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(operation) = &self.operation {
            write!(f, " [operation: {operation}]")?;
        }
        if let Some(target) = &self.target {
            write!(f, " [target: {target}]")?;
        }
        if let Some(hint) = &self.hint {
            write!(f, " [hint: {hint}]")?;
        }
        Ok(())
    }
}

impl std::error::Error for XtaskError {}

impl From<std::io::Error> for XtaskError {
    fn from(value: std::io::Error) -> Self {
        XtaskError::io(value.to_string())
    }
}
