use super::args::{
    build_criterion_named_args, flamegraph_args_include_output, parse_named_bench_args,
};
use super::{ensure_perf_dirs, PERF_FLAMEGRAPH_DIR};
use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};

pub(super) fn perf_check(ctx: &CommandContext) -> XtaskResult<()> {
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

pub(super) fn perf_bench(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    ensure_perf_dirs(ctx)?;
    let mut bench_args = vec!["bench".to_string()];
    if args.is_empty() {
        bench_args.push("--workspace".to_string());
    } else {
        bench_args.extend(args);
    }
    ctx.process().run_owned(ctx.root(), "cargo", bench_args)
}

pub(super) fn perf_baseline(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    let (baseline, cargo_args) = parse_named_bench_args("baseline", args)?;
    ensure_perf_dirs(ctx)?;
    ctx.process().run_owned(
        ctx.root(),
        "cargo",
        build_criterion_named_args(&cargo_args, "save-baseline", &baseline),
    )
}

pub(super) fn perf_compare(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    let (baseline, cargo_args) = parse_named_bench_args("compare", args)?;
    ensure_perf_dirs(ctx)?;
    ctx.process().run_owned(
        ctx.root(),
        "cargo",
        build_criterion_named_args(&cargo_args, "baseline", &baseline),
    )
}

pub(super) fn perf_flamegraph(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
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

pub(super) fn perf_heaptrack(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
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

#[cfg(test)]
mod tests {
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
}
