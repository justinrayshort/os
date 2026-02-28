//! Wiki submodule management workflows.

use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};
use crate::XtaskCommand;

const WIKI_DIR: &str = "wiki";

/// `cargo wiki ...`
pub struct WikiCommand;

/// Supported `cargo wiki` subcommands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WikiOptions {
    Status,
    Sync,
    Help,
}

impl XtaskCommand for WikiCommand {
    type Options = WikiOptions;

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        parse_wiki_options(args)
    }

    fn run(ctx: &CommandContext, options: Self::Options) -> XtaskResult<()> {
        match options {
            WikiOptions::Status => wiki_status(ctx),
            WikiOptions::Sync => wiki_sync(ctx),
            WikiOptions::Help => {
                print_wiki_usage();
                Ok(())
            }
        }
    }
}

fn parse_wiki_options(args: &[String]) -> XtaskResult<WikiOptions> {
    match args.first().map(String::as_str) {
        None | Some("status") => Ok(WikiOptions::Status),
        Some("sync") => Ok(WikiOptions::Sync),
        Some("help" | "--help" | "-h") => Ok(WikiOptions::Help),
        Some(other) => Err(XtaskError::validation(format!(
            "unknown wiki subcommand: {other}"
        ))),
    }
}

fn print_wiki_usage() {
    eprintln!(
        "Usage: cargo xtask wiki <subcommand>\n\
         \n\
         Subcommands:\n\
           status                Show wiki submodule initialization, branch, HEAD, and dirty state\n\
           sync                  Refresh submodule wiring and initialize/update `wiki/`\n\
                                 Refuses to run when `wiki/` has local modifications\n"
    );
}

fn wiki_status(ctx: &CommandContext) -> XtaskResult<()> {
    let wiki_dir = ctx.root().join(WIKI_DIR);
    let initialized = wiki_dir.join(".git").exists();

    println!("wiki path: {}", wiki_dir.display());
    println!("initialized: {}", if initialized { "yes" } else { "no" });

    if !initialized {
        println!("branch: unavailable");
        println!("head: unavailable");
        println!("dirty: unavailable");
        println!("hint: run `cargo wiki sync` to initialize or refresh the submodule");
        return Ok(());
    }

    let branch = ctx
        .workspace()
        .git_current_branch_at(&wiki_dir)
        .unwrap_or_else(|| "detached".into());
    let head = ctx.workspace().git_head_sha_at(&wiki_dir);
    let changed = ctx.workspace().git_changed_paths_at(&wiki_dir)?;

    println!("branch: {branch}");
    println!("head: {head}");
    println!("dirty: {}", if changed.is_empty() { "no" } else { "yes" });
    if !changed.is_empty() {
        println!("changed paths:");
        for path in changed {
            println!("  - {path}");
        }
    }

    Ok(())
}

fn wiki_sync(ctx: &CommandContext) -> XtaskResult<()> {
    let wiki_dir = ctx.root().join(WIKI_DIR);
    if wiki_dir.join(".git").exists() {
        let changed = ctx.workspace().git_changed_paths_at(&wiki_dir)?;
        if !changed.is_empty() {
            return Err(XtaskError::validation(
                "wiki submodule has local modifications; refusing to sync",
            )
            .with_operation("cargo wiki sync")
            .with_path(&wiki_dir)
            .with_hint(
                "review `cargo wiki status`, then commit, stash, or discard wiki/ changes before syncing",
            ));
        }
    }

    ctx.workflow()
        .with_workflow_run("wiki", Some("sync".into()), || {
            ctx.workflow()
                .run_timed_stage("Refresh git submodule wiring", || {
                    ctx.process()
                        .run(ctx.root(), "git", vec!["submodule", "sync", "--recursive"])
                })?;
            ctx.workflow()
                .run_timed_stage("Initialize or update wiki submodule", || {
                    ctx.process().run(
                        ctx.root(),
                        "git",
                        vec!["submodule", "update", "--init", "--recursive"],
                    )
                })?;
            ctx.workflow()
                .run_timed_stage("Report wiki submodule status", || wiki_status(ctx))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_defaults_to_status() {
        assert_eq!(parse_wiki_options(&[]).expect("parse"), WikiOptions::Status);
    }

    #[test]
    fn parse_sync_subcommand() {
        assert_eq!(
            parse_wiki_options(&["sync".into()]).expect("parse"),
            WikiOptions::Sync
        );
    }

    #[test]
    fn parse_rejects_unknown_subcommand() {
        let err = parse_wiki_options(&["bogus".into()]).expect_err("invalid");
        assert!(err.to_string().contains("unknown wiki subcommand"));
    }
}
