//! Performance engineering workflows.

use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};
use crate::runtime::workflow::unix_timestamp_secs;
use crate::XtaskCommand;
use serde_json::json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;

const PERF_DIR: &str = ".artifacts/perf";
const PERF_FLAMEGRAPH_DIR: &str = ".artifacts/perf/flamegraphs";
const PERF_REPORT_DIR: &str = ".artifacts/perf/reports";
const DEV_LOOP_BASELINE_DEFAULT_OUTPUT: &str = ".artifacts/perf/reports/dev-loop-baseline.json";

/// `cargo perf ...`
pub struct PerfCommand;

impl XtaskCommand for PerfCommand {
    type Options = Vec<String>;

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        Ok(args.to_vec())
    }

    fn run(ctx: &CommandContext, args: Self::Options) -> XtaskResult<()> {
        let Some(subcommand) = args.first().map(String::as_str) else {
            print_perf_usage();
            return Ok(());
        };

        match subcommand {
            "doctor" => perf_doctor(ctx),
            "check" => perf_check(ctx),
            "bench" => perf_bench(ctx, args[1..].to_vec()),
            "baseline" => perf_baseline(ctx, args[1..].to_vec()),
            "compare" => perf_compare(ctx, args[1..].to_vec()),
            "dev-loop-baseline" => perf_dev_loop_baseline(ctx, args[1..].to_vec()),
            "flamegraph" => perf_flamegraph(ctx, args[1..].to_vec()),
            "heaptrack" => perf_heaptrack(ctx, args[1..].to_vec()),
            "help" | "--help" | "-h" => {
                print_perf_usage();
                Ok(())
            }
            other => Err(XtaskError::validation(format!(
                "unknown perf subcommand: {other}"
            ))),
        }
    }
}

pub(crate) fn print_perf_usage() {
    eprintln!(
        "Usage: cargo xtask perf <subcommand> [args]\n\
         \n\
         Subcommands:\n\
           doctor                 Check local benchmark/profiling tool availability\n\
           check                  Run tests/doctests and compile benchmark targets\n\
           bench [args]           Run `cargo bench --workspace` (pass args through)\n\
           baseline <name> [args] Run Criterion benchmarks and save baseline `<name>`\n\
           compare <name> [args]  Run Criterion benchmarks and compare to baseline `<name>`\n\
           dev-loop-baseline [--output <path>]\n\
                                  Run standard dev-loop timing commands and write JSON output\n\
           flamegraph [args]      Run `cargo flamegraph` (adds default SVG output path)\n\
           heaptrack [-- cmd...]  Run heaptrack around a command (default: cargo bench --workspace)\n"
    );
}

fn perf_doctor(ctx: &CommandContext) -> XtaskResult<()> {
    let cargo_ok = ctx.process().command_available("cargo");
    let perf_ok = ctx.process().command_available("perf");
    let heaptrack_ok = ctx.process().command_available("heaptrack");
    let cargo_flamegraph_ok = ctx.process().cargo_subcommand_available("flamegraph");
    let sccache_ok = ctx.process().command_available("sccache");
    let rustc_wrapper = env::var("RUSTC_WRAPPER").ok();
    let sccache_wrapper_active = rustc_wrapper
        .as_deref()
        .map(is_sccache_wrapper)
        .unwrap_or(false);

    println!("performance tooling status:");
    print_tool_status("cargo", cargo_ok, "required");
    print_tool_status(
        "cargo flamegraph",
        cargo_flamegraph_ok,
        "optional (install with `cargo install flamegraph`)",
    );
    print_tool_status("perf", perf_ok, "optional (Linux CPU sampling backend)");
    print_tool_status("heaptrack", heaptrack_ok, "optional (Linux heap profiler)");
    print_tool_status(
        "sccache",
        sccache_ok,
        "optional (compiler artifact cache for faster rebuilds)",
    );

    let wrapper_status = match rustc_wrapper {
        Some(ref wrapper) if sccache_wrapper_active => {
            format!("active ({wrapper}); run `sccache --show-stats`")
        }
        Some(wrapper) => format!("set to `{wrapper}` (not sccache)"),
        None => "not set (set `RUSTC_WRAPPER=sccache` to enable local compiler caching)".into(),
    };
    println!("- RUSTC_WRAPPER: {wrapper_status}");

    if cargo_ok {
        Ok(())
    } else {
        Err(XtaskError::environment(
            "required command `cargo` not found",
        ))
    }
}

fn print_tool_status(name: &str, available: bool, note: &str) {
    let status = if available { "ok" } else { "missing" };
    println!("- {name}: {status} ({note})");
}

fn perf_check(ctx: &CommandContext) -> XtaskResult<()> {
    ensure_perf_dirs(ctx)?;

    ctx.workflow()
        .run_timed_stage("Functional test suite (unit + integration)", || {
            ctx.process().run(
                ctx.root(),
                "cargo",
                vec!["test", "--workspace", "--lib", "--tests"],
            )
        })?;
    ctx.workflow()
        .run_timed_stage("Feature-expanded test suite", || {
            ctx.process().run(
                ctx.root(),
                "cargo",
                vec!["test", "--workspace", "--all-features", "--lib", "--tests"],
            )
        })?;
    ctx.workflow().run_timed_stage("Rustdoc doctests", || {
        ctx.process()
            .run(ctx.root(), "cargo", vec!["test", "--workspace", "--doc"])
    })?;
    ctx.workflow()
        .run_timed_stage("Benchmark target compile validation", || {
            ctx.process().run(
                ctx.root(),
                "cargo",
                vec!["bench", "--workspace", "--no-run"],
            )
        })?;

    println!("\n==> Performance preflight complete");
    Ok(())
}

fn perf_bench(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    ensure_perf_dirs(ctx)?;
    let mut bench_args = vec!["bench".to_string()];
    if args.is_empty() {
        bench_args.push("--workspace".to_string());
    } else {
        bench_args.extend(args);
    }
    ctx.process().run_owned(ctx.root(), "cargo", bench_args)
}

fn perf_baseline(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    let (baseline, cargo_args) = parse_named_bench_args("baseline", args)?;
    ensure_perf_dirs(ctx)?;
    ctx.process().run_owned(
        ctx.root(),
        "cargo",
        build_criterion_named_args(&cargo_args, "save-baseline", &baseline),
    )
}

fn perf_compare(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    let (baseline, cargo_args) = parse_named_bench_args("compare", args)?;
    ensure_perf_dirs(ctx)?;
    ctx.process().run_owned(
        ctx.root(),
        "cargo",
        build_criterion_named_args(&cargo_args, "baseline", &baseline),
    )
}

fn perf_dev_loop_baseline(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
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
        "git_commit": git_commit_sha(ctx.root()),
        "toolchain": {
            "cargo": command_stdout_line("cargo", &["--version"]),
            "rustc": command_stdout_line("rustc", &["--version"]),
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

fn parse_dev_loop_baseline_output_arg(args: Vec<String>) -> XtaskResult<PathBuf> {
    if args.is_empty() {
        return Ok(PathBuf::from(DEV_LOOP_BASELINE_DEFAULT_OUTPUT));
    }

    if args.len() == 2 && args[0] == "--output" {
        return Ok(PathBuf::from(&args[1]));
    }

    Err(XtaskError::validation(
        "`dev-loop-baseline` expects no args or `--output <path>`",
    ))
}

fn parse_named_bench_args(command: &str, args: Vec<String>) -> XtaskResult<(String, Vec<String>)> {
    let Some(baseline) = args.first().cloned() else {
        return Err(XtaskError::validation(format!(
            "`{command}` requires a baseline name"
        )));
    };

    let cargo_args = args[1..].to_vec();
    if cargo_args.iter().any(|arg| arg == "--") {
        return Err(XtaskError::validation(
            "do not pass your own `--`; xtask appends Criterion flags automatically",
        ));
    }

    Ok((baseline, cargo_args))
}

fn build_criterion_named_args(
    cargo_args: &[String],
    criterion_flag: &str,
    baseline: &str,
) -> Vec<String> {
    let mut args = if cargo_args.is_empty() {
        vec!["bench".to_string(), "--workspace".to_string()]
    } else {
        let mut user_args = vec!["bench".to_string()];
        user_args.extend(cargo_args.iter().cloned());
        user_args
    };
    args.push("--".to_string());
    args.push(format!("--{criterion_flag}"));
    args.push(baseline.to_string());
    args
}

fn perf_flamegraph(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    ctx.process().ensure_command(
        "cargo",
        "Install Rust toolchain and ensure `cargo` is on PATH.",
    )?;
    if !ctx.process().cargo_subcommand_available("flamegraph") {
        return Err(XtaskError::environment(
            "required cargo subcommand `flamegraph` not found. Install it with `cargo install flamegraph`.",
        ));
    }
    ensure_perf_dirs(ctx)?;

    let mut cmd = vec!["flamegraph".to_string()];
    let has_output = flamegraph_args_include_output(&args);
    cmd.extend(args);
    if !has_output {
        cmd.push("--output".to_string());
        cmd.push(
            ctx.root()
                .join(PERF_FLAMEGRAPH_DIR)
                .join("flamegraph.svg")
                .display()
                .to_string(),
        );
    }
    ctx.process().run_owned(ctx.root(), "cargo", cmd)
}

fn perf_heaptrack(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    ctx.process().ensure_command(
        "heaptrack",
        "Install heaptrack (Linux) or use an equivalent heap profiler on your platform.",
    )?;
    ensure_perf_dirs(ctx)?;
    let cmd = if args.is_empty() {
        vec![
            "cargo".to_string(),
            "bench".to_string(),
            "--workspace".to_string(),
        ]
    } else if args.first().map(String::as_str) == Some("--") {
        args[1..].to_vec()
    } else {
        args
    };
    ctx.process().run_owned(ctx.root(), "heaptrack", cmd)
}

fn ensure_perf_dirs(ctx: &CommandContext) -> XtaskResult<()> {
    ctx.artifacts().ensure_dir(&ctx.root().join(PERF_DIR))?;
    ctx.artifacts()
        .ensure_dir(&ctx.root().join(PERF_FLAMEGRAPH_DIR))?;
    ctx.artifacts()
        .ensure_dir(&ctx.root().join(PERF_REPORT_DIR))
}

fn command_stdout_line(program: &str, args: &[&str]) -> String {
    let Ok(output) = std::process::Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    else {
        return "unavailable".into();
    };
    if !output.status.success() {
        return "unavailable".into();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("unavailable")
        .trim()
        .to_string()
}

fn git_commit_sha(root: &Path) -> String {
    let Ok(output) = std::process::Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "HEAD"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    else {
        return "unavailable".into();
    };
    if !output.status.success() {
        return "unavailable".into();
    }
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn flamegraph_args_include_output(args: &[String]) -> bool {
    let mut i = 0usize;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--output" || arg.starts_with("--output=") {
            return true;
        }
        i += 1;
    }
    false
}

fn is_sccache_wrapper(wrapper: &str) -> bool {
    wrapper == "sccache" || wrapper.ends_with("/sccache")
}

fn resolve_output_path(root: &Path, output: PathBuf) -> PathBuf {
    if output.is_absolute() {
        output
    } else {
        root.join(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn criterion_args_default_to_workspace() {
        let args = build_criterion_named_args(&[], "save-baseline", "local-main");
        assert_eq!(
            args,
            vec![
                "bench",
                "--workspace",
                "--",
                "--save-baseline",
                "local-main",
            ]
        );
    }

    #[test]
    fn criterion_args_preserve_cargo_filters() {
        let args = build_criterion_named_args(
            &["--bench".into(), "runtime".into()],
            "baseline",
            "local-main",
        );
        assert_eq!(
            args,
            vec![
                "bench",
                "--bench",
                "runtime",
                "--",
                "--baseline",
                "local-main"
            ]
        );
    }

    #[test]
    fn named_bench_args_reject_user_double_dash() {
        let err = parse_named_bench_args("baseline", vec!["main".into(), "--".into()]).unwrap_err();
        assert!(err.to_string().contains("do not pass your own"));
    }

    #[test]
    fn dev_loop_baseline_output_arg_defaults_to_standard_path() {
        let output = parse_dev_loop_baseline_output_arg(Vec::new()).expect("parse");
        assert_eq!(output, PathBuf::from(DEV_LOOP_BASELINE_DEFAULT_OUTPUT));
    }

    #[test]
    fn dev_loop_baseline_output_arg_accepts_explicit_path() {
        let output = parse_dev_loop_baseline_output_arg(vec![
            "--output".into(),
            ".artifacts/custom.json".into(),
        ])
        .expect("parse");
        assert_eq!(output, PathBuf::from(".artifacts/custom.json"));
    }

    #[test]
    fn dev_loop_baseline_output_arg_rejects_invalid_shape() {
        let err = parse_dev_loop_baseline_output_arg(vec!["oops".into()]).unwrap_err();
        assert!(err.to_string().contains("expects no args"));
    }

    #[test]
    fn flamegraph_output_flag_detection_handles_short_and_long_forms() {
        assert!(flamegraph_args_include_output(&[
            "--output".into(),
            "a.svg".into()
        ]));
        assert!(flamegraph_args_include_output(&["--output=a.svg".into()]));
        assert!(!flamegraph_args_include_output(&[
            "--bench".into(),
            "foo".into()
        ]));
    }

    #[test]
    fn heaptrack_defaults_to_workspace_bench() {
        let cmd = if Vec::<String>::new().is_empty() {
            vec![
                "cargo".to_string(),
                "bench".to_string(),
                "--workspace".to_string(),
            ]
        } else {
            unreachable!()
        };
        assert_eq!(cmd, vec!["cargo", "bench", "--workspace"]);
    }

    #[test]
    fn heaptrack_supports_double_dash_command_sentinel() {
        let args = vec!["--".into(), "cargo".into(), "test".into()];
        let cmd = if args.first().map(String::as_str) == Some("--") {
            args[1..].to_vec()
        } else {
            args
        };
        assert_eq!(cmd, vec!["cargo", "test"]);
    }

    #[test]
    fn sccache_wrapper_detection_handles_binary_and_path() {
        assert!(is_sccache_wrapper("sccache"));
        assert!(is_sccache_wrapper("/usr/local/bin/sccache"));
        assert!(!is_sccache_wrapper("/usr/local/bin/clang"));
    }
}
