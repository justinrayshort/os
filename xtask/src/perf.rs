//! Performance engineering workflows (`cargo xtask perf ...`).
//!
//! This module provides repeatable local entrypoints for benchmark execution and profiling so the
//! workspace can standardize artifact locations and command usage without forcing heavyweight CI
//! integration by default.

use serde_json::json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const PERF_DIR: &str = ".artifacts/perf";
const PERF_FLAMEGRAPH_DIR: &str = ".artifacts/perf/flamegraphs";
const PERF_REPORT_DIR: &str = ".artifacts/perf/reports";
const DEV_LOOP_BASELINE_DEFAULT_OUTPUT: &str = ".artifacts/perf/reports/dev-loop-baseline.json";

/// Executes a performance workflow subcommand.
pub(crate) fn run_perf_command(root: &Path, args: Vec<String>) -> Result<(), String> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        print_perf_usage();
        return Ok(());
    };

    match subcommand {
        "doctor" => perf_doctor(),
        "check" => perf_check(root),
        "bench" => perf_bench(root, args[1..].to_vec()),
        "baseline" => perf_baseline(root, args[1..].to_vec()),
        "compare" => perf_compare(root, args[1..].to_vec()),
        "dev-loop-baseline" => perf_dev_loop_baseline(root, args[1..].to_vec()),
        "flamegraph" => perf_flamegraph(root, args[1..].to_vec()),
        "heaptrack" => perf_heaptrack(root, args[1..].to_vec()),
        "help" | "--help" | "-h" => {
            print_perf_usage();
            Ok(())
        }
        other => Err(format!("unknown perf subcommand: {other}")),
    }
}

/// Prints help for the performance workflow commands.
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
           heaptrack [-- cmd...]  Run heaptrack around a command (default: cargo bench --workspace)\n\
         \n\
         Notes:\n\
           - Artifacts default to `.artifacts/perf/`.\n\
           - `baseline`/`compare` append Criterion flags after `--`; do not pass your own `--`.\n\
           - `flamegraph` and `heaptrack` are optional local tools and may be unavailable by host OS.\n"
    );
}

fn perf_doctor() -> Result<(), String> {
    let cargo_ok = command_available("cargo");
    let perf_ok = command_available("perf");
    let heaptrack_ok = command_available("heaptrack");
    let cargo_flamegraph_ok = cargo_subcommand_available("flamegraph");
    let sccache_ok = command_available("sccache");
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
        Err("required command `cargo` not found".to_string())
    }
}

fn print_tool_status(name: &str, available: bool, note: &str) {
    let status = if available { "ok" } else { "missing" };
    println!("- {name}: {status} ({note})");
}

fn perf_check(root: &Path) -> Result<(), String> {
    ensure_perf_dirs(root)?;

    log_stage("Functional test suite (unit + integration)");
    run(
        root,
        "cargo",
        vec!["test", "--workspace", "--lib", "--tests"],
    )?;

    log_stage("Feature-expanded test suite");
    run(
        root,
        "cargo",
        vec!["test", "--workspace", "--all-features", "--lib", "--tests"],
    )?;

    log_stage("Rustdoc doctests");
    run(root, "cargo", vec!["test", "--workspace", "--doc"])?;

    log_stage("Benchmark target compile validation");
    run(root, "cargo", vec!["bench", "--workspace", "--no-run"])?;

    println!("\n==> Performance preflight complete");
    Ok(())
}

fn perf_bench(root: &Path, args: Vec<String>) -> Result<(), String> {
    ensure_perf_dirs(root)?;
    log_stage("Benchmark execution");

    let mut bench_args = vec!["bench".to_string()];
    if args.is_empty() {
        bench_args.push("--workspace".to_string());
    } else {
        bench_args.extend(args);
    }

    run_owned(root, "cargo", bench_args)
}

fn perf_baseline(root: &Path, args: Vec<String>) -> Result<(), String> {
    let (baseline, cargo_args) = parse_named_bench_args("baseline", args)?;
    ensure_perf_dirs(root)?;
    log_stage(&format!("Criterion baseline capture ({baseline})"));
    run_owned(
        root,
        "cargo",
        build_criterion_named_args(&cargo_args, "save-baseline", &baseline),
    )
}

fn perf_compare(root: &Path, args: Vec<String>) -> Result<(), String> {
    let (baseline, cargo_args) = parse_named_bench_args("compare", args)?;
    ensure_perf_dirs(root)?;
    log_stage(&format!("Criterion baseline comparison ({baseline})"));
    run_owned(
        root,
        "cargo",
        build_criterion_named_args(&cargo_args, "baseline", &baseline),
    )
}

fn perf_dev_loop_baseline(root: &Path, args: Vec<String>) -> Result<(), String> {
    ensure_perf_dirs(root)?;
    let output = parse_dev_loop_baseline_output_arg(args)?;
    let output_path = resolve_output_path(root, output);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
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

    log_stage("Developer loop baseline capture");
    let total_started = Instant::now();
    let mut command_reports = Vec::with_capacity(commands.len());
    for spec in commands {
        let started_unix_secs = unix_timestamp_secs();
        let started = Instant::now();
        run_owned(root, spec.program, spec.args.clone())?;
        let elapsed = started.elapsed().as_secs_f64();
        command_reports.push(json!({
            "command": spec.command_string(),
            "started_unix_secs": started_unix_secs,
            "duration_secs": elapsed,
        }));
    }

    let report = json!({
        "generated_unix_secs": unix_timestamp_secs(),
        "workspace_root": root.display().to_string(),
        "git_commit": git_commit_sha(root),
        "toolchain": {
            "cargo": command_stdout_line("cargo", &["--version"]),
            "rustc": command_stdout_line("rustc", &["--version"]),
        },
        "commands": command_reports,
        "total_duration_secs": total_started.elapsed().as_secs_f64(),
    });

    let report_body = serde_json::to_string_pretty(&report)
        .map_err(|err| format!("failed to encode report: {err}"))?;
    fs::write(&output_path, report_body)
        .map_err(|err| format!("failed to write {}: {err}", output_path.display()))?;
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

fn parse_dev_loop_baseline_output_arg(args: Vec<String>) -> Result<PathBuf, String> {
    if args.is_empty() {
        return Ok(PathBuf::from(DEV_LOOP_BASELINE_DEFAULT_OUTPUT));
    }

    if args.len() == 2 && args[0] == "--output" {
        return Ok(PathBuf::from(args[1].clone()));
    }

    Err("usage: cargo xtask perf dev-loop-baseline [--output <path>]".to_string())
}

fn resolve_output_path(root: &Path, output: PathBuf) -> PathBuf {
    if output.is_absolute() {
        output
    } else {
        root.join(output)
    }
}

fn git_commit_sha(root: &Path) -> String {
    let Ok(output) = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "HEAD"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    else {
        return "unknown".to_string();
    };

    if !output.status.success() {
        return "unknown".to_string();
    }

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn command_stdout_line(program: &str, args: &[&str]) -> String {
    let Ok(output) = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    else {
        return "unknown".to_string();
    };

    if !output.status.success() {
        return "unknown".to_string();
    }

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn parse_named_bench_args(
    subcommand: &str,
    args: Vec<String>,
) -> Result<(String, Vec<String>), String> {
    let Some(name) = args.first().cloned() else {
        return Err(format!(
            "missing baseline name. Usage: cargo xtask perf {subcommand} <name> [cargo bench args]"
        ));
    };

    let cargo_args = args[1..].to_vec();
    if cargo_args.iter().any(|arg| arg == "--") {
        return Err(format!(
            "`cargo xtask perf {subcommand}` appends Criterion args automatically; do not pass `--`"
        ));
    }

    Ok((name, cargo_args))
}

fn build_criterion_named_args(
    cargo_args: &[String],
    criterion_flag: &str,
    name: &str,
) -> Vec<String> {
    let mut args = vec!["bench".to_string()];
    if cargo_args.is_empty() {
        args.push("--workspace".to_string());
    } else {
        args.extend(cargo_args.iter().cloned());
    }
    args.push("--".to_string());
    args.push(format!("--{criterion_flag}"));
    args.push(name.to_string());
    args
}

fn perf_flamegraph(root: &Path, args: Vec<String>) -> Result<(), String> {
    ensure_command(
        "cargo",
        "Rust toolchain is required to run `cargo flamegraph`.",
    )?;
    if !cargo_subcommand_available("flamegraph") {
        return Err(
            "cargo subcommand `flamegraph` is not available. Install with `cargo install flamegraph`."
                .to_string(),
        );
    }

    ensure_perf_dirs(root)?;
    let mut flamegraph_args = if args.is_empty() {
        return Err(
            "no target/workload provided. Example: `cargo xtask perf flamegraph --bench <name>`"
                .to_string(),
        );
    } else {
        args
    };

    if !has_output_flag(&flamegraph_args) {
        let output = root
            .join(PERF_FLAMEGRAPH_DIR)
            .join(format!("flamegraph-{}.svg", unix_timestamp_secs()))
            .display()
            .to_string();
        flamegraph_args.push("-o".to_string());
        flamegraph_args.push(output);
    }

    log_stage("CPU flamegraph capture");
    let mut cmd = vec!["flamegraph".to_string()];
    cmd.extend(flamegraph_args);
    run_owned(root, "cargo", cmd)
}

fn perf_heaptrack(root: &Path, args: Vec<String>) -> Result<(), String> {
    ensure_command(
        "heaptrack",
        "Install heaptrack (Linux) or use an equivalent heap profiler on your platform.",
    )?;
    ensure_perf_dirs(root)?;

    let command = parse_heaptrack_command(args);
    let Some((program, program_args)) = command.split_first() else {
        return Err("heaptrack command cannot be empty".to_string());
    };

    log_stage("Heap profiler capture");
    let mut cmd = vec![program.to_string()];
    cmd.extend(program_args.iter().cloned());
    run_owned(root, "heaptrack", cmd)
}

fn parse_heaptrack_command(args: Vec<String>) -> Vec<String> {
    if args.is_empty() {
        return vec![
            "cargo".to_string(),
            "bench".to_string(),
            "--workspace".to_string(),
        ];
    }

    if args.first().map(String::as_str) == Some("--") {
        return args[1..].to_vec();
    }

    args
}

fn has_output_flag(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "-o" || arg == "--output" || arg.starts_with("--output="))
}

fn is_sccache_wrapper(wrapper: &str) -> bool {
    let trimmed = wrapper.trim();
    if trimmed.is_empty() {
        return false;
    }

    let normalized = trimmed.replace('\\', "/");
    normalized == "sccache" || normalized.ends_with("/sccache")
}

fn ensure_perf_dirs(root: &Path) -> Result<(), String> {
    for rel in [PERF_DIR, PERF_FLAMEGRAPH_DIR, PERF_REPORT_DIR] {
        let path = root.join(rel);
        fs::create_dir_all(&path)
            .map_err(|err| format!("failed to create {}: {err}", path.display()))?;
    }
    Ok(())
}

fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn command_available(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn cargo_subcommand_available(subcommand: &str) -> bool {
    Command::new("cargo")
        .arg(subcommand)
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn ensure_command(program: &str, hint: &str) -> Result<(), String> {
    if command_available(program) {
        Ok(())
    } else {
        Err(format!("required command `{program}` not found. {hint}"))
    }
}

fn run(root: &Path, program: &str, args: Vec<&str>) -> Result<(), String> {
    let owned = args.into_iter().map(ToString::to_string).collect();
    run_owned(root, program, owned)
}

fn run_owned(root: &Path, program: &str, args: Vec<String>) -> Result<(), String> {
    print_command(program, &args);
    let status = Command::new(program)
        .current_dir(root)
        .args(&args)
        .status()
        .map_err(|err| format!("failed to start `{program}`: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("`{program}` exited with status {status}"))
    }
}

fn log_stage(message: &str) {
    println!("\n==> {message}");
}

fn print_command(program: &str, args: &[String]) {
    if args.is_empty() {
        println!("+ {program}");
    } else {
        println!("+ {program} {}", args.join(" "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn criterion_args_default_to_workspace() {
        let args = build_criterion_named_args(&[], "save-baseline", "main");
        assert_eq!(
            args,
            vec!["bench", "--workspace", "--", "--save-baseline", "main"]
        );
    }

    #[test]
    fn criterion_args_preserve_cargo_filters() {
        let args = build_criterion_named_args(
            &["-p".into(), "desktop_runtime".into(), "event_loop".into()],
            "baseline",
            "prior",
        );
        assert_eq!(
            args,
            vec![
                "bench",
                "-p",
                "desktop_runtime",
                "event_loop",
                "--",
                "--baseline",
                "prior"
            ]
        );
    }

    #[test]
    fn flamegraph_output_flag_detection_handles_short_and_long_forms() {
        assert!(has_output_flag(&["-o".into(), "x.svg".into()]));
        assert!(has_output_flag(&["--output".into(), "x.svg".into()]));
        assert!(has_output_flag(&["--output=x.svg".into()]));
        assert!(!has_output_flag(&["--bench".into(), "runtime".into()]));
    }

    #[test]
    fn heaptrack_defaults_to_workspace_bench() {
        assert_eq!(
            parse_heaptrack_command(Vec::new()),
            vec!["cargo", "bench", "--workspace"]
        );
    }

    #[test]
    fn heaptrack_supports_double_dash_command_sentinel() {
        assert_eq!(
            parse_heaptrack_command(vec!["--".into(), "cargo".into(), "test".into()]),
            vec!["cargo", "test"]
        );
    }

    #[test]
    fn named_bench_args_reject_user_double_dash() {
        let err = parse_named_bench_args("baseline", vec!["main".into(), "--".into()])
            .expect_err("expected error");
        assert!(err.contains("do not pass `--`"));
    }

    #[test]
    fn dev_loop_baseline_output_arg_defaults_to_standard_path() {
        let output = parse_dev_loop_baseline_output_arg(Vec::new()).expect("parse");
        assert_eq!(output, PathBuf::from(DEV_LOOP_BASELINE_DEFAULT_OUTPUT));
    }

    #[test]
    fn dev_loop_baseline_output_arg_accepts_explicit_path() {
        let output = parse_dev_loop_baseline_output_arg(vec!["--output".into(), "x.json".into()])
            .expect("parse");
        assert_eq!(output, PathBuf::from("x.json"));
    }

    #[test]
    fn dev_loop_baseline_output_arg_rejects_invalid_shape() {
        let err = parse_dev_loop_baseline_output_arg(vec!["--bad".into()]).expect_err("invalid");
        assert!(err.contains("usage: cargo xtask perf dev-loop-baseline"));
    }

    #[test]
    fn sccache_wrapper_detection_handles_binary_and_path() {
        assert!(is_sccache_wrapper("sccache"));
        assert!(is_sccache_wrapper("/usr/local/bin/sccache"));
        assert!(is_sccache_wrapper("C:\\tools\\sccache"));
        assert!(!is_sccache_wrapper("/usr/bin/rustc"));
    }
}
