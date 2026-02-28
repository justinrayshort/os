use super::args::is_sccache_wrapper;
use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};
use std::env;

pub(super) fn perf_doctor(ctx: &CommandContext) -> XtaskResult<()> {
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
