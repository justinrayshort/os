//! External GitHub Wiki checkout workflows.

use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};
use crate::wiki_config::resolve_wiki_checkout;
use crate::XtaskCommand;
use std::fs;

/// `cargo wiki ...`
pub struct WikiCommand;

/// Supported `cargo wiki` subcommands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WikiOptions {
    Status,
    Clone,
    Verify,
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
            WikiOptions::Clone => wiki_clone(ctx),
            WikiOptions::Verify => wiki_verify(ctx),
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
        Some("clone") => Ok(WikiOptions::Clone),
        Some("verify") => Ok(WikiOptions::Verify),
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
           status                Show external wiki checkout path, branch, HEAD, remote, and sync state\n\
           clone                 Clone the external wiki repo into the configured checkout path\n\
           verify                Validate the configured external wiki checkout state\n"
    );
}

fn wiki_status(ctx: &CommandContext) -> XtaskResult<()> {
    let checkout = resolve_wiki_checkout(ctx.root())?;
    let wiki_dir = checkout.path;

    println!("wiki path: {}", wiki_dir.display());
    println!("path source: {}", checkout.source);
    println!("expected remote: {}", checkout.config.remote_url);
    println!(
        "expected default branch: {}",
        checkout.config.default_branch
    );
    println!(
        "initialized: {}",
        if wiki_dir.exists() { "yes" } else { "no" }
    );

    if !wiki_dir.exists() {
        println!("branch: unavailable");
        println!("upstream: unavailable");
        println!("head: unavailable");
        println!("remote: unavailable");
        println!("dirty: unavailable");
        println!("ahead: unavailable");
        println!("behind: unavailable");
        println!("hint: run `cargo wiki clone` to create the external checkout");
        return Ok(());
    }

    if !wiki_dir.join(".git").exists() {
        println!("branch: unavailable");
        println!("upstream: unavailable");
        println!("head: unavailable");
        println!("remote: unavailable");
        println!("dirty: unavailable");
        println!("ahead: unavailable");
        println!("behind: unavailable");
        println!("hint: existing path is not a git checkout");
        return Ok(());
    }

    let branch = ctx
        .workspace()
        .git_current_branch_at(&wiki_dir)
        .unwrap_or_else(|| "detached".into());
    let upstream = ctx
        .workspace()
        .git_upstream_branch_at(&wiki_dir)
        .unwrap_or_else(|| "unavailable".into());
    let head = ctx.workspace().git_head_sha_at(&wiki_dir);
    let remote = ctx
        .workspace()
        .git_remote_url_at(&wiki_dir, "origin")
        .unwrap_or_else(|| "unavailable".into());
    let changed = ctx.workspace().git_changed_paths_at(&wiki_dir)?;
    let ahead_behind = ctx.workspace().git_ahead_behind_at(&wiki_dir);

    println!("branch: {branch}");
    println!("upstream: {upstream}");
    println!("head: {head}");
    println!("remote: {remote}");
    println!("dirty: {}", if changed.is_empty() { "no" } else { "yes" });
    match ahead_behind {
        Some((ahead, behind)) => {
            println!("ahead: {ahead}");
            println!("behind: {behind}");
        }
        None => {
            println!("ahead: unavailable");
            println!("behind: unavailable");
        }
    }
    if !changed.is_empty() {
        println!("changed paths:");
        for path in changed {
            println!("  - {path}");
        }
    }

    Ok(())
}

fn wiki_clone(ctx: &CommandContext) -> XtaskResult<()> {
    let checkout = resolve_wiki_checkout(ctx.root())?;
    let wiki_dir = checkout.path;

    if wiki_dir.exists() {
        if wiki_dir.join(".git").exists() {
            return Err(XtaskError::validation(format!(
                "wiki checkout already exists at {}",
                wiki_dir.display()
            ))
            .with_operation("cargo wiki clone")
            .with_path(&wiki_dir)
            .with_hint("use `cargo wiki status` or `cargo wiki verify` instead"));
        }
        return Err(XtaskError::validation(format!(
            "wiki checkout target already exists but is not a git repository: {}",
            wiki_dir.display()
        ))
        .with_operation("cargo wiki clone")
        .with_path(&wiki_dir)
        .with_hint("move or remove the existing path before cloning"));
    }

    let Some(parent) = wiki_dir.parent() else {
        return Err(XtaskError::environment(format!(
            "unable to determine parent directory for {}",
            wiki_dir.display()
        )));
    };
    fs::create_dir_all(parent).map_err(|err| {
        XtaskError::io(format!("failed to create {}: {err}", parent.display()))
            .with_operation("cargo wiki clone")
            .with_path(parent)
    })?;

    ctx.workflow()
        .with_workflow_run("wiki", Some("clone".into()), || {
            ctx.workflow()
                .run_timed_stage("Clone external wiki repository", || {
                    ctx.process().run_owned(
                        parent,
                        "git",
                        vec![
                            "clone".into(),
                            checkout.config.remote_url.clone(),
                            wiki_dir.display().to_string(),
                        ],
                    )
                })?;
            ctx.workflow()
                .run_timed_stage("Report wiki checkout status", || wiki_status(ctx))
        })
}

fn wiki_verify(ctx: &CommandContext) -> XtaskResult<()> {
    let checkout = resolve_wiki_checkout(ctx.root())?;
    let wiki_dir = checkout.path;

    if !wiki_dir.exists() {
        return Err(XtaskError::environment(format!(
            "external wiki checkout is missing at {}",
            wiki_dir.display()
        ))
        .with_operation("cargo wiki verify")
        .with_path(&wiki_dir)
        .with_hint("run `cargo wiki clone` or set `OS_WIKI_PATH` to an existing checkout"));
    }
    if !wiki_dir.join(".git").exists() {
        return Err(XtaskError::environment(format!(
            "external wiki checkout is not a git repository: {}",
            wiki_dir.display()
        ))
        .with_operation("cargo wiki verify")
        .with_path(&wiki_dir));
    }

    let remote = ctx
        .workspace()
        .git_remote_url_at(&wiki_dir, "origin")
        .ok_or_else(|| {
            XtaskError::validation("wiki checkout has no configured `origin` remote")
                .with_operation("cargo wiki verify")
                .with_path(&wiki_dir)
        })?;
    if remote != checkout.config.remote_url {
        return Err(XtaskError::validation(format!(
            "wiki checkout remote mismatch: expected `{}`, found `{remote}`",
            checkout.config.remote_url
        ))
        .with_operation("cargo wiki verify")
        .with_path(&wiki_dir));
    }

    let branch = ctx
        .workspace()
        .git_current_branch_at(&wiki_dir)
        .ok_or_else(|| {
            XtaskError::validation("wiki checkout is on a detached HEAD")
                .with_operation("cargo wiki verify")
                .with_path(&wiki_dir)
                .with_hint("switch to a local branch tracking the wiki default branch")
        })?;
    if branch != checkout.config.default_branch {
        return Err(XtaskError::validation(format!(
            "wiki checkout branch mismatch: expected `{}`, found `{branch}`",
            checkout.config.default_branch
        ))
        .with_operation("cargo wiki verify")
        .with_path(&wiki_dir));
    }

    let changed = ctx.workspace().git_changed_paths_at(&wiki_dir)?;
    if !changed.is_empty() {
        return Err(XtaskError::validation(
            "wiki checkout has local modifications; commit or discard them before verification",
        )
        .with_operation("cargo wiki verify")
        .with_path(&wiki_dir));
    }

    if let Some((ahead, behind)) = ctx.workspace().git_ahead_behind_at(&wiki_dir) {
        if ahead > 0 || behind > 0 {
            return Err(XtaskError::validation(format!(
                "wiki checkout is not synchronized with upstream (ahead {ahead}, behind {behind})"
            ))
            .with_operation("cargo wiki verify")
            .with_path(&wiki_dir)
            .with_hint("push or pull the external wiki checkout before proceeding"));
        }
    }

    println!("OK");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_defaults_to_status() {
        assert_eq!(parse_wiki_options(&[]).expect("parse"), WikiOptions::Status);
    }

    #[test]
    fn parse_clone_subcommand() {
        assert_eq!(
            parse_wiki_options(&["clone".into()]).expect("parse"),
            WikiOptions::Clone
        );
    }

    #[test]
    fn parse_verify_subcommand() {
        assert_eq!(
            parse_wiki_options(&["verify".into()]).expect("parse"),
            WikiOptions::Verify
        );
    }

    #[test]
    fn parse_rejects_unknown_subcommand() {
        let err = parse_wiki_options(&["bogus".into()]).expect_err("invalid");
        assert!(err.to_string().contains("unknown wiki subcommand"));
    }
}
