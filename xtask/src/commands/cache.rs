//! Compiler-cache bootstrap and validation workflows.

use crate::runtime::cache::SccacheStatus;
use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};
use crate::XtaskCommand;

/// `cargo cache ...`
pub struct CacheCommand;

/// Supported `cargo cache` subcommands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CacheOptions {
    Bootstrap,
    Doctor,
    Stats,
    Help,
}

impl XtaskCommand for CacheCommand {
    type Options = CacheOptions;

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        parse_cache_options(args)
    }

    fn run(ctx: &CommandContext, options: Self::Options) -> XtaskResult<()> {
        match options {
            CacheOptions::Bootstrap => cache_bootstrap(ctx),
            CacheOptions::Doctor => cache_doctor(ctx),
            CacheOptions::Stats => cache_stats(ctx),
            CacheOptions::Help => {
                print_cache_usage();
                Ok(())
            }
        }
    }
}

fn parse_cache_options(args: &[String]) -> XtaskResult<CacheOptions> {
    match args.first().map(String::as_str) {
        None | Some("doctor") => Ok(CacheOptions::Doctor),
        Some("bootstrap") => Ok(CacheOptions::Bootstrap),
        Some("stats") => Ok(CacheOptions::Stats),
        Some("help" | "--help" | "-h") => Ok(CacheOptions::Help),
        Some(other) => Err(XtaskError::validation(format!(
            "unknown cache subcommand: {other}"
        ))),
    }
}

pub(crate) fn print_cache_usage() {
    eprintln!(
        "Usage: cargo xtask cache <subcommand>\n\
         \n\
         Subcommands:\n\
           bootstrap            Create/validate the repo-local sccache contract and start the server\n\
           doctor               Validate the repo-local sccache contract without creating missing dirs\n\
           stats                Show canonical sccache statistics for this workspace\n"
    );
}

pub(crate) fn print_sccache_status(status: &SccacheStatus) {
    println!("sccache binary: {}", status.binary_path);
    println!("sccache version: {}", status.version);
    println!("rustc wrapper: {}", status.config.wrapper);
    println!("cache backend: {}", status.config.backend);
    println!("cache dir: {}", status.config.dir.display());
    println!("cache dir existed before run: {}", status.cache_dir_preexisting);
    println!("cache size limit: {}", status.config.cache_size);
    println!("cache location: {}", status.stats.cache_location);
    println!(
        "cache hits: {} | misses: {} | writes: {} | compile requests: {}",
        status.stats.stats.total_hits(),
        status.stats.stats.total_misses(),
        status.stats.stats.cache_writes,
        status.stats.stats.compile_requests
    );
}

fn cache_bootstrap(ctx: &CommandContext) -> XtaskResult<()> {
    ctx.workflow()
        .with_workflow_run("cache", Some("bootstrap".into()), || {
            ctx.workflow()
                .run_timed_stage("Compiler cache bootstrap: sccache", || {
                    let status = ctx.cache().bootstrap()?;
                    print_sccache_status(&status);
                    Ok(())
                })
        })
}

fn cache_doctor(ctx: &CommandContext) -> XtaskResult<()> {
    ctx.workflow()
        .with_workflow_run("cache", Some("doctor".into()), || {
            ctx.workflow()
                .run_timed_stage("Compiler cache prerequisite: sccache", || {
                    let status = ctx.cache().validate(false)?;
                    print_sccache_status(&status);
                    Ok(())
                })
        })
}

fn cache_stats(ctx: &CommandContext) -> XtaskResult<()> {
    ctx.workflow()
        .with_workflow_run("cache", Some("stats".into()), || {
            ctx.workflow()
                .run_timed_stage("Compiler cache statistics: sccache", || {
                    let status = ctx.cache().validate(false)?;
                    let stats = ctx.cache().stats()?;
                    print_sccache_status(&status);
                    println!(
                        "raw stats summary: compile_requests={} hits={} misses={} writes={} non_cacheable={}",
                        stats.stats.compile_requests,
                        stats.stats.total_hits(),
                        stats.stats.total_misses(),
                        stats.stats.cache_writes,
                        stats.stats.requests_not_cacheable
                    );
                    Ok(())
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_options_default_to_doctor() {
        assert_eq!(parse_cache_options(&[]).expect("parse"), CacheOptions::Doctor);
    }

    #[test]
    fn cache_options_accept_bootstrap_and_stats() {
        assert_eq!(
            parse_cache_options(&["bootstrap".into()]).expect("parse"),
            CacheOptions::Bootstrap
        );
        assert_eq!(
            parse_cache_options(&["stats".into()]).expect("parse"),
            CacheOptions::Stats
        );
    }
}
