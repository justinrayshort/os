//! Verification and changed-scope workflow commands.

mod changed;
mod config;
mod flow;
mod run;

use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};
use crate::XtaskCommand;
use config::{
    load_verify_profiles, parse_verify_options, print_verify_usage,
    resolve_verify_options_from_profile, VerifyFastDesktopMode, VerifyMode, VerifyOptions,
};
pub use flow::FlowOptions;
use flow::{flow_command_inner, parse_flow_options, print_flow_usage};
use run::run_verify;

/// `cargo verify`
pub struct VerifyCommand;

impl XtaskCommand for VerifyCommand {
    type Options = VerifyOptions;

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        parse_verify_options(args.to_vec())
    }

    fn run(ctx: &CommandContext, mut options: Self::Options) -> XtaskResult<()> {
        if options.show_help {
            let profiles = load_verify_profiles(ctx).ok();
            print_verify_usage(profiles.as_ref());
            return Ok(());
        }

        let profiles = load_verify_profiles(ctx)?;
        options = resolve_verify_options_from_profile(options, &profiles)?;
        if options.mode == VerifyMode::Full && options.desktop_mode != VerifyFastDesktopMode::Auto {
            return Err(XtaskError::validation(
                "`--with-desktop` and `--without-desktop` are only valid with `cargo verify-fast`",
            ));
        }

        let run_profile = options.profile.clone();
        ctx.workflow().with_workflow_run("verify", run_profile, || {
            run_verify(ctx, options.mode, options.desktop_mode)
        })
    }
}

/// `cargo flow`
pub struct FlowCommand;

impl XtaskCommand for FlowCommand {
    type Options = FlowOptions;

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        parse_flow_options(args.to_vec())
    }

    fn run(ctx: &CommandContext, options: Self::Options) -> XtaskResult<()> {
        if options.show_help {
            print_flow_usage();
            return Ok(());
        }
        flow_command_inner(ctx, options)
    }
}
