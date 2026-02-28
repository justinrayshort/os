use super::args::parse_dev_loop_baseline_output_arg;
use super::{ensure_perf_dirs, resolve_output_path};
use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};
use crate::runtime::workflow::unix_timestamp_secs;
use serde_json::json;
use std::fs;
use std::time::Instant;

pub(super) fn perf_dev_loop_baseline(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    ensure_perf_dirs(ctx)?;
    let output = parse_dev_loop_baseline_output_arg(args)?;
    let output_path = resolve_output_path(ctx.root(), output);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            XtaskError::io(format!("failed to create {}: {err}", parent.display()))
        })?;
    }

    let commands = vec![
        BaselineCommandSpec::new("cargo", vec!["clean"]),
        BaselineCommandSpec::new("cargo", vec!["check", "--workspace"]),
        BaselineCommandSpec::new(
            "cargo",
            vec!["test", "--workspace", "--lib", "--tests", "--no-run"],
        ),
        BaselineCommandSpec::new("cargo", vec!["verify-fast"]),
        BaselineCommandSpec::new("cargo", vec!["xtask", "docs", "all"]),
    ];

    let total_started = Instant::now();
    let mut command_reports = Vec::with_capacity(commands.len());
    for spec in commands {
        let started_unix_secs = unix_timestamp_secs();
        let started = Instant::now();
        ctx.process()
            .run_owned(ctx.root(), spec.program, spec.args.clone())?;
        command_reports.push(json!({
            "command": spec.command_string(),
            "started_unix_secs": started_unix_secs,
            "duration_secs": started.elapsed().as_secs_f64(),
        }));
    }

    let report = json!({
        "generated_unix_secs": unix_timestamp_secs(),
        "workspace_root": ctx.root().display().to_string(),
        "git_commit": ctx.workspace().git_head_sha(),
        "toolchain": {
            "cargo": ctx.process().capture_stdout_line("cargo", &["--version"]),
            "rustc": ctx.process().capture_stdout_line("rustc", &["--version"]),
        },
        "commands": command_reports,
        "total_duration_secs": total_started.elapsed().as_secs_f64(),
    });

    let report_body = serde_json::to_string_pretty(&report)
        .map_err(|err| XtaskError::io(format!("failed to encode report: {err}")))?;
    fs::write(&output_path, report_body).map_err(|err| {
        XtaskError::io(format!("failed to write {}: {err}", output_path.display()))
    })?;
    println!("Wrote dev-loop baseline report: {}", output_path.display());
    Ok(())
}

#[derive(Clone, Debug)]
struct BaselineCommandSpec {
    program: &'static str,
    args: Vec<String>,
}

impl BaselineCommandSpec {
    fn new(program: &'static str, args: Vec<&'static str>) -> Self {
        Self {
            program,
            args: args.into_iter().map(ToString::to_string).collect(),
        }
    }

    fn command_string(&self) -> String {
        if self.args.is_empty() {
            self.program.to_string()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }
}
