use super::run_subcommand;
use crate::runtime::context::CommandContext;
use crate::runtime::error::XtaskResult;

pub(crate) fn validate(ctx: &CommandContext) -> XtaskResult<()> {
    run_subcommand(ctx, vec!["ui-conformance".into()])
}
