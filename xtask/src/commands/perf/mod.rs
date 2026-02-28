//! Performance engineering workflows.

mod args;
mod doctor;
mod report;
mod run;

use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};
use crate::XtaskCommand;
use doctor::perf_doctor;
use report::perf_dev_loop_baseline;
use run::{perf_baseline, perf_bench, perf_check, perf_compare, perf_flamegraph, perf_heaptrack};

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

fn ensure_perf_dirs(ctx: &CommandContext) -> XtaskResult<()> {
    ctx.artifacts().ensure_dir(&ctx.root().join(PERF_DIR))?;
    ctx.artifacts()
        .ensure_dir(&ctx.root().join(PERF_FLAMEGRAPH_DIR))?;
    ctx.artifacts()
        .ensure_dir(&ctx.root().join(PERF_REPORT_DIR))
}
