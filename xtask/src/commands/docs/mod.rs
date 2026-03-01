//! Documentation validation command family.

mod audit;
mod frontmatter;
mod links;
mod mermaid;
mod openapi;
mod structure;
mod ui_conformance;
mod wiki;

use crate::runtime::context::CommandContext;
use crate::runtime::error::XtaskResult;
use crate::XtaskCommand;
use std::path::PathBuf;

/// `cargo docs ...`
pub struct DocsCommand;

impl XtaskCommand for DocsCommand {
    type Options = Vec<String>;

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        Ok(args.to_vec())
    }

    fn run(ctx: &CommandContext, args: Self::Options) -> XtaskResult<()> {
        match args.first().map(String::as_str) {
            Some("structure") => structure::validate(ctx),
            Some("wiki") => wiki::validate(ctx),
            Some("frontmatter") => frontmatter::validate(ctx),
            Some("links") => links::validate(ctx),
            Some("mermaid") => mermaid::validate(ctx),
            Some("openapi") => openapi::validate(ctx),
            Some("ui-conformance") => ui_conformance::validate(ctx),
            Some("all") => run_subcommand(ctx, args),
            Some("audit-report") => {
                let Some(path) = args
                    .windows(2)
                    .find(|window| window[0] == "--output")
                    .map(|window| PathBuf::from(&window[1]))
                else {
                    return Err(crate::runtime::error::XtaskError::validation(
                        "missing `--output <path>`",
                    ));
                };
                let _ = path;
                run_subcommand(ctx, args)
            }
            _ => crate::docs::run_docs_command(ctx.root(), args),
        }
    }
}

pub(crate) fn run_all(ctx: &CommandContext) -> XtaskResult<()> {
    audit::run_all(ctx)
}

pub(crate) fn write_report(ctx: &CommandContext, output: PathBuf) -> XtaskResult<()> {
    audit::write_report(ctx, output)
}

fn run_subcommand(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    crate::docs::run_docs_command(ctx.root(), args)
}

fn output_args(output: impl Into<PathBuf>) -> Vec<String> {
    vec!["--output".into(), output.into().display().to_string()]
}
