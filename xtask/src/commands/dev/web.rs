use super::server::{
    dev_server_foreground, dev_server_logs, dev_server_start, dev_server_status, dev_server_stop,
    wasm_target_installed,
};
use super::SITE_CARGO_FEATURE;
use crate::runtime::context::CommandContext;
use crate::runtime::error::XtaskResult;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BuildProfile {
    Dev,
    Release,
}

pub(super) fn print_dev_usage() {
    eprintln!(
        "Usage: cargo dev [serve|start|stop|status|logs|restart|build] [args]\n\
         \n\
         Subcommands:\n\
           (default)           Start trunk dev server in foreground (defaults to --open)\n\
           serve [trunk args]  Same as default foreground mode\n\
           start [trunk args]  Start trunk dev server in background (no browser open by default)\n\
           stop                Stop the managed background dev server\n\
           status              Show managed background dev server status\n\
           logs [--lines N]    Print recent managed dev server log output\n\
           restart [args]      Restart the managed background dev server\n\
           build [args]        Build a dev static bundle via trunk (non-release)\n\
         \n\
         Notes:\n\
           - `cargo dev stop` only manages servers started with `cargo dev start`.\n\
           - Development serve/build defaults disable SRI; file hashing stays enabled unless explicitly overridden.\n\
           - Logs and state are stored under `.artifacts/dev-server/`.\n"
    );
}

pub(super) fn print_tauri_usage() {
    eprintln!(
        "Usage: cargo xtask tauri <dev|build|check> [args]\n\
         \n\
         Subcommands:\n\
           dev [args]    Run `cargo tauri dev`\n\
           build [args]  Run `cargo tauri build`\n\
           check         Validate desktop_tauri compiles\n"
    );
}

pub(crate) fn site_dir(root: &Path) -> PathBuf {
    root.join("crates/site")
}

fn tauri_dir(root: &Path) -> PathBuf {
    root.join("crates/desktop_tauri")
}

pub(super) fn trunk_build(
    ctx: &CommandContext,
    args: Vec<String>,
    profile: BuildProfile,
) -> XtaskResult<()> {
    ctx.process().ensure_command(
        "trunk",
        "Install it with `cargo setup-web` (or `cargo install trunk`)",
    )?;

    let mut trunk_args = vec!["build".to_string(), "index.html".to_string()];
    if profile == BuildProfile::Release {
        trunk_args.push("--release".to_string());
    }
    if !args_specify_dist(&args) {
        trunk_args.push("--dist".to_string());
        trunk_args.push(
            match profile {
                BuildProfile::Dev => "target/trunk-dev-dist",
                BuildProfile::Release => "target/trunk-dist",
            }
            .to_string(),
        );
    }
    if profile == BuildProfile::Dev && !args_specify_no_sri(&args) {
        trunk_args.push("--no-sri=true".to_string());
    }
    trunk_args.extend(args);
    ctx.process().run_trunk(site_dir(ctx.root()), trunk_args)
}

pub(super) fn tauri_dev(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    ctx.process().run_tauri_cli(
        &tauri_dir(ctx.root()),
        prepend_tauri_subcommand("tauri", "dev", args),
    )
}

pub(super) fn tauri_build(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    ctx.process().run_tauri_cli(
        &tauri_dir(ctx.root()),
        prepend_tauri_subcommand("tauri", "build", args),
    )
}

pub(super) fn tauri_check(ctx: &CommandContext) -> XtaskResult<()> {
    ctx.process()
        .run(ctx.root(), "cargo", vec!["check", "-p", "desktop_tauri"])
}

fn prepend_tauri_subcommand(root_cmd: &str, subcommand: &str, args: Vec<String>) -> Vec<String> {
    let mut all = vec![root_cmd.to_string(), subcommand.to_string()];
    all.extend(args);
    all
}

pub(super) fn run_check_web(ctx: &CommandContext) -> XtaskResult<()> {
    ctx.process().run(
        ctx.root(),
        "cargo",
        vec!["check", "-p", "site", "--features", SITE_CARGO_FEATURE],
    )?;

    if wasm_target_installed() {
        ctx.process().run(
            ctx.root(),
            "cargo",
            vec![
                "check",
                "-p",
                "site",
                "--target",
                "wasm32-unknown-unknown",
                "--features",
                SITE_CARGO_FEATURE,
            ],
        )
    } else {
        ctx.workflow()
            .warn("wasm32-unknown-unknown target not installed; skipping wasm cargo check");
        Ok(())
    }
}

pub(super) fn run_dev_command(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    match args.first().map(String::as_str) {
        None => dev_server_foreground(ctx, Vec::new()),
        Some("serve") => dev_server_foreground(ctx, args[1..].to_vec()),
        Some("start") => dev_server_start(ctx, args[1..].to_vec()),
        Some("stop") => dev_server_stop(ctx, false),
        Some("status") => dev_server_status(ctx),
        Some("logs") => dev_server_logs(ctx, args[1..].to_vec()),
        Some("restart") => {
            dev_server_stop(ctx, true)?;
            dev_server_start(ctx, args[1..].to_vec())
        }
        Some("build") => trunk_build(ctx, args[1..].to_vec(), BuildProfile::Dev),
        Some("help" | "--help" | "-h") => {
            print_dev_usage();
            Ok(())
        }
        _ => dev_server_foreground(ctx, args),
    }
}

fn args_specify_dist(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--dist" || arg.starts_with("--dist="))
}

fn args_specify_no_sri(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--no-sri" || arg.starts_with("--no-sri="))
}
