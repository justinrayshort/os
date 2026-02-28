use super::{output_args, run_subcommand};
use crate::runtime::context::CommandContext;
use crate::runtime::error::XtaskResult;
use std::path::PathBuf;

pub(crate) fn run_all(ctx: &CommandContext) -> XtaskResult<()> {
    run_subcommand(ctx, vec!["all".into()])
}

pub(crate) fn write_report(ctx: &CommandContext, output: PathBuf) -> XtaskResult<()> {
    let mut args = vec!["audit-report".into()];
    args.extend(output_args(output));
    run_subcommand(ctx, args)
}
