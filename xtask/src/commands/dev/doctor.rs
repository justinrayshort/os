use super::config::load_dev_server_config;
use super::server::{
    inspect_managed_pid, read_dev_server_state, remove_dev_server_state, wasm_target_installed,
    ManagedPidStatus,
};
use super::SetupWebCommand;
use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};
use crate::XtaskCommand;

#[derive(Clone, Debug)]
pub struct DoctorOptions {
    pub(crate) fix: bool,
    pub(crate) show_help: bool,
}

pub(super) fn parse_doctor_options(args: Vec<String>) -> XtaskResult<DoctorOptions> {
    let mut options = DoctorOptions {
        fix: false,
        show_help: false,
    };

    for arg in args {
        match arg.as_str() {
            "--fix" => options.fix = true,
            "help" | "--help" | "-h" => options.show_help = true,
            other => {
                return Err(XtaskError::validation(format!(
                    "unsupported `cargo doctor` argument `{other}`"
                )))
            }
        }
    }

    Ok(options)
}

pub(super) fn print_doctor_usage() {
    eprintln!(
        "Usage: cargo doctor [--fix]\n\
         \n\
         Checks local tooling and managed dev-server hygiene.\n\
         \n\
         Flags:\n\
           --fix  Remove stale managed dev server state when safe\n"
    );
}

pub(super) fn run_doctor(ctx: &CommandContext, options: DoctorOptions) -> XtaskResult<()> {
    let config = load_dev_server_config(ctx)?;
    ctx.workflow().with_workflow_run("doctor", None, || {
        ctx.workflow()
            .run_timed_stage("Tooling prerequisite: trunk", || {
                if ctx.process().command_available("trunk") {
                    println!("    trunk is available");
                    Ok(())
                } else if options.fix {
                    println!("    trunk missing; running `cargo setup-web`");
                    SetupWebCommand::run(ctx, ())?;
                    if ctx.process().command_available("trunk") {
                        println!("    trunk installed");
                        Ok(())
                    } else {
                        Err(XtaskError::environment(
                            "`trunk` is not available (run `cargo setup-web`)",
                        ))
                    }
                } else {
                    Err(XtaskError::environment(
                        "`trunk` is not available (run `cargo setup-web`)",
                    ))
                }
            })?;

        ctx.workflow()
            .run_timed_stage("Tooling prerequisite: wasm target", || {
                if wasm_target_installed() {
                    println!("    wasm32-unknown-unknown target installed");
                    Ok(())
                } else {
                    Err(XtaskError::environment(
                        "wasm32-unknown-unknown target missing (run `cargo setup-web`)",
                    ))
                }
            })?;

        ctx.workflow()
            .run_timed_stage("Docs prerequisite: wiki submodule", || {
                let wiki_root = ctx.root().join("wiki");
                if wiki_root.join(".git").exists() || wiki_root.join("Home.md").exists() {
                    println!("    wiki submodule initialized");
                    Ok(())
                } else {
                    Err(XtaskError::environment(
                        "wiki submodule missing (run `cargo wiki sync`)",
                    ))
                }
            })?;

        ctx.workflow()
            .run_timed_stage("Dev server state hygiene", || {
                let Some(state) = read_dev_server_state(ctx, &config)? else {
                    println!("    no managed dev server state file");
                    return Ok(());
                };

                match inspect_managed_pid(state.pid)? {
                    ManagedPidStatus::Managed => {
                        println!("    managed dev server state is healthy");
                        Ok(())
                    }
                    ManagedPidStatus::NotRunning | ManagedPidStatus::Unmanaged(_)
                        if options.fix =>
                    {
                        remove_dev_server_state(ctx, &config)?;
                        println!("    removed stale managed dev server state");
                        Ok(())
                    }
                    ManagedPidStatus::NotRunning | ManagedPidStatus::Unmanaged(_) => Err(
                        XtaskError::validation(
                            "managed dev server state is stale (run `cargo dev stop` or `cargo doctor --fix`)",
                        ),
                    ),
                }
            })?;

        println!("\n==> Doctor checks passed");
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_option_parser_accepts_fix() {
        let parsed = parse_doctor_options(vec!["--fix".into()]).expect("parse");
        assert!(parsed.fix);
        assert!(!parsed.show_help);
    }
}
