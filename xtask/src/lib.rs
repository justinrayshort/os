//! Workspace maintenance and developer workflow commands (`cargo xtask`).
//!
//! The crate is organized as a small CLI layer over a shared Rust-native automation runtime.
//! Command modules own workflow-specific policy while [`runtime`] owns process execution,
//! artifact handling, workflow recording, and environment normalization.

pub mod cli;
pub mod commands;
pub mod docs;
pub mod runtime;
pub mod wiki_config;

use crate::cli::TopLevelCommand;
use crate::commands::cache::CacheCommand;
use crate::commands::dev::{
    BuildWebCommand, CheckWebCommand, DevCommand, DoctorCommand, SetupWebCommand, TauriCommand,
};
use crate::commands::docs::DocsCommand;
use crate::commands::e2e::E2eCommand;
use crate::commands::perf::PerfCommand;
use crate::commands::verify::{FlowCommand, VerifyCommand};
use crate::commands::wiki::WikiCommand;
use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};

/// Shared command contract for top-level xtask command families.
///
/// Each top-level workflow family owns its own typed option parsing while sharing the same
/// runtime services through [`CommandContext`]. Implementations should treat
/// [`XtaskCommand::parse`] as a pure translation step from raw CLI arguments into a typed options
/// value and keep side effects in [`XtaskCommand::run`].
///
/// This trait is intentionally small: command domains are free to organize internal subcommands
/// however they need, but they should not bypass the shared runtime once execution starts.
pub trait XtaskCommand {
    /// Typed options produced by CLI parsing for the command family.
    type Options;

    /// Parse command-line arguments into typed options.
    ///
    /// Implementations should return [`XtaskError::validation`](crate::runtime::error::XtaskError::validation)
    /// for invalid user-facing argument shapes.
    fn parse(args: &[String]) -> XtaskResult<Self::Options>;

    /// Execute the command family using the shared runtime context.
    ///
    /// Implementations should prefer `ctx.process()`, `ctx.workflow()`, `ctx.artifacts()`, and
    /// `ctx.workspace()` over constructing command-local helpers.
    fn run(ctx: &CommandContext, options: Self::Options) -> XtaskResult<()>;
}

/// Executes the `xtask` binary using the current process arguments.
///
/// This is the library-backed entrypoint used by [`xtask/src/bin/xtask.rs`](crate). It creates a
/// fresh [`CommandContext`], parses the top-level command selection, and delegates to the owning
/// command family.
pub fn execute_from_env() -> XtaskResult<()> {
    let parsed = cli::parse(std::env::args().skip(1).collect())?;
    let ctx = CommandContext::new()?;

    match parsed {
        TopLevelCommand::Cache(args) => CacheCommand::run(&ctx, CacheCommand::parse(&args)?),
        TopLevelCommand::SetupWeb(args) => {
            SetupWebCommand::run(&ctx, SetupWebCommand::parse(&args)?)
        }
        TopLevelCommand::Dev(args) => DevCommand::run(&ctx, DevCommand::parse(&args)?),
        TopLevelCommand::BuildWeb(args) => {
            BuildWebCommand::run(&ctx, BuildWebCommand::parse(&args)?)
        }
        TopLevelCommand::CheckWeb(args) => {
            CheckWebCommand::run(&ctx, CheckWebCommand::parse(&args)?)
        }
        TopLevelCommand::Tauri(args) => TauriCommand::run(&ctx, TauriCommand::parse(&args)?),
        TopLevelCommand::Flow(args) => FlowCommand::run(&ctx, FlowCommand::parse(&args)?),
        TopLevelCommand::Doctor(args) => DoctorCommand::run(&ctx, DoctorCommand::parse(&args)?),
        TopLevelCommand::Docs(args) => DocsCommand::run(&ctx, DocsCommand::parse(&args)?),
        TopLevelCommand::E2e(args) => E2eCommand::run(&ctx, E2eCommand::parse(&args)?),
        TopLevelCommand::Perf(args) => PerfCommand::run(&ctx, PerfCommand::parse(&args)?),
        TopLevelCommand::Verify(args) => VerifyCommand::run(&ctx, VerifyCommand::parse(&args)?),
        TopLevelCommand::Wiki(args) => WikiCommand::run(&ctx, WikiCommand::parse(&args)?),
        TopLevelCommand::Help => {
            cli::print_usage();
            Ok(())
        }
    }
}

/// Converts an xtask result into a stable process exit code.
///
/// All command failures currently map to exit code `1` after printing the formatted
/// [`XtaskError`] to stderr.
pub fn exit_code(result: XtaskResult<()>) -> std::process::ExitCode {
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            std::process::ExitCode::from(1)
        }
    }
}

impl From<String> for XtaskError {
    fn from(value: String) -> Self {
        XtaskError::validation(value)
    }
}
