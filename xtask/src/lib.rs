//! Workspace maintenance and developer workflow commands (`cargo xtask`).
//!
//! The crate is organized as a small CLI layer over a shared Rust-native automation runtime.
//! Command modules own workflow-specific policy while [`runtime`] owns process execution,
//! artifact handling, workflow recording, and environment normalization.

pub mod cli;
pub mod commands;
pub mod docs;
pub mod runtime;

use crate::cli::TopLevelCommand;
use crate::commands::dev::{
    BuildWebCommand, CheckWebCommand, DevCommand, DoctorCommand, SetupWebCommand, TauriCommand,
};
use crate::commands::docs::DocsCommand;
use crate::commands::perf::PerfCommand;
use crate::commands::verify::{FlowCommand, VerifyCommand};
use crate::commands::wiki::WikiCommand;
use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};

/// Shared command contract for top-level xtask command families.
pub trait XtaskCommand {
    /// Typed options produced by CLI parsing for the command family.
    type Options;

    /// Parse command-line arguments into typed options.
    fn parse(args: &[String]) -> XtaskResult<Self::Options>;

    /// Execute the command family using the shared runtime context.
    fn run(ctx: &CommandContext, options: Self::Options) -> XtaskResult<()>;
}

/// Executes the `xtask` binary using the current process arguments.
pub fn execute_from_env() -> XtaskResult<()> {
    let parsed = cli::parse(std::env::args().skip(1).collect())?;
    let ctx = CommandContext::new()?;

    match parsed {
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
