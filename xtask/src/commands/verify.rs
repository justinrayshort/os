//! Verification and changed-scope workflow commands.

use crate::commands::dev::{wasm_target_installed, BuildWebCommand, CheckWebCommand};
use crate::commands::docs;
use crate::runtime::config::ConfigLoader;
use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};
use crate::XtaskCommand;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Component, Path};
use std::process::{Command, Stdio};

const DESKTOP_TAURI_PACKAGE: &str = "desktop_tauri";
const VERIFY_PROFILES_FILE: &str = "tools/automation/verify_profiles.toml";

/// `cargo verify`
pub struct VerifyCommand;

impl XtaskCommand for VerifyCommand {
    type Options = VerifyOptions;

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        parse_verify_options(args.to_vec())
    }

    fn run(ctx: &CommandContext, mut options: Self::Options) -> XtaskResult<()> {
        if options.show_help {
            let profiles = load_verify_profiles(ctx).ok();
            print_verify_usage(profiles.as_ref());
            return Ok(());
        }

        let profiles = load_verify_profiles(ctx)?;
        options = resolve_verify_options_from_profile(options, &profiles)?;
        if options.mode == VerifyMode::Full && options.desktop_mode != VerifyFastDesktopMode::Auto {
            return Err(XtaskError::validation(
                "`--with-desktop` and `--without-desktop` are only valid with `cargo verify-fast`",
            ));
        }

        let run_profile = options.profile.clone();
        ctx.workflow()
            .with_workflow_run("verify", run_profile, || match options.mode {
                VerifyMode::Fast => verify_fast(ctx, options.desktop_mode),
                VerifyMode::Full => verify_full(ctx),
            })
    }
}

/// `cargo flow`
pub struct FlowCommand;

impl XtaskCommand for FlowCommand {
    type Options = FlowOptions;

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        parse_flow_options(args.to_vec())
    }

    fn run(ctx: &CommandContext, options: Self::Options) -> XtaskResult<()> {
        if options.show_help {
            print_flow_usage();
            return Ok(());
        }
        flow_command_inner(ctx, options)
    }
}

#[derive(Clone, Debug, Deserialize)]
struct VerifyProfilesFile {
    profile: BTreeMap<String, VerifyProfileSpec>,
}

#[derive(Clone, Debug, Deserialize)]
struct VerifyProfileSpec {
    mode: String,
    desktop_mode: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VerifyMode {
    Fast,
    Full,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VerifyFastDesktopMode {
    Auto,
    WithDesktop,
    WithoutDesktop,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifyOptions {
    mode: VerifyMode,
    desktop_mode: VerifyFastDesktopMode,
    explicit_mode: bool,
    explicit_desktop_mode: bool,
    profile: Option<String>,
    show_help: bool,
}

#[derive(Clone, Debug)]
struct VerifyFastDesktopDecision {
    include_desktop: bool,
    reason: String,
}

fn load_verify_profiles(ctx: &CommandContext) -> XtaskResult<BTreeMap<String, VerifyProfileSpec>> {
    let loader = ConfigLoader::<VerifyProfilesFile>::new(ctx.root(), VERIFY_PROFILES_FILE);
    let parsed = loader.load()?;
    if parsed.profile.is_empty() {
        return Err(XtaskError::config(format!(
            "{} does not define any profiles",
            loader.path().display()
        )));
    }
    Ok(parsed.profile)
}

fn resolve_verify_profile(
    profile_name: &str,
    profiles: &BTreeMap<String, VerifyProfileSpec>,
) -> XtaskResult<(VerifyMode, VerifyFastDesktopMode)> {
    let Some(profile) = profiles.get(profile_name) else {
        let known = profiles.keys().cloned().collect::<Vec<_>>().join(", ");
        return Err(XtaskError::config(format!(
            "unknown verify profile `{profile_name}` (known: {known})"
        )));
    };

    let mode = match profile.mode.as_str() {
        "fast" => VerifyMode::Fast,
        "full" => VerifyMode::Full,
        other => {
            return Err(XtaskError::config(format!(
                "verify profile `{profile_name}` has invalid mode `{other}` (expected `fast` or `full`)"
            )))
        }
    };

    let desktop_mode = match profile.desktop_mode.as_deref().unwrap_or("auto") {
        "auto" => VerifyFastDesktopMode::Auto,
        "with-desktop" => VerifyFastDesktopMode::WithDesktop,
        "without-desktop" => VerifyFastDesktopMode::WithoutDesktop,
        other => {
            return Err(XtaskError::config(format!(
                "verify profile `{profile_name}` has invalid desktop_mode `{other}` (expected `auto`, `with-desktop`, `without-desktop`)"
            )))
        }
    };

    Ok((mode, desktop_mode))
}

fn print_verify_profile_selection(
    name: &str,
    mode: VerifyMode,
    desktop_mode: VerifyFastDesktopMode,
) {
    let mode_text = match mode {
        VerifyMode::Fast => "fast",
        VerifyMode::Full => "full",
    };
    let desktop_text = match desktop_mode {
        VerifyFastDesktopMode::Auto => "auto",
        VerifyFastDesktopMode::WithDesktop => "with-desktop",
        VerifyFastDesktopMode::WithoutDesktop => "without-desktop",
    };
    println!(
        "\n==> Verify profile selected: `{name}` (mode={mode_text}, desktop_mode={desktop_text})"
    );
}

fn resolve_verify_options_from_profile(
    mut options: VerifyOptions,
    profiles: &BTreeMap<String, VerifyProfileSpec>,
) -> XtaskResult<VerifyOptions> {
    let Some(profile_name) = options.profile.clone() else {
        return Ok(options);
    };
    if options.explicit_mode {
        return Err(XtaskError::validation(
            "`--profile` cannot be combined with `fast`/`full` positional mode",
        ));
    }
    if options.explicit_desktop_mode {
        return Err(XtaskError::validation(
            "`--profile` cannot be combined with `--with-desktop`/`--without-desktop`",
        ));
    }
    let (mode, desktop_mode) = resolve_verify_profile(&profile_name, profiles)?;
    options.mode = mode;
    options.desktop_mode = desktop_mode;
    print_verify_profile_selection(&profile_name, mode, desktop_mode);
    Ok(options)
}

fn verify_profile_names(profiles: &BTreeMap<String, VerifyProfileSpec>) -> String {
    profiles.keys().cloned().collect::<Vec<_>>().join(", ")
}

fn print_verify_usage(profiles: Option<&BTreeMap<String, VerifyProfileSpec>>) {
    let profile_list = profiles
        .map(verify_profile_names)
        .unwrap_or_else(|| "<unavailable>".to_string());
    eprintln!(
        "Usage: cargo verify [fast|full] [--with-desktop|--without-desktop] [--profile <name>]\n\
         \n\
         Profiles:\n\
           {}\n\
         \n\
         Notes:\n\
           - `--profile` cannot be combined with explicit `fast`/`full` or desktop flags.\n\
           - desktop flags are only valid with `fast` mode.\n",
        profile_list
    );
}

fn parse_verify_options(args: Vec<String>) -> XtaskResult<VerifyOptions> {
    let mut options = VerifyOptions {
        mode: VerifyMode::Full,
        desktop_mode: VerifyFastDesktopMode::Auto,
        explicit_mode: false,
        explicit_desktop_mode: false,
        profile: None,
        show_help: false,
    };
    let mut i = 0usize;

    if let Some(first) = args.first().map(String::as_str) {
        match first {
            "fast" => {
                options.mode = VerifyMode::Fast;
                options.explicit_mode = true;
                i = 1;
            }
            "full" => {
                options.mode = VerifyMode::Full;
                options.explicit_mode = true;
                i = 1;
            }
            _ => {}
        }
    }

    while i < args.len() {
        match args[i].as_str() {
            "--with-desktop" => {
                if options.desktop_mode == VerifyFastDesktopMode::WithoutDesktop {
                    return Err(XtaskError::validation(
                        "`--with-desktop` cannot be combined with `--without-desktop`",
                    ));
                }
                options.desktop_mode = VerifyFastDesktopMode::WithDesktop;
                options.explicit_desktop_mode = true;
                i += 1;
            }
            "--without-desktop" => {
                if options.desktop_mode == VerifyFastDesktopMode::WithDesktop {
                    return Err(XtaskError::validation(
                        "`--with-desktop` cannot be combined with `--without-desktop`",
                    ));
                }
                options.desktop_mode = VerifyFastDesktopMode::WithoutDesktop;
                options.explicit_desktop_mode = true;
                i += 1;
            }
            "--profile" => {
                let Some(profile) = args.get(i + 1) else {
                    return Err(XtaskError::validation("missing value for `--profile`"));
                };
                options.profile = Some(profile.clone());
                i += 2;
            }
            "help" | "--help" | "-h" => {
                options.show_help = true;
                i += 1;
            }
            other => {
                return Err(XtaskError::validation(format!(
                    "unsupported `cargo verify` argument `{other}` (expected `fast`, `full`, `--with-desktop`, `--without-desktop`, `--profile`)"
                )));
            }
        }
    }

    Ok(options)
}

fn resolve_verify_fast_desktop_decision(
    ctx: &CommandContext,
    mode: VerifyFastDesktopMode,
) -> VerifyFastDesktopDecision {
    match mode {
        VerifyFastDesktopMode::WithDesktop => VerifyFastDesktopDecision {
            include_desktop: true,
            reason: "forced by `--with-desktop`".to_string(),
        },
        VerifyFastDesktopMode::WithoutDesktop => VerifyFastDesktopDecision {
            include_desktop: false,
            reason: "forced by `--without-desktop`".to_string(),
        },
        VerifyFastDesktopMode::Auto => match collect_changed_paths(ctx) {
            Ok(paths) => infer_verify_fast_desktop_decision(paths),
            Err(err) => {
                ctx.workflow().warn(&format!(
                    "failed to inspect changed files for desktop auto-detection; including desktop host checks ({err})"
                ));
                VerifyFastDesktopDecision {
                    include_desktop: true,
                    reason: "automatic detection failed; defaulting to include desktop host checks"
                        .to_string(),
                }
            }
        },
    }
}

fn infer_verify_fast_desktop_decision(changed_paths: Vec<String>) -> VerifyFastDesktopDecision {
    if changed_paths.is_empty() {
        return VerifyFastDesktopDecision {
            include_desktop: false,
            reason: "no changed files detected by `git status --porcelain`".to_string(),
        };
    }

    let desktop_triggers: Vec<String> = changed_paths
        .iter()
        .filter(|path| looks_like_desktop_host_change(path))
        .cloned()
        .collect();

    if !desktop_triggers.is_empty() {
        return VerifyFastDesktopDecision {
            include_desktop: true,
            reason: format!(
                "desktop-relevant changes detected ({})",
                format_path_list_for_reason(&desktop_triggers)
            ),
        };
    }

    VerifyFastDesktopDecision {
        include_desktop: false,
        reason: "no desktop host trigger paths changed".to_string(),
    }
}

fn format_path_list_for_reason(paths: &[String]) -> String {
    if paths.len() <= 3 {
        return paths.join(", ");
    }

    let shown = paths[..3].join(", ");
    format!("{shown} (+{} more)", paths.len() - 3)
}

fn looks_like_desktop_host_change(path: &str) -> bool {
    path.starts_with("crates/desktop_tauri/")
        || path.starts_with("crates/platform_host/")
        || path.starts_with("crates/platform_host_web/")
        || matches!(path, "Cargo.toml" | "Cargo.lock" | ".cargo/config.toml")
}

fn print_verify_fast_desktop_decision(decision: &VerifyFastDesktopDecision) {
    println!("\n==> Desktop host coverage (verify-fast)");
    if decision.include_desktop {
        println!("    included: {}", decision.reason);
    } else {
        println!("    skipped: {}", decision.reason);
        println!("    hint: pass `cargo verify-fast --with-desktop` to force desktop host checks");
    }
}

fn verify_fast(ctx: &CommandContext, desktop_mode: VerifyFastDesktopMode) -> XtaskResult<()> {
    let desktop_decision = resolve_verify_fast_desktop_decision(ctx, desktop_mode);
    print_verify_fast_desktop_decision(&desktop_decision);

    ctx.workflow()
        .run_timed_stage("Rust format and default test matrix", || {
            run_cargo_default_matrix_fast(ctx, desktop_decision.include_desktop)
        })?;
    ctx.workflow()
        .run_timed_stage("Rust all-features matrix", || {
            run_rust_feature_matrix_fast(ctx, desktop_decision.include_desktop)
        })?;
    ctx.workflow()
        .run_timed_stage("Rustdoc build and doctests", || {
            run_rustdoc_checks_fast(ctx, desktop_decision.include_desktop)
        })?;
    ctx.workflow()
        .run_timed_stage("Documentation validation and audit", || {
            run_docs_checks(ctx)
        })?;
    println!("\n==> Verification complete");
    Ok(())
}

fn verify_full(ctx: &CommandContext) -> XtaskResult<()> {
    ctx.workflow()
        .run_timed_stage("Rust format and default test matrix", || {
            run_cargo_default_matrix_full(ctx)
        })?;
    ctx.workflow()
        .run_timed_stage("Rust all-features matrix", || {
            run_rust_feature_matrix_full(ctx)
        })?;
    ctx.workflow()
        .run_timed_stage("Rustdoc build and doctests", || {
            run_rustdoc_checks_full(ctx)
        })?;
    ctx.workflow()
        .run_timed_stage("Documentation validation and audit", || {
            run_docs_checks(ctx)
        })?;
    ctx.workflow()
        .run_timed_stage("Prototype compile checks", || {
            run_prototype_compile_checks(ctx)
        })?;
    ctx.workflow()
        .run_timed_stage("Clippy lint checks", || run_optional_clippy(ctx))?;
    println!("\n==> Verification complete");
    Ok(())
}

fn run_cargo_default_matrix_fast(ctx: &CommandContext, include_desktop: bool) -> XtaskResult<()> {
    ctx.process()
        .run(ctx.root(), "cargo", vec!["fmt", "--all", "--", "--check"])?;
    let mut test_args = vec!["test", "--workspace", "--lib", "--tests"];
    if !include_desktop {
        test_args.extend(["--exclude", DESKTOP_TAURI_PACKAGE]);
    }
    ctx.process().run(ctx.root(), "cargo", test_args)
}

fn run_cargo_default_matrix_full(ctx: &CommandContext) -> XtaskResult<()> {
    ctx.process()
        .run(ctx.root(), "cargo", vec!["fmt", "--all", "--", "--check"])?;
    ctx.process()
        .run(ctx.root(), "cargo", vec!["check", "--workspace"])?;
    ctx.process().run(
        ctx.root(),
        "cargo",
        vec!["test", "--workspace", "--lib", "--tests"],
    )?;
    Ok(())
}

fn run_rust_feature_matrix_fast(ctx: &CommandContext, include_desktop: bool) -> XtaskResult<()> {
    let mut workspace_feature_test_args =
        vec!["test", "--workspace", "--all-features", "--lib", "--tests"];
    if !include_desktop {
        workspace_feature_test_args.extend(["--exclude", DESKTOP_TAURI_PACKAGE]);
    }

    ctx.process()
        .run(ctx.root(), "cargo", workspace_feature_test_args)
}

fn run_rust_feature_matrix_full(ctx: &CommandContext) -> XtaskResult<()> {
    ctx.process().run(
        ctx.root(),
        "cargo",
        vec!["check", "--workspace", "--all-features"],
    )?;
    ctx.process().run(
        ctx.root(),
        "cargo",
        vec!["test", "--workspace", "--all-features", "--lib", "--tests"],
    )?;

    Ok(())
}

fn run_rustdoc_checks_fast(ctx: &CommandContext, include_desktop: bool) -> XtaskResult<()> {
    let mut doc_args = vec!["doc".into(), "--workspace".into(), "--no-deps".into()];
    if !include_desktop {
        doc_args.extend(["--exclude".into(), DESKTOP_TAURI_PACKAGE.into()]);
    }
    ctx.process().run_owned_with_env(
        ctx.root(),
        "cargo",
        doc_args,
        &[("RUSTDOCFLAGS", "-Dwarnings")],
    )?;

    let mut doc_test_args = vec!["test", "--workspace", "--doc"];
    if !include_desktop {
        doc_test_args.extend(["--exclude", DESKTOP_TAURI_PACKAGE]);
    }
    ctx.process().run(ctx.root(), "cargo", doc_test_args)?;
    Ok(())
}

fn run_rustdoc_checks_full(ctx: &CommandContext) -> XtaskResult<()> {
    ctx.process().run_owned_with_env(
        ctx.root(),
        "cargo",
        vec!["doc".into(), "--workspace".into(), "--no-deps".into()],
        &[("RUSTDOCFLAGS", "-Dwarnings")],
    )?;
    ctx.process()
        .run(ctx.root(), "cargo", vec!["test", "--workspace", "--doc"])?;
    Ok(())
}

fn run_docs_checks(ctx: &CommandContext) -> XtaskResult<()> {
    docs::run_all(ctx)?;
    docs::write_report(ctx, ctx.artifacts().docs_audit_report())
}

fn run_optional_clippy(ctx: &CommandContext) -> XtaskResult<()> {
    let clippy_available = Command::new("cargo")
        .args(["clippy", "-V"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);

    if clippy_available {
        ctx.process().run(
            ctx.root(),
            "cargo",
            vec![
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
        )?;
    } else {
        ctx.workflow()
            .warn("cargo clippy not available; skipping clippy stage");
    }

    Ok(())
}

fn run_prototype_compile_checks(ctx: &CommandContext) -> XtaskResult<()> {
    CheckWebCommand::run(ctx, ())?;

    if wasm_target_installed() {
        BuildWebCommand::run(ctx, Vec::new())?;
    } else {
        ctx.workflow()
            .warn("wasm32-unknown-unknown target not installed; skipping trunk build");
    }

    Ok(())
}

#[derive(Clone, Debug)]
pub struct FlowOptions {
    scope_all: bool,
    packages: Vec<String>,
    include_docs: bool,
    show_help: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoMetadataPackage>,
    workspace_members: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct CargoMetadataPackage {
    id: String,
    name: String,
    manifest_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WorkspacePackage {
    name: String,
    manifest_dir: String,
}

fn flow_command_inner(ctx: &CommandContext, options: FlowOptions) -> XtaskResult<()> {
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

fn parse_flow_options(args: Vec<String>) -> XtaskResult<FlowOptions> {
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

fn print_flow_usage() {
    eprintln!(
        "Usage: cargo flow [--all] [--package <name> ...] [--docs]\n\
         \n\
         Flags:\n\
           --all              Run checks for the full workspace\n\
           --package, -p      Restrict checks to one or more packages\n\
           --docs             Include docs validation regardless of detected changes\n"
    );
}

fn collect_changed_paths(ctx: &CommandContext) -> XtaskResult<Vec<String>> {
    let output = Command::new("git")
        .current_dir(ctx.root())
        .args(["status", "--porcelain"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| {
            XtaskError::process_launch(format!("failed to start `git status --porcelain`: {err}"))
        })?;

    if !output.status.success() {
        return Err(XtaskError::process_exit(format!(
            "`git status --porcelain` exited with status {}",
            output.status
        )));
    }

    let mut paths = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(path) = parse_porcelain_status_path(line) {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn parse_porcelain_status_path(line: &str) -> Option<String> {
    if line.len() < 4 {
        return None;
    }

    let raw = line[3..].trim();
    if let Some((_, new)) = raw.split_once(" -> ") {
        Some(new.trim().to_string())
    } else if raw.is_empty() {
        None
    } else {
        Some(raw.to_string())
    }
}

fn load_workspace_packages(ctx: &CommandContext) -> XtaskResult<Vec<WorkspacePackage>> {
    let output = Command::new("cargo")
        .current_dir(ctx.root())
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| {
            XtaskError::process_launch(format!("failed to start `cargo metadata`: {err}"))
        })?;

    if !output.status.success() {
        return Err(XtaskError::process_exit(format!(
            "`cargo metadata` exited with status {}",
            output.status
        )));
    }

    let metadata: CargoMetadata = serde_json::from_slice(&output.stdout).map_err(|err| {
        XtaskError::validation(format!("failed to parse `cargo metadata` output: {err}"))
    })?;

    let mut packages = Vec::new();
    for member in &metadata.workspace_members {
        let Some(pkg) = metadata.packages.iter().find(|pkg| &pkg.id == member) else {
            continue;
        };
        let manifest_dir = Path::new(&pkg.manifest_path)
            .parent()
            .map(path_to_posix)
            .unwrap_or_default();
        packages.push(WorkspacePackage {
            name: pkg.name.clone(),
            manifest_dir,
        });
    }
    Ok(packages)
}

fn path_to_posix(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn detect_changed_packages(
    changed_paths: &[String],
    workspace_packages: &[WorkspacePackage],
) -> Vec<String> {
    workspace_packages
        .iter()
        .filter(|pkg| {
            changed_paths.iter().any(|path| {
                path == &pkg.manifest_dir || path.starts_with(&(pkg.manifest_dir.clone() + "/"))
            })
        })
        .map(|pkg| pkg.name.clone())
        .collect()
}

fn looks_like_docs_change(path: &str) -> bool {
    path.starts_with("docs/")
        || path.starts_with("wiki/")
        || path == "AGENTS.md"
        || path == "README.md"
}

fn looks_like_workspace_wide_change(path: &str) -> bool {
    matches!(
        path,
        "Cargo.toml" | "Cargo.lock" | ".cargo/config.toml" | VERIFY_PROFILES_FILE
    ) || path == "tools/automation/dev_server.toml"
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

fn format_package_list(packages: &[String]) -> String {
    if packages.len() <= 4 {
        return packages.join(", ");
    }
    let shown = packages[..4].join(", ");
    format!("{shown} (+{} more)", packages.len() - 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_profiles() -> BTreeMap<String, VerifyProfileSpec> {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "dev".to_string(),
            VerifyProfileSpec {
                mode: "fast".into(),
                desktop_mode: Some("auto".into()),
            },
        );
        profiles.insert(
            "release".to_string(),
            VerifyProfileSpec {
                mode: "full".into(),
                desktop_mode: Some("auto".into()),
            },
        );
        profiles
    }

    #[test]
    fn desktop_trigger_detection_matches_expected_paths() {
        assert!(looks_like_desktop_host_change(
            "crates/desktop_tauri/src/main.rs"
        ));
        assert!(!looks_like_desktop_host_change(
            "crates/apps/notepad/src/lib.rs"
        ));
    }

    #[test]
    fn changed_package_detection_matches_workspace_path_prefixes() {
        let packages = vec![
            WorkspacePackage {
                name: "site".into(),
                manifest_dir: "crates/site".into(),
            },
            WorkspacePackage {
                name: "xtask".into(),
                manifest_dir: "xtask".into(),
            },
        ];
        let detected = detect_changed_packages(
            &["crates/site/src/main.rs".into(), "README.md".into()],
            &packages,
        );
        assert_eq!(detected, vec!["site"]);
    }

    #[test]
    fn porcelain_parser_handles_rename_records() {
        assert_eq!(
            parse_porcelain_status_path("R  old/path -> new/path"),
            Some("new/path".into())
        );
    }

    #[test]
    fn verify_fast_auto_detection_includes_when_desktop_trigger_paths_changed() {
        let decision =
            infer_verify_fast_desktop_decision(vec!["crates/platform_host/src/lib.rs".into()]);
        assert!(decision.include_desktop);
    }

    #[test]
    fn verify_fast_auto_detection_skips_when_no_desktop_trigger_paths_changed() {
        let decision =
            infer_verify_fast_desktop_decision(vec!["crates/apps/notepad/src/lib.rs".into()]);
        assert!(!decision.include_desktop);
    }

    #[test]
    fn verify_option_parser_defaults_to_full_mode() {
        let parsed = parse_verify_options(Vec::new()).expect("parse");
        assert_eq!(parsed.mode, VerifyMode::Full);
        assert_eq!(parsed.desktop_mode, VerifyFastDesktopMode::Auto);
    }

    #[test]
    fn verify_option_parser_accepts_fast_desktop_flags() {
        let parsed =
            parse_verify_options(vec!["fast".into(), "--with-desktop".into()]).expect("parse");
        assert_eq!(parsed.mode, VerifyMode::Fast);
        assert_eq!(parsed.desktop_mode, VerifyFastDesktopMode::WithDesktop);
    }

    #[test]
    fn verify_option_parser_rejects_conflicting_desktop_flags() {
        let err = parse_verify_options(vec![
            "fast".into(),
            "--with-desktop".into(),
            "--without-desktop".into(),
        ])
        .unwrap_err();
        assert!(err.to_string().contains("cannot be combined"));
    }

    #[test]
    fn verify_option_parser_supports_profiles() {
        let parsed =
            parse_verify_options(vec!["--profile".into(), "dev".into()]).expect("parse options");
        let options =
            resolve_verify_options_from_profile(parsed, &test_profiles()).expect("resolve profile");
        assert_eq!(options.mode, VerifyMode::Fast);
        assert_eq!(options.profile.as_deref(), Some("dev"));
    }

    #[test]
    fn flow_option_parser_rejects_package_with_all_scope() {
        let err = parse_flow_options(vec!["--all".into(), "--package".into(), "xtask".into()])
            .unwrap_err();
        assert!(err.to_string().contains("cannot be combined"));
    }
}
