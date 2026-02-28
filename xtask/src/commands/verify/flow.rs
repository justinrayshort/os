use super::changed::{
    collect_changed_paths, detect_changed_packages, format_package_list, load_workspace_packages,
    looks_like_docs_change, looks_like_workspace_wide_change,
};
use crate::commands::docs;
use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};

#[derive(Clone, Debug)]
pub struct FlowOptions {
    pub(crate) scope_all: bool,
    pub(crate) packages: Vec<String>,
    pub(crate) include_docs: bool,
    pub(crate) show_help: bool,
}

pub(super) fn flow_command_inner(ctx: &CommandContext, options: FlowOptions) -> XtaskResult<()> {
    ctx.workflow().with_workflow_run("flow", None, || {
        let changed_paths = collect_changed_paths(ctx)?;
        let workspace_packages = load_workspace_packages(ctx)?;
        let mut changed_packages = detect_changed_packages(&changed_paths, &workspace_packages);
        let docs_changed = changed_paths
            .iter()
            .any(|path| looks_like_docs_change(path));
        let workspace_wide = changed_paths
            .iter()
            .any(|path| looks_like_workspace_wide_change(path));

        if options.scope_all || workspace_wide {
            changed_packages = workspace_packages
                .iter()
                .map(|pkg| pkg.name.clone())
                .collect::<Vec<_>>();
        }
        if !options.packages.is_empty() {
            changed_packages = options.packages.clone();
        }
        changed_packages.sort();
        changed_packages.dedup();

        let run_docs = options.include_docs || docs_changed || workspace_wide;

        if changed_packages.is_empty() && !run_docs {
            println!("No changed packages/docs detected; nothing to run.");
            return Ok(());
        }

        if !changed_packages.is_empty() {
            ctx.workflow()
                .run_timed_stage("Changed package cargo check", || {
                    ctx.process().run_owned(
                        ctx.root(),
                        "cargo",
                        cargo_check_package_args(&changed_packages),
                    )
                })?;
            ctx.workflow()
                .run_timed_stage("Changed package cargo test", || {
                    ctx.process().run_owned(
                        ctx.root(),
                        "cargo",
                        cargo_test_package_args(&changed_packages),
                    )
                })?;
            println!(
                "Packages checked: {}",
                format_package_list(&changed_packages)
            );
        }

        if run_docs {
            ctx.workflow()
                .run_timed_stage("Changed docs validation", || docs::run_all(ctx))?;
            println!("Docs validation included");
        }

        println!("\n==> Flow complete");
        Ok(())
    })
}

pub(super) fn parse_flow_options(args: Vec<String>) -> XtaskResult<FlowOptions> {
    let mut options = FlowOptions {
        scope_all: false,
        packages: Vec::new(),
        include_docs: false,
        show_help: false,
    };
    let mut i = 0usize;

    while i < args.len() {
        match args[i].as_str() {
            "--all" => {
                if !options.packages.is_empty() {
                    return Err(XtaskError::validation(
                        "`--all` cannot be combined with `--package`",
                    ));
                }
                options.scope_all = true;
                i += 1;
            }
            "--package" | "-p" => {
                if options.scope_all {
                    return Err(XtaskError::validation(
                        "`--package` cannot be combined with `--all`",
                    ));
                }
                let Some(package) = args.get(i + 1) else {
                    return Err(XtaskError::validation("missing value for `--package`"));
                };
                options.packages.push(package.clone());
                i += 2;
            }
            "--docs" => {
                options.include_docs = true;
                i += 1;
            }
            "help" | "--help" | "-h" => {
                options.show_help = true;
                i += 1;
            }
            other => {
                return Err(XtaskError::validation(format!(
                    "unsupported `cargo flow` argument `{other}`"
                )));
            }
        }
    }

    Ok(options)
}

pub(super) fn print_flow_usage() {
    eprintln!(
        "Usage: cargo flow [--all] [--package <name> ...] [--docs]\n\
         \n\
         Flags:\n\
           --all              Run checks for the full workspace\n\
           --package, -p      Restrict checks to one or more packages\n\
           --docs             Include docs validation regardless of detected changes\n"
    );
}

fn cargo_check_package_args(packages: &[String]) -> Vec<String> {
    let mut args = vec!["check".to_string()];
    for package in packages {
        args.push("-p".to_string());
        args.push(package.clone());
    }
    args
}

fn cargo_test_package_args(packages: &[String]) -> Vec<String> {
    let mut args = vec![
        "test".to_string(),
        "--lib".to_string(),
        "--tests".to_string(),
    ];
    for package in packages {
        args.push("-p".to_string());
        args.push(package.clone());
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_option_parser_rejects_package_with_all_scope() {
        let err = parse_flow_options(vec!["--all".into(), "--package".into(), "xtask".into()])
            .unwrap_err();
        assert!(err.to_string().contains("cannot be combined"));
    }
}
