//! Development, prototype, and host-shell workflow commands.

mod config;
mod doctor;
mod server;
mod web;

use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};
use crate::XtaskCommand;
pub use doctor::DoctorOptions;
use doctor::{parse_doctor_options, print_doctor_usage, run_doctor};
use web::{
    print_tauri_usage, run_check_web, run_dev_command, tauri_build, tauri_check, tauri_dev,
    trunk_build, BuildProfile,
};

const DEV_SERVER_CONFIG_FILE: &str = "tools/automation/dev_server.toml";
const SITE_CARGO_FEATURE: &str = "csr";

/// `cargo setup-web`
pub struct SetupWebCommand;

impl XtaskCommand for SetupWebCommand {
    type Options = ();

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        if args.is_empty() {
            Ok(())
        } else {
            Err(XtaskError::validation(
                "`cargo setup-web` does not accept extra arguments",
            ))
        }
    }

    fn run(ctx: &CommandContext, _: Self::Options) -> XtaskResult<()> {
        ctx.process().run(
            ctx.root(),
            "rustup",
            vec!["target", "add", "wasm32-unknown-unknown"],
        )?;

        if ctx.process().command_available("trunk") {
            println!("trunk already installed");
            return Ok(());
        }

        ctx.process()
            .run(ctx.root(), "cargo", vec!["install", "trunk"])
    }
}

/// `cargo build-web`
pub struct BuildWebCommand;

impl XtaskCommand for BuildWebCommand {
    type Options = Vec<String>;

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        Ok(args.to_vec())
    }

    fn run(ctx: &CommandContext, args: Self::Options) -> XtaskResult<()> {
        trunk_build(ctx, args, BuildProfile::Release)
    }
}

/// `cargo check-web`
pub struct CheckWebCommand;

impl XtaskCommand for CheckWebCommand {
    type Options = ();

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        if args.is_empty() {
            Ok(())
        } else {
            Err(XtaskError::validation(
                "`cargo check-web` does not accept extra arguments",
            ))
        }
    }

    fn run(ctx: &CommandContext, _: Self::Options) -> XtaskResult<()> {
        run_check_web(ctx)
    }
}

/// `cargo tauri ...`
pub struct TauriCommand;

impl XtaskCommand for TauriCommand {
    type Options = Vec<String>;

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        Ok(args.to_vec())
    }

    fn run(ctx: &CommandContext, args: Self::Options) -> XtaskResult<()> {
        match args.first().map(String::as_str) {
            Some("dev") => tauri_dev(ctx, args[1..].to_vec()),
            Some("build") => tauri_build(ctx, args[1..].to_vec()),
            Some("check") => tauri_check(ctx),
            Some("help" | "--help" | "-h") | None => {
                print_tauri_usage();
                Ok(())
            }
            Some(other) => Err(XtaskError::validation(format!(
                "unknown tauri subcommand: {other}"
            ))),
        }
    }
}

/// `cargo doctor`
pub struct DoctorCommand;

impl XtaskCommand for DoctorCommand {
    type Options = DoctorOptions;

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        parse_doctor_options(args.to_vec())
    }

    fn run(ctx: &CommandContext, options: Self::Options) -> XtaskResult<()> {
        if options.show_help {
            print_doctor_usage();
            return Ok(());
        }
        run_doctor(ctx, options)
    }
}

/// `cargo dev ...`
pub struct DevCommand;

impl XtaskCommand for DevCommand {
    type Options = Vec<String>;

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        Ok(args.to_vec())
    }

    fn run(ctx: &CommandContext, args: Self::Options) -> XtaskResult<()> {
        run_dev_command(ctx, args)
    }
}

pub(crate) use server::wasm_target_installed;

#[cfg(test)]
mod tests {}
